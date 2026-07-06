//! Godot 4 GDExtension binding for [`orbital_movement_gdextension`].
//!
//! # Role
//!
//! This crate is a thin adapter: physics, orbit math, guided transfers, and viability
//! assessment all live in the core library. Godot types (`Vector3`, `Dictionary`) are
//! converted at the boundary. Invalid inputs generally yield sentinel values (`-1`,
//! `false`, `Vector3.ZERO`) instead of panicking.
//!
//! # Layout
//!
//! - [`GdOrbitType`] / [`GdBodyState`]: internal enums exposed to GDScript as `i64` constants.
//! - [`OrbitalSimulation`]: [`RefCounted`] wrapper around [`Simulation`].
//! - `to_gd_vec3` / `from_gd_vec3`: f64 simulation units ↔ Godot `f32` vectors.
//! - `with_body` / `with_body_mut`: safe body-id dispatch; negative ids are rejected.

use godot::prelude::*;
use orbital_movement_gdextension::{
    BodyId, BodyState, CentralBody, HIGH_INCLINATION_RAD, LOW_EARTH_INCLINATION_RAD,
    MOLNIYA_APOGEE_ALTITUDE_R, MOLNIYA_PERIGEE_ALTITUDE_R, OrbitParams, OrbitType, Simulation,
    SimulationScale, StarConfig, SurfaceTessellationConfig, TransferAvailability,
    TransferBurnStatus, TransferViabilityConfig,
    Vec3, build_orbit_params_from_ui, orbit_ui_defaults, orbit_uses_computed_altitude,
    orbit_uses_elliptical_params, orbit_uses_inclination_param,
};

struct OrbitalMovementGdextensionExtension;

#[gdextension]
unsafe impl ExtensionLibrary for OrbitalMovementGdextensionExtension {}

// ---------------------------------------------------------------------------
// Godot-facing orbit and body state enums (mirrored as i64 constants on
// OrbitalSimulation for GDScript).
// ---------------------------------------------------------------------------

/// Godot-facing orbit type enum (use `OrbitalSimulation.ORBIT_*` constants from GDScript).
#[derive(GodotConvert, Var, Debug, Clone, Copy, PartialEq, Eq)]
#[godot(via = i64)]
pub enum GdOrbitType {
    /// Prograde circular equatorial orbit.
    CircularEquatorial = 0,
    /// Circular polar orbit.
    CircularPolar = 1,
    /// Geostationary equatorial orbit.
    Geostationary = 2,
    /// Low circular orbit.
    LowCircular = 3,
    /// Retrograde circular equatorial orbit.
    RetrogradeEquatorial = 4,
    /// Coplanar elliptical equatorial orbit.
    EllipticalEquatorial = 5,
    /// Prograde circular orbit in the ecliptic (planet–sun) plane.
    EclipticPrograde = 6,
    /// Retrograde circular orbit in the ecliptic plane.
    EclipticRetrograde = 7,
    /// Elliptical orbit with configurable inclination.
    EllipticalInclined = 8,
    /// Inclined circular orbit at geostationary altitude (Tundra).
    Tundra = 9,
    /// Highly elliptical inclined Molniya orbit.
    Molniya = 10,
    /// Supersynchronous graveyard orbit above GEO.
    Graveyard = 11,
}

impl GdOrbitType {
    /// Converts a Godot-facing orbit type constant to the internal enum.
    pub fn from_i64(value: i64) -> Option<Self> {
        match value {
            0 => Some(Self::CircularEquatorial),
            1 => Some(Self::CircularPolar),
            2 => Some(Self::Geostationary),
            3 => Some(Self::LowCircular),
            4 => Some(Self::RetrogradeEquatorial),
            5 => Some(Self::EllipticalEquatorial),
            6 => Some(Self::EclipticPrograde),
            7 => Some(Self::EclipticRetrograde),
            8 => Some(Self::EllipticalInclined),
            9 => Some(Self::Tundra),
            10 => Some(Self::Molniya),
            11 => Some(Self::Graveyard),
            _ => None,
        }
    }
}

impl From<GdOrbitType> for OrbitType {
    fn from(value: GdOrbitType) -> Self {
        match value {
            GdOrbitType::CircularEquatorial => OrbitType::CircularEquatorial,
            GdOrbitType::CircularPolar => OrbitType::CircularPolar,
            GdOrbitType::Geostationary => OrbitType::Geostationary,
            GdOrbitType::LowCircular => OrbitType::LowCircular,
            GdOrbitType::RetrogradeEquatorial => OrbitType::RetrogradeEquatorial,
            GdOrbitType::EllipticalEquatorial => OrbitType::EllipticalEquatorial,
            GdOrbitType::EclipticPrograde => OrbitType::EclipticPrograde,
            GdOrbitType::EclipticRetrograde => OrbitType::EclipticRetrograde,
            GdOrbitType::EllipticalInclined => OrbitType::EllipticalInclined,
            GdOrbitType::Tundra => OrbitType::Tundra,
            GdOrbitType::Molniya => OrbitType::Molniya,
            GdOrbitType::Graveyard => OrbitType::Graveyard,
        }
    }
}

/// Godot-facing body state enum (use `OrbitalSimulation.STATE_*` constants from GDScript).
#[derive(GodotConvert, Var, Debug, Clone, Copy, PartialEq, Eq)]
#[godot(via = i64)]
pub enum GdBodyState {
    /// Free flight under gravity.
    Flying = 0,
    /// Contact with the planet surface.
    SurfaceContact = 1,
}

impl From<BodyState> for GdBodyState {
    fn from(value: BodyState) -> Self {
        match value {
            BodyState::Flying => GdBodyState::Flying,
            BodyState::SurfaceContact => GdBodyState::SurfaceContact,
        }
    }
}

// ---------------------------------------------------------------------------
// Vector conversion helpers (simulation f64 ↔ Godot f32).
// ---------------------------------------------------------------------------

/// Converts a simulation [`Vec3`] to Godot space. Components are narrowed to `f32`.
fn to_gd_vec3(v: Vec3) -> godot::builtin::Vector3 {
    godot::builtin::Vector3::new(v.x as f32, v.y as f32, v.z as f32)
}

/// Converts a Godot vector to simulation [`Vec3`] (widened to `f64`).
fn from_gd_vec3(v: godot::builtin::Vector3) -> Vec3 {
    Vec3::new(v.x as f64, v.y as f64, v.z as f64)
}

// ---------------------------------------------------------------------------
// OrbitalSimulation — main Godot API.
// ---------------------------------------------------------------------------

/// Main orbital simulation API for Godot.
#[derive(GodotClass)]
#[class(base = RefCounted)]
struct OrbitalSimulation {
    base: Base<RefCounted>,
    simulation: Option<Simulation>,
}

#[godot_api]
impl IRefCounted for OrbitalSimulation {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            simulation: None,
        }
    }
}

#[godot_api]
impl OrbitalSimulation {
    // --- Orbit type constants (match GdOrbitType discriminant values) ---

    #[constant]
    const ORBIT_CIRCULAR_EQUATORIAL: i64 = 0;

    #[constant]
    const ORBIT_CIRCULAR_POLAR: i64 = 1;

    #[constant]
    const ORBIT_GEOSTATIONARY: i64 = 2;

    #[constant]
    const ORBIT_LOW_CIRCULAR: i64 = 3;

    #[constant]
    const ORBIT_RETROGRADE_EQUATORIAL: i64 = 4;

    #[constant]
    const ORBIT_ELLIPTICAL_EQUATORIAL: i64 = 5;

    #[constant]
    const ORBIT_ECLIPTIC_PROGRADE: i64 = 6;

    #[constant]
    const ORBIT_ECLIPTIC_RETROGRADE: i64 = 7;

    #[constant]
    const ORBIT_ELLIPTICAL_INCLINED: i64 = 8;

    #[constant]
    const ORBIT_TUNDRA: i64 = 9;

    #[constant]
    const ORBIT_MOLNIYA: i64 = 10;

    #[constant]
    const ORBIT_GRAVEYARD: i64 = 11;

    // --- Transfer burn status (active guided transfer on a body) ---

    #[constant]
    const TRANSFER_IDLE: i64 = 0;

    #[constant]
    const TRANSFER_BURNING: i64 = 1;

    #[constant]
    const TRANSFER_FINISHED: i64 = 2;

    /// Guided transfer is supported and reasonably fast (`assess_transfer_viability` → `availability`).
    #[constant]
    const TRANSFER_VIABILITY_AVAILABLE: i64 = 0;

    /// Transfer is physically possible but too slow for typical gameplay.
    #[constant]
    const TRANSFER_VIABILITY_IMPRACTICAL: i64 = 1;

    /// Transfer cannot be started (invalid target, escape trajectory, no thrust, etc.).
    #[constant]
    const TRANSFER_VIABILITY_UNAVAILABLE: i64 = 2;

    // --- Body state ---

    #[constant]
    const STATE_FLYING: i64 = 0;

    #[constant]
    const STATE_SURFACE_CONTACT: i64 = 1;

    // --- Local thrust direction bit flags (combinable with bitwise OR) ---

    #[constant]
    const THRUST_PROGRADE: i64 = 1;

    #[constant]
    const THRUST_RETROGRADE: i64 = 2;

    #[constant]
    const THRUST_LEFT: i64 = 4;

    #[constant]
    const THRUST_RIGHT: i64 = 8;

    #[constant]
    const THRUST_UP: i64 = 16;

    #[constant]
    const THRUST_DOWN: i64 = 32;

    // --- Simulation lifecycle ---

    /// Creates an Earth-like simulation with default units.
    #[func]
    fn create_earth_like(&mut self, rotation_period_s: f64) -> bool {
        match Simulation::earth_like(rotation_period_s) {
            Ok(sim) => {
                self.simulation = Some(sim);
                true
            }
            Err(_) => false,
        }
    }

    /// Creates an Earth-like simulation with axial tilt applied to the sun's ecliptic plane.
    #[func]
    fn create_earth_like_with_obliquity(
        &mut self,
        rotation_period_s: f64,
        obliquity_rad: f64,
    ) -> bool {
        match Simulation::earth_like_with_obliquity(rotation_period_s, obliquity_rad) {
            Ok(sim) => {
                self.simulation = Some(sim);
                true
            }
            Err(_) => false,
        }
    }

    /// Creates a simulation with custom central body parameters.
    #[func]
    fn create_custom(
        &mut self,
        mass_earth: f64,
        radius_earth: f64,
        rotation_period_s: f64,
        length_scale: f64,
        time_scale: f64,
    ) -> bool {
        let central = CentralBody {
            mass_earth,
            radius_earth,
            spin_axis: Vec3::Y,
            rotation_period_s,
        };
        let scale = SimulationScale {
            length_scale,
            time_scale,
        };
        match Simulation::new(central, scale) {
            Ok(sim) => {
                self.simulation = Some(sim);
                true
            }
            Err(_) => false,
        }
    }

    /// Configures the star / light source on the ecliptic plane.
    ///
    /// `obliquity_rad` tilts the ecliptic from the equatorial plane (XZ) about +X.
    /// `orbital_longitude_rad` is the sun's position on that plane (0 = +X equinox).
    #[func]
    fn set_star(
        &mut self,
        distance_earth_radii: f64,
        obliquity_rad: f64,
        orbital_longitude_rad: f64,
    ) {
        if let Some(sim) = self.simulation.as_mut() {
            sim.set_star(StarConfig::new(
                distance_earth_radii,
                obliquity_rad,
                orbital_longitude_rad,
            ));
        }
    }

    /// Advances the simulation by `delta` seconds.
    #[func]
    fn step(&mut self, delta: f64) -> bool {
        match self.simulation.as_mut() {
            Some(sim) => sim.step(delta).is_ok(),
            None => false,
        }
    }

    /// Creates a body with explicit position, velocity, and mass.
    #[func]
    fn create_body(&mut self, position: Vector3, velocity: Vector3, mass: f64) -> i64 {
        match self.simulation.as_mut() {
            Some(sim) => {
                match sim.create_body(from_gd_vec3(position), from_gd_vec3(velocity), mass) {
                    Ok(id) => id.0 as i64,
                    Err(_) => -1,
                }
            }
            None => -1,
        }
    }

    /// Creates a body in a circular orbit at the given altitude.
    #[func]
    fn create_body_circular(
        &mut self,
        orbit_type: i64,
        altitude_earth_radii: f64,
        mass: f64,
    ) -> i64 {
        let Some(orbit) = GdOrbitType::from_i64(orbit_type) else {
            return -1;
        };
        let params = OrbitParams::circular(altitude_earth_radii);
        self.create_body_in_orbit_internal(orbit, params, mass)
    }

    /// Creates a body in an elliptical equatorial orbit.
    #[func]
    fn create_body_elliptical(
        &mut self,
        perigee_altitude: f64,
        apogee_altitude: f64,
        mass: f64,
    ) -> i64 {
        let params = OrbitParams::elliptical(perigee_altitude, apogee_altitude);
        self.create_body_in_orbit_internal(GdOrbitType::EllipticalEquatorial, params, mass)
    }

    fn create_body_in_orbit_internal(
        &mut self,
        orbit_type: GdOrbitType,
        params: OrbitParams,
        mass: f64,
    ) -> i64 {
        match self.simulation.as_mut() {
            Some(sim) => match sim.create_body_in_orbit(orbit_type.into(), params, mass) {
                Ok(id) => id.0 as i64,
                Err(_) => -1,
            },
            None => -1,
        }
    }

    /// Returns the body position.
    #[func]
    fn get_position(&self, body_id: i64) -> Vector3 {
        self.with_body(body_id, |sim, id| {
            to_gd_vec3(sim.position(id).unwrap_or(Vec3::ZERO))
        })
        .unwrap_or(Vector3::ZERO)
    }

    /// Returns the body velocity.
    #[func]
    fn get_velocity(&self, body_id: i64) -> Vector3 {
        self.with_body(body_id, |sim, id| {
            to_gd_vec3(sim.velocity(id).unwrap_or(Vec3::ZERO))
        })
        .unwrap_or(Vector3::ZERO)
    }

    /// Returns the body state (`STATE_FLYING` or `STATE_SURFACE_CONTACT`).
    #[func]
    fn get_state(&self, body_id: i64) -> i64 {
        match self.with_body(body_id, |sim, id| sim.state(id)) {
            Some(Ok(state)) => GdBodyState::from(state) as i64,
            _ => GdBodyState::Flying as i64,
        }
    }

    /// Clears surface contact without changing velocity.
    #[func]
    fn clear_surface_contact(&mut self, body_id: i64) -> bool {
        self.with_body_mut(body_id, |sim, id| sim.clear_surface_contact(id))
            .is_some()
    }

    /// Applies a force in the given direction for the next step.
    #[func]
    fn apply_force(&mut self, body_id: i64, direction: Vector3, magnitude: f64) -> bool {
        self.with_body_mut(body_id, |sim, id| {
            sim.apply_force(id, from_gd_vec3(direction), magnitude)
        })
        .is_some()
    }

    /// Applies an instantaneous delta-v burn.
    #[func]
    fn apply_delta_v(&mut self, body_id: i64, delta_v: Vector3) -> bool {
        self.with_body_mut(body_id, |sim, id| {
            sim.apply_instantaneous_delta_v(id, from_gd_vec3(delta_v))
        })
        .is_some()
    }

    /// Returns the delta-v required to reach a circular orbit at the given altitude.
    #[func]
    fn get_delta_v_to_orbit(
        &self,
        body_id: i64,
        orbit_type: i64,
        altitude_earth_radii: f64,
    ) -> Vector3 {
        let Some(orbit) = GdOrbitType::from_i64(orbit_type) else {
            return Vector3::ZERO;
        };
        let params = OrbitParams::circular(altitude_earth_radii);
        self.with_body(body_id, |sim, id| {
            sim.required_delta_v_to_orbit(id, orbit.into(), params)
                .map(to_gd_vec3)
                .ok()
        })
        .flatten()
        .unwrap_or(Vector3::ZERO)
    }

    /// Returns the unit thrust direction toward a target circular orbit.
    #[func]
    fn get_thrust_direction_to_orbit(
        &self,
        body_id: i64,
        orbit_type: i64,
        altitude_earth_radii: f64,
    ) -> Vector3 {
        let Some(orbit) = GdOrbitType::from_i64(orbit_type) else {
            return Vector3::ZERO;
        };
        let params = OrbitParams::circular(altitude_earth_radii);
        self.with_body(body_id, |sim, id| {
            sim.thrust_direction_to_orbit(id, orbit.into(), params)
                .map(to_gd_vec3)
                .ok()
        })
        .flatten()
        .unwrap_or(Vector3::ZERO)
    }

    /// Returns visible planetary surface area from the body position.
    #[func]
    fn get_visible_surface_area(&self, body_id: i64) -> f64 {
        self.with_body(body_id, |sim, id| sim.visible_surface_area(id).ok())
            .flatten()
            .unwrap_or(0.0)
    }

    /// Returns horizon half-angle in radians from the body position.
    #[func]
    fn get_horizon_half_angle(&self, body_id: i64) -> f64 {
        self.with_body(body_id, |sim, id| sim.horizon_half_angle(id).ok())
            .flatten()
            .unwrap_or(0.0)
    }

    /// Projects the osculating orbit onto the planet surface in the planet-fixed frame.
    ///
    /// Returns a Dictionary with keys:
    /// `ground_track`, `visibility_port`, `visibility_starboard` (PackedVector3Array),
    /// `ground_line_vertices`, `corridor_vertices`, `corridor_normals`, `corridor_indices`,
    /// and `ephemeral` (bool, true when thrust or transfer burn is reshaping the path).
    #[func]
    fn get_orbital_surface_track(
        &self,
        body_id: i64,
        spin_angle_rad: f64,
        max_points: i64,
        display_radius: f64,
    ) -> godot::builtin::VarDictionary {
        use godot::builtin::{PackedVector3Array, Variant, VarDictionary};

        let mut result = VarDictionary::new();
        let max_points = max_points.clamp(8, 2048) as usize;
        let track = self
            .with_body(body_id, |sim, id| {
                sim.orbital_surface_track(id, spin_angle_rad, max_points)
            })
            .and_then(|r| r.ok());
        let Some(track) = track else {
            let empty = Variant::from(PackedVector3Array::new());
            result.set("ground_track", &empty);
            result.set("visibility_port", &empty);
            result.set("visibility_starboard", &empty);
            result.set("ground_line_vertices", &empty);
            result.set("corridor_vertices", &empty);
            result.set("corridor_normals", &empty);
            result.set("corridor_indices", &Variant::from(godot::builtin::PackedInt32Array::new()));
            result.set("ephemeral", false);
            return result;
        };

        let ground_track = Self::vec3_slice_to_packed(&track.ground_track);
        let visibility_port = Self::vec3_slice_to_packed(&track.visibility_port);
        let visibility_starboard = Self::vec3_slice_to_packed(&track.visibility_starboard);
        result.set("ground_track", &Variant::from(ground_track));
        result.set("visibility_port", &Variant::from(visibility_port));
        result.set("visibility_starboard", &Variant::from(visibility_starboard));
        result.set("ephemeral", track.ephemeral);

        let radius = if display_radius > 0.0 {
            display_radius
        } else {
            self.get_planet_radius()
        };
        let config = SurfaceTessellationConfig::default();
        let ground_line = track.tessellate_ground_line(radius, &config);
        let corridor = track.tessellate_corridor(radius, &config);
        result.set(
            "ground_line_vertices",
            &Variant::from(Self::vec3_slice_to_packed(&ground_line)),
        );
        Self::push_surface_mesh(&mut result, "corridor", &corridor);
        result
    }

    /// Returns a tessellated visibility-cap mesh for a body in the planet-fixed frame.
    #[func]
    fn get_visibility_cap_mesh(
        &self,
        body_id: i64,
        spin_angle_rad: f64,
        display_radius: f64,
    ) -> godot::builtin::VarDictionary {
        use godot::builtin::VarDictionary;
        let mut result = VarDictionary::new();
        let radius = if display_radius > 0.0 {
            display_radius
        } else {
            self.get_planet_radius()
        };
        let mesh = self
            .with_body(body_id, |sim, id| {
                sim.visibility_cap_mesh_for_body(id, spin_angle_rad, radius)
            })
            .and_then(|r| r.ok());
        let Some(mesh) = mesh else {
            Self::push_surface_mesh(&mut result, "cap", &orbital_movement_gdextension::SurfaceMesh::default());
            return result;
        };
        Self::push_surface_mesh(&mut result, "cap", &mesh);
        result
    }

    /// Returns planet surface radius in simulation units.
    #[func]
    fn get_planet_radius(&self) -> f64 {
        self.simulation
            .as_ref()
            .map(Simulation::planet_radius)
            .unwrap_or(1.0)
    }

    /// Returns geostationary altitude above the surface in Earth radii.
    #[func]
    fn get_geostationary_altitude(&self) -> f64 {
        self.simulation
            .as_ref()
            .and_then(|sim| sim.geostationary_altitude_earth_radii().ok())
            .unwrap_or(0.0)
    }

    /// Returns graveyard orbit altitude above the surface in Earth radii.
    #[func]
    fn get_graveyard_altitude(&self) -> f64 {
        self.simulation
            .as_ref()
            .and_then(|sim| sim.graveyard_altitude_earth_radii().ok())
            .unwrap_or(0.0)
    }

    /// Returns the normalized spin axis of the central body.
    #[func]
    fn get_spin_axis(&self) -> Vector3 {
        self.simulation
            .as_ref()
            .map(|sim| to_gd_vec3(sim.spin_axis()))
            .unwrap_or(Vector3::UP)
    }

    /// Sidereal rotation period of the central body in simulation seconds.
    #[func]
    fn get_rotation_period_s(&self) -> f64 {
        self.simulation
            .as_ref()
            .map(Simulation::rotation_period_s)
            .unwrap_or(86_400.0)
    }

    /// Planet spin rate in radians per simulation second.
    #[func]
    fn get_angular_rate_rad_s(&self) -> f64 {
        self.simulation
            .as_ref()
            .map(Simulation::angular_rate_rad_s)
            .unwrap_or(0.0)
    }

    /// Body position in the planet-fixed frame at `spin_angle_rad`.
    #[func]
    fn get_position_planet_fixed(&self, body_id: i64, spin_angle_rad: f64) -> Vector3 {
        self.with_body(body_id, |sim, id| {
            sim.body_position_planet_fixed(id, spin_angle_rad)
                .map(to_gd_vec3)
                .ok()
        })
        .flatten()
        .unwrap_or(Vector3::ZERO)
    }

    /// Returns apparent star position for the given planet spin angle.
    #[func]
    fn get_star_apparent_position(&self, spin_angle_rad: f64) -> Vector3 {
        self.simulation
            .as_ref()
            .map(|sim| to_gd_vec3(sim.star_apparent_position(spin_angle_rad)))
            .unwrap_or(Vector3::ZERO)
    }

    /// Returns fixed inertial star position.
    #[func]
    fn get_star_inertial_position(&self) -> Vector3 {
        self.simulation
            .as_ref()
            .map(|sim| to_gd_vec3(sim.star_inertial_position()))
            .unwrap_or(Vector3::ZERO)
    }

    /// Current simulation time in seconds.
    #[func]
    fn get_time(&self) -> f64 {
        self.simulation
            .as_ref()
            .map(Simulation::time_s)
            .unwrap_or(0.0)
    }

    /// Clears all bodies and resets simulation time.
    #[func]
    fn reset_simulation(&mut self) -> bool {
        if let Some(sim) = self.simulation.as_mut() {
            sim.reset();
            true
        } else {
            false
        }
    }

    /// Returns a unit thrust vector from local-frame direction bit flags.
    #[func]
    fn get_thrust_direction_from_flags(&self, body_id: i64, direction_flags: i64) -> Vector3 {
        self.with_body(body_id, |sim, id| {
            sim.thrust_direction_from_flags(id, direction_flags as u32)
                .map(to_gd_vec3)
                .ok()
        })
        .flatten()
        .unwrap_or(Vector3::ZERO)
    }

    /// Applies thrust using local-frame direction bit flags.
    #[func]
    fn apply_force_from_flags(
        &mut self,
        body_id: i64,
        magnitude: f64,
        direction_flags: i64,
    ) -> bool {
        self.with_body_mut(body_id, |sim, id| {
            sim.apply_force_from_flags(id, magnitude, direction_flags as u32)
        })
        .is_some()
    }

    /// Sets the maximum thrust force for a body (simulation force units).
    #[func]
    fn set_max_thrust(&mut self, body_id: i64, max_thrust: f64) -> bool {
        self.with_body_mut(body_id, |sim, id| sim.set_max_thrust(id, max_thrust))
            .is_some()
    }

    /// Returns the maximum thrust force configured for a body.
    #[func]
    fn get_max_thrust(&self, body_id: i64) -> f64 {
        self.with_body(body_id, |sim, id| sim.max_thrust(id).ok())
            .flatten()
            .unwrap_or(0.0)
    }

    /// Applies an instantaneous delta-v transfer burn to the target orbit.
    #[func]
    fn apply_transfer_to_orbit(
        &mut self,
        body_id: i64,
        orbit_type: i64,
        altitude_earth_radii: f64,
        perigee_altitude: f64,
        apogee_altitude: f64,
        inclination_rad: f64,
    ) -> bool {
        let Some(orbit) = GdOrbitType::from_i64(orbit_type) else {
            return false;
        };
        let params = Self::build_orbit_params(
            orbit,
            altitude_earth_radii,
            perigee_altitude,
            apogee_altitude,
            inclination_rad,
        );
        self.with_body_mut(body_id, |sim, id| {
            sim.apply_transfer_to_orbit(id, orbit.into(), params)
        })
        .is_some()
    }

    /// Starts a guided transfer toward the target orbit (limited by the body's `max_thrust`).
    #[func]
    fn begin_transfer_to_orbit(
        &mut self,
        body_id: i64,
        orbit_type: i64,
        altitude_earth_radii: f64,
        perigee_altitude: f64,
        apogee_altitude: f64,
        inclination_rad: f64,
    ) -> bool {
        let Some(orbit) = GdOrbitType::from_i64(orbit_type) else {
            return false;
        };
        let params = Self::build_orbit_params(
            orbit,
            altitude_earth_radii,
            perigee_altitude,
            apogee_altitude,
            inclination_rad,
        );
        self.with_body_mut(body_id, |sim, id| {
            sim.begin_transfer_to_orbit(id, orbit.into(), params)
        })
        .is_some()
    }

    /// Starts a guided transfer (alias for [`Self::begin_transfer_to_orbit`]).
    #[func]
    fn begin_transfer_burn(
        &mut self,
        body_id: i64,
        orbit_type: i64,
        altitude_earth_radii: f64,
        perigee_altitude: f64,
        apogee_altitude: f64,
        inclination_rad: f64,
    ) -> bool {
        self.begin_transfer_to_orbit(
            body_id,
            orbit_type,
            altitude_earth_radii,
            perigee_altitude,
            apogee_altitude,
            inclination_rad,
        )
    }

    /// Returns transfer burn status (`TRANSFER_IDLE`, `TRANSFER_BURNING`, or `TRANSFER_FINISHED`).
    #[func]
    fn get_transfer_burn_status(&self, body_id: i64) -> i64 {
        self.with_body(body_id, |sim, id| match sim.transfer_burn_status(id) {
            TransferBurnStatus::Idle => 0,
            TransferBurnStatus::Burning => 1,
            TransferBurnStatus::Finished => 2,
        })
        .unwrap_or(0)
    }

    /// Returns remaining corrective delta-v magnitude for the active transfer.
    #[func]
    fn get_transfer_burn_remaining(&self, body_id: i64) -> f64 {
        self.with_body(body_id, |sim, id| sim.transfer_burn_remaining(id))
            .unwrap_or(0.0)
    }

    /// Returns transfer burn progress from 0.0 to 1.0.
    #[func]
    fn get_transfer_burn_progress(&self, body_id: i64) -> f64 {
        self.with_body(body_id, |sim, id| sim.transfer_burn_progress(id))
            .unwrap_or(0.0)
    }

    /// Clears transfer burn state after acknowledging completion.
    #[func]
    fn clear_transfer_burn(&mut self, body_id: i64) -> bool {
        if let Some(sim) = self.simulation.as_mut()
            && body_id >= 0
        {
            sim.clear_transfer_burn(BodyId(body_id as u32));
            return true;
        }
        false
    }

    /// Assesses whether a guided transfer is available, impractical, or unavailable.
    ///
    /// Returns a Dictionary with keys:
    /// `availability` (TRANSFER_* constant), `initial_delta_v`, `theoretical_min_burn_time_s`,
    /// `estimated_guided_burn_time_s`, `estimated_burn_steps`, `plane_change_deg`,
    /// `lowering_altitude`, `reason` (empty when available).
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn assess_transfer_viability(
        &self,
        body_id: i64,
        orbit_type: i64,
        altitude_earth_radii: f64,
        perigee_altitude: f64,
        apogee_altitude: f64,
        inclination_rad: f64,
        max_practical_burn_time_s: f64,
        transfer_max_thrust: f64,
    ) -> godot::builtin::VarDictionary {
        use godot::builtin::VarDictionary;
        let mut result = VarDictionary::new();
        let Some(orbit) = GdOrbitType::from_i64(orbit_type) else {
            result.set("availability", Self::TRANSFER_VIABILITY_UNAVAILABLE);
            result.set("reason", "invalid orbit type");
            return result;
        };
        let params = Self::build_orbit_params(
            orbit,
            altitude_earth_radii,
            perigee_altitude,
            apogee_altitude,
            inclination_rad,
        );
        let config = TransferViabilityConfig {
            max_practical_burn_time_s: if max_practical_burn_time_s > 0.0 {
                max_practical_burn_time_s
            } else {
                TransferViabilityConfig::default().max_practical_burn_time_s
            },
            ..TransferViabilityConfig::default()
        };
        let thrust_override = if transfer_max_thrust > 0.0 {
            Some(transfer_max_thrust)
        } else {
            None
        };
        let report = self
            .with_body(body_id, |sim, id| {
                sim.assess_transfer_viability_with_thrust(
                    id,
                    orbit.into(),
                    params,
                    &config,
                    thrust_override,
                )
            })
            .and_then(|r| r.ok());
        let Some(report) = report else {
            result.set("availability", Self::TRANSFER_VIABILITY_UNAVAILABLE);
            result.set("reason", "body not found or assessment failed");
            return result;
        };
        let availability = match report.availability {
            TransferAvailability::Available => Self::TRANSFER_VIABILITY_AVAILABLE,
            TransferAvailability::Impractical => Self::TRANSFER_VIABILITY_IMPRACTICAL,
            TransferAvailability::Unavailable => Self::TRANSFER_VIABILITY_UNAVAILABLE,
        };
        result.set("availability", availability);
        result.set("initial_delta_v", report.initial_delta_v);
        result.set(
            "theoretical_min_burn_time_s",
            report.theoretical_min_burn_time_s,
        );
        result.set(
            "estimated_guided_burn_time_s",
            report.estimated_guided_burn_time_s,
        );
        result.set("estimated_burn_steps", report.estimated_burn_steps as i64);
        result.set("plane_change_deg", report.plane_change_rad.to_degrees());
        result.set("lowering_altitude", report.lowering_altitude);
        result.set("reason", report.reason.unwrap_or_default());
        result
    }

    /// Default low-Earth inclination in radians (~51.6°, ISS-class).
    #[func]
    fn default_low_earth_inclination_rad() -> f64 {
        LOW_EARTH_INCLINATION_RAD
    }

    /// Default high-inclination value in radians (~63.4°, Molniya/Tundra class).
    #[func]
    fn default_high_inclination_rad() -> f64 {
        HIGH_INCLINATION_RAD
    }

    /// Default Molniya perigee altitude above the surface (Earth radii).
    #[func]
    fn molniya_perigee_altitude() -> f64 {
        MOLNIYA_PERIGEE_ALTITUDE_R
    }

    /// Default Molniya apogee altitude above the surface (Earth radii).
    #[func]
    fn molniya_apogee_altitude() -> f64 {
        MOLNIYA_APOGEE_ALTITUDE_R
    }

    // --- Orbit metadata (static; does not require a body) ---

    /// Whether the orbit type uses perigee/apogee UI fields (elliptical and Molniya).
    #[func]
    fn orbit_uses_elliptical_params(orbit_type: i64) -> bool {
        GdOrbitType::from_i64(orbit_type)
            .is_some_and(|orbit| orbit_uses_elliptical_params(orbit.into()))
    }

    /// Whether the orbit type uses an inclination UI field.
    #[func]
    fn orbit_uses_inclination_param(orbit_type: i64) -> bool {
        GdOrbitType::from_i64(orbit_type)
            .is_some_and(|orbit| orbit_uses_inclination_param(orbit.into()))
    }

    /// Whether altitude is computed from planet physics (GEO, graveyard, tundra).
    #[func]
    fn orbit_uses_computed_altitude(orbit_type: i64) -> bool {
        GdOrbitType::from_i64(orbit_type)
            .is_some_and(|orbit| orbit_uses_computed_altitude(orbit.into()))
    }

    /// Default UI field values when the user selects an orbit type.
    ///
    /// Dictionary keys: `altitude_earth_radii`, `perigee_altitude_earth_radii`,
    /// `apogee_altitude_earth_radii`, `inclination_rad`, `inclination_deg`.
    #[func]
    fn get_orbit_ui_defaults(orbit_type: i64) -> godot::builtin::VarDictionary {
        use godot::builtin::VarDictionary;
        let mut result = VarDictionary::new();
        let Some(orbit) = GdOrbitType::from_i64(orbit_type) else {
            return result;
        };
        let defaults = orbit_ui_defaults(orbit.into());
        result.set("altitude_earth_radii", defaults.altitude_earth_radii);
        result.set(
            "perigee_altitude_earth_radii",
            defaults.perigee_altitude_earth_radii,
        );
        result.set(
            "apogee_altitude_earth_radii",
            defaults.apogee_altitude_earth_radii,
        );
        result.set("inclination_rad", defaults.inclination_rad);
        result.set("inclination_deg", defaults.inclination_rad.to_degrees());
        result
    }

    /// Maps UI fields to [`OrbitParams`] via the core library.
    fn build_orbit_params(
        orbit: GdOrbitType,
        altitude_earth_radii: f64,
        perigee_altitude: f64,
        apogee_altitude: f64,
        inclination_rad: f64,
    ) -> OrbitParams {
        build_orbit_params_from_ui(
            orbit.into(),
            altitude_earth_radii,
            perigee_altitude,
            apogee_altitude,
            inclination_rad,
        )
    }

    /// Creates a body in the selected orbit and returns its id.
    #[func]
    fn spawn_body_in_orbit(
        &mut self,
        orbit_type: i64,
        altitude_earth_radii: f64,
        perigee_altitude: f64,
        apogee_altitude: f64,
        inclination_rad: f64,
        mass: f64,
    ) -> i64 {
        let Some(orbit) = GdOrbitType::from_i64(orbit_type) else {
            return -1;
        };
        let params = Self::build_orbit_params(
            orbit,
            altitude_earth_radii,
            perigee_altitude,
            apogee_altitude,
            inclination_rad,
        );
        self.create_body_in_orbit_internal(orbit, params, mass)
    }

    /// Converts simulation points to a Godot packed array.
    fn vec3_slice_to_packed(points: &[Vec3]) -> godot::builtin::PackedVector3Array {
        use godot::builtin::PackedVector3Array;
        let mut packed = PackedVector3Array::new();
        for point in points {
            packed.push(to_gd_vec3(*point));
        }
        packed
    }

    /// Adds `vertices`, `normals`, and `indices` arrays for a surface mesh to a dictionary.
    fn push_surface_mesh(
        result: &mut godot::builtin::VarDictionary,
        prefix: &str,
        mesh: &orbital_movement_gdextension::SurfaceMesh,
    ) {
        use godot::builtin::{PackedInt32Array, Variant};
        let (vertices_key, normals_key, indices_key) = match prefix {
            "corridor" => ("corridor_vertices", "corridor_normals", "corridor_indices"),
            "cap" => ("cap_vertices", "cap_normals", "cap_indices"),
            _ => ("mesh_vertices", "mesh_normals", "mesh_indices"),
        };
        result.set(
            vertices_key,
            &Variant::from(Self::vec3_slice_to_packed(&mesh.vertices)),
        );
        result.set(
            normals_key,
            &Variant::from(Self::vec3_slice_to_packed(&mesh.normals)),
        );
        let mut indices = PackedInt32Array::new();
        for index in &mesh.indices {
            indices.push(*index as i32);
        }
        result.set(indices_key, &Variant::from(indices));
    }

    /// Runs `f` on a valid body id; returns `None` when the simulation is missing or id < 0.
    fn with_body<T>(&self, body_id: i64, f: impl FnOnce(&Simulation, BodyId) -> T) -> Option<T> {
        let sim = self.simulation.as_ref()?;
        if body_id < 0 {
            return None;
        }
        Some(f(sim, BodyId(body_id as u32)))
    }

    /// Like [`Self::with_body`], but unwraps `Result` from `f` into `Option`.
    fn with_body_mut<T>(
        &mut self,
        body_id: i64,
        f: impl FnOnce(&mut Simulation, BodyId) -> Result<T, orbital_movement_gdextension::ProjectError>,
    ) -> Option<T> {
        let sim = self.simulation.as_mut()?;
        if body_id < 0 {
            return None;
        }
        f(sim, BodyId(body_id as u32)).ok()
    }
}
