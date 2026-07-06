//! Simulation world owning the central body, small bodies, and star.

use crate::central_body::CentralBody;
use crate::collision::resolve_surface_contact;
use crate::error::ProjectError;
use crate::integrator::{MotionState, velocity_verlet_step};
use crate::math::Vec3;
use crate::orbits::{
    OrbitParams, OrbitType, initial_state_for_orbit, required_delta_v_to_orbit,
    required_delta_v_to_orbit_with_mission, state_on_orbit_at_position,
    target_orbit_reference_radius, thrust_direction_to_orbit,
};
use crate::small_body::{BodyId, BodyState, SmallBody};
use crate::star::StarConfig;
use crate::surface_geometry::SurfaceMesh;
use crate::transfer_burn::{TRANSFER_SNAP_EPSILON, TransferBurnStatus, TransferBurnTracker};
use crate::transfer_viability::{
    TransferViabilityConfig, TransferViabilityReport, assess_transfer_viability,
};
use crate::units::SimulationScale;
use crate::visibility::visible_surface_area;

/// Orbital mechanics simulation with a single central gravitating body.
#[derive(Debug, Clone)]
pub struct Simulation {
    central: CentralBody,
    scale: SimulationScale,
    star: StarConfig,
    bodies: Vec<SmallBody>,
    next_id: u32,
    time_s: f64,
    transfer_burns: TransferBurnTracker,
}

impl Simulation {
    /// Creates a new simulation.
    pub fn new(central: CentralBody, scale: SimulationScale) -> Result<Self, ProjectError> {
        central.validate()?;
        Ok(Self {
            central,
            scale,
            star: StarConfig::default(),
            bodies: Vec::new(),
            next_id: 1,
            time_s: 0.0,
            transfer_burns: TransferBurnTracker::default(),
        })
    }

    /// Creates an Earth-like simulation with default scaling.
    pub fn earth_like(rotation_period_s: f64) -> Result<Self, ProjectError> {
        Self::new(
            CentralBody::earth_like(rotation_period_s)?,
            SimulationScale::earth_radii(),
        )
    }

    /// Creates an Earth-like simulation with axial tilt applied to the sun's ecliptic plane.
    ///
    /// The planet spin axis remains +Y. Obliquity tilts the sun's orbital plane, not the planet.
    pub fn earth_like_with_obliquity(
        rotation_period_s: f64,
        obliquity_rad: f64,
    ) -> Result<Self, ProjectError> {
        let mut sim = Self::earth_like(rotation_period_s)?;
        sim.set_star(StarConfig::new(
            100.0,
            obliquity_rad,
            std::f64::consts::FRAC_PI_2,
        ));
        Ok(sim)
    }

    /// Current simulation time in seconds.
    pub fn time_s(&self) -> f64 {
        self.time_s
    }

    /// Central body configuration.
    pub fn central(&self) -> &CentralBody {
        &self.central
    }

    /// Simulation unit scaling.
    pub fn scale(&self) -> SimulationScale {
        self.scale
    }

    /// Star / light source configuration.
    pub fn star(&self) -> &StarConfig {
        &self.star
    }

    /// Mutable star configuration.
    pub fn star_mut(&mut self) -> &mut StarConfig {
        &mut self.star
    }

    /// Sets star configuration.
    pub fn set_star(&mut self, star: StarConfig) {
        self.star = star;
    }

    /// Gravitational parameter μ.
    pub fn mu(&self) -> f64 {
        self.central.mu(self.scale)
    }

    /// Creates a small body with explicit position and velocity.
    pub fn create_body(
        &mut self,
        position: Vec3,
        velocity: Vec3,
        mass: f64,
    ) -> Result<BodyId, ProjectError> {
        if mass <= 0.0 {
            return Err(ProjectError::InvalidMass);
        }
        let id = BodyId(self.next_id);
        self.next_id += 1;
        self.bodies
            .push(SmallBody::new(id, position, velocity, mass));
        Ok(id)
    }

    /// Creates a small body in a known orbit.
    pub fn create_body_in_orbit(
        &mut self,
        orbit_type: OrbitType,
        mut params: OrbitParams,
        mass: f64,
    ) -> Result<BodyId, ProjectError> {
        if matches!(
            orbit_type,
            OrbitType::EclipticPrograde | OrbitType::EclipticRetrograde
        ) {
            params.obliquity_rad = self.star.obliquity_rad;
        }
        let state = initial_state_for_orbit(&self.central, self.scale, orbit_type, params)?;
        self.create_body(state.position, state.velocity, mass)
    }

    /// Advances the simulation by `dt` seconds.
    pub fn step(&mut self, dt: f64) -> Result<(), ProjectError> {
        if dt <= 0.0 {
            return Err(ProjectError::InvalidTimeStep("time step must be positive"));
        }

        let mu = self.mu();
        let surface = self.central.surface_radius();

        for body in &mut self.bodies {
            if body.state() != BodyState::Flying {
                body.clear_thrust();
                continue;
            }

            let mut motion = MotionState {
                position: body.position(),
                velocity: body.velocity(),
            };
            velocity_verlet_step(&mut motion, body.thrust(), body.mass(), mu, dt);
            body.set_position(motion.position);
            body.set_velocity(motion.velocity);
            body.clear_thrust();
            resolve_surface_contact(body, surface);
        }

        let transfer_ids = self.transfer_burns.body_ids();
        for id in transfer_ids {
            if self.transfer_burns.status(id) == TransferBurnStatus::Burning {
                self.apply_guided_transfer_step(id, dt)?;
            }
        }

        self.time_s += dt;
        Ok(())
    }

    /// Returns position for a body.
    pub fn position(&self, id: BodyId) -> Result<Vec3, ProjectError> {
        Ok(self.body(id)?.position())
    }

    /// Returns velocity for a body.
    pub fn velocity(&self, id: BodyId) -> Result<Vec3, ProjectError> {
        Ok(self.body(id)?.velocity())
    }

    /// Returns state for a body.
    pub fn state(&self, id: BodyId) -> Result<BodyState, ProjectError> {
        Ok(self.body(id)?.state())
    }

    /// Clears surface contact so the body can remain on the surface without integrating.
    pub fn clear_surface_contact(&mut self, id: BodyId) -> Result<(), ProjectError> {
        self.body_mut(id)?.set_state(BodyState::Flying);
        Ok(())
    }

    /// Applies a force vector for the next integration step.
    pub fn apply_force(
        &mut self,
        id: BodyId,
        direction: Vec3,
        magnitude: f64,
    ) -> Result<(), ProjectError> {
        if direction.length_squared() <= f64::EPSILON {
            return Err(ProjectError::InvalidVector(
                "force direction must be non-zero",
            ));
        }
        self.transfer_burns.clear(id);
        let body = self.body_mut(id)?;
        if body.mass() <= 0.0 {
            return Err(ProjectError::InvalidMass);
        }
        let magnitude = Self::clamp_force_magnitude(body.max_thrust(), magnitude);
        body.add_force(direction.normalized() * magnitude);
        Ok(())
    }

    /// Sets the maximum thrust force for a body.
    pub fn set_max_thrust(&mut self, id: BodyId, max_thrust: f64) -> Result<(), ProjectError> {
        if max_thrust < 0.0 {
            return Err(ProjectError::InvalidVector(
                "max_thrust must be non-negative",
            ));
        }
        self.body_mut(id)?.set_max_thrust(max_thrust);
        Ok(())
    }

    /// Returns the maximum thrust force configured for a body.
    pub fn max_thrust(&self, id: BodyId) -> Result<f64, ProjectError> {
        Ok(self.body(id)?.max_thrust())
    }

    /// Applies an instantaneous delta-v burn.
    pub fn apply_instantaneous_delta_v(
        &mut self,
        id: BodyId,
        delta_v: Vec3,
    ) -> Result<(), ProjectError> {
        self.transfer_burns.clear(id);
        self.body_mut(id)?.apply_delta_v(delta_v);
        Ok(())
    }

    /// Required delta-v to reach the target orbit from the body's current state.
    pub fn required_delta_v_to_orbit(
        &self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
    ) -> Result<Vec3, ProjectError> {
        let body = self.body(id)?;
        required_delta_v_to_orbit(
            &self.central,
            self.scale,
            body.position(),
            body.velocity(),
            orbit_type,
            params,
        )
    }

    /// Unit thrust direction toward the target orbit.
    pub fn thrust_direction_to_orbit(
        &self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
    ) -> Result<Vec3, ProjectError> {
        let body = self.body(id)?;
        thrust_direction_to_orbit(
            &self.central,
            self.scale,
            body.position(),
            body.velocity(),
            orbit_type,
            params,
        )
    }

    /// Visible spherical cap area on the planet from the body's position.
    pub fn visible_surface_area(&self, id: BodyId) -> Result<f64, ProjectError> {
        let body = self.body(id)?;
        Ok(visible_surface_area(
            body.position().length(),
            self.central.surface_radius(),
        ))
    }

    /// Horizon half-angle (radians) from the body's position.
    pub fn horizon_half_angle(&self, id: BodyId) -> Result<f64, ProjectError> {
        let body = self.body(id)?;
        Ok(crate::visibility::horizon_half_angle(
            body.position().length(),
            self.central.surface_radius(),
        ))
    }

    /// Ground track and visibility corridor on the planet surface for the body's osculating orbit.
    ///
    /// Samples one orbital period (or a limited arc for escape trajectories) using the body's
    /// instantaneous position and velocity. When thrust or a guided transfer is active, the
    /// result is marked [`OrbitalSurfaceTrack::ephemeral`] because the path will change.
    pub fn orbital_surface_track(
        &self,
        id: BodyId,
        spin_angle_rad: f64,
        max_points: usize,
    ) -> Result<crate::ground_track::OrbitalSurfaceTrack, ProjectError> {
        let body = self.body(id)?;
        let ephemeral = crate::ground_track::is_ephemeral_trajectory(
            body.thrust(),
            self.transfer_burns.status(id),
        );
        crate::ground_track::orbital_surface_track(
            self.mu(),
            self.central.surface_radius(),
            self.spin_axis(),
            self.angular_rate_rad_s(),
            spin_angle_rad,
            body.position(),
            body.velocity(),
            ephemeral,
            max_points,
        )
    }

    /// Planet surface radius in simulation units.
    pub fn planet_radius(&self) -> f64 {
        self.central.surface_radius()
    }

    /// Geostationary altitude above the surface in Earth radii for this simulation.
    pub fn geostationary_altitude_earth_radii(&self) -> Result<f64, ProjectError> {
        self.central.geostationary_altitude_earth_radii(self.scale)
    }

    /// Normalized spin axis of the central body.
    pub fn spin_axis(&self) -> Vec3 {
        self.central.spin_axis_normalized()
    }

    /// Sidereal rotation period of the central body in simulation seconds.
    pub fn rotation_period_s(&self) -> f64 {
        self.central.rotation_period_s
    }

    /// Planet spin rate in radians per simulation second.
    pub fn angular_rate_rad_s(&self) -> f64 {
        self.central.angular_rate_rad_s()
    }

    /// Transforms an inertial position into the planet-fixed frame at `spin_angle_rad`.
    ///
    /// Applies a rotation of `-spin_angle_rad` about the spin axis (inverse of planet rotation).
    pub fn position_in_planet_fixed_frame(&self, position: Vec3, spin_angle_rad: f64) -> Vec3 {
        position.rotate_about_axis(self.spin_axis(), -spin_angle_rad)
    }

    /// Body position in the planet-fixed frame at `spin_angle_rad`.
    pub fn body_position_planet_fixed(
        &self,
        id: BodyId,
        spin_angle_rad: f64,
    ) -> Result<Vec3, ProjectError> {
        let position = self.position(id)?;
        Ok(self.position_in_planet_fixed_frame(position, spin_angle_rad))
    }

    /// Apparent star position for a given planet spin angle (co-rotating frame).
    pub fn star_apparent_position(&self, spin_angle_rad: f64) -> Vec3 {
        self.star
            .apparent_position(self.central.spin_axis_normalized(), spin_angle_rad)
    }

    /// Fixed inertial star position.
    pub fn star_inertial_position(&self) -> Vec3 {
        self.star
            .inertial_position(self.central.spin_axis_normalized())
    }

    /// Graveyard orbit altitude above the surface in Earth radii.
    pub fn graveyard_altitude_earth_radii(&self) -> Result<f64, ProjectError> {
        crate::orbits::graveyard_altitude_earth_radii(
            self.mu(),
            self.central.angular_rate_rad_s(),
            self.central.surface_radius(),
        )
    }

    /// Clears all bodies and resets simulation time to zero.
    pub fn reset(&mut self) {
        self.bodies.clear();
        self.next_id = 1;
        self.time_s = 0.0;
        self.transfer_burns.clear_all();
    }

    /// Unit thrust vector from local-frame direction bit flags.
    pub fn thrust_direction_from_flags(
        &self,
        id: BodyId,
        flags: u32,
    ) -> Result<Vec3, ProjectError> {
        let body = self.body(id)?;
        crate::thrust_frame::thrust_direction_from_flags(body.position(), body.velocity(), flags)
    }

    /// Applies thrust using local-frame direction bit flags.
    pub fn apply_force_from_flags(
        &mut self,
        id: BodyId,
        magnitude: f64,
        flags: u32,
    ) -> Result<(), ProjectError> {
        let direction = self.thrust_direction_from_flags(id, flags)?;
        self.apply_force(id, direction, magnitude)
    }

    /// Applies an instantaneous transfer burn to the target orbit.
    pub fn apply_transfer_to_orbit(
        &mut self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
    ) -> Result<(), ProjectError> {
        let delta_v = self.required_delta_v_to_orbit(id, orbit_type, params)?;
        self.apply_instantaneous_delta_v(id, delta_v)
    }

    /// Starts a guided transfer toward the target orbit, limited by the body's `max_thrust`.
    pub fn begin_transfer_to_orbit(
        &mut self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
    ) -> Result<(), ProjectError> {
        let max_thrust = self.body(id)?.max_thrust();
        if max_thrust <= 0.0 {
            return Err(ProjectError::InvalidVector(
                "max_thrust must be positive to begin a transfer",
            ));
        }

        let params = self.prepare_orbit_params(orbit_type, params);
        let source_radius = self.body(id)?.position().length();
        let target_radius =
            target_orbit_reference_radius(&self.central, self.scale, orbit_type, params)?;
        let lowering_altitude = target_radius + 1e-9 < source_radius;
        let remaining = self.required_delta_v_to_orbit(id, orbit_type, params)?;
        let remaining_mag = remaining.length();

        if remaining_mag <= TRANSFER_SNAP_EPSILON
            && self.transfer_orbit_within_tolerance(id, orbit_type, params)?
        {
            self.snap_body_to_orbit(id, orbit_type, params)?;
            self.transfer_burns
                .start(id, orbit_type, params, remaining_mag, lowering_altitude);
            self.transfer_burns.finish(id);
            return Ok(());
        }

        self.transfer_burns
            .start(id, orbit_type, params, remaining_mag, lowering_altitude);
        Ok(())
    }

    /// Starts a guided transfer (alias for [`Self::begin_transfer_to_orbit`]).
    pub fn begin_transfer_burn(
        &mut self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
    ) -> Result<(), ProjectError> {
        self.begin_transfer_to_orbit(id, orbit_type, params)
    }

    /// Returns transfer burn status for a body.
    pub fn transfer_burn_status(&self, id: BodyId) -> TransferBurnStatus {
        self.transfer_burns.status(id)
    }

    /// Remaining delta-v for an active or recently finished transfer burn.
    pub fn transfer_burn_remaining(&self, id: BodyId) -> f64 {
        self.transfer_burns.remaining(id)
    }

    /// Transfer burn progress in `[0, 1]`.
    pub fn transfer_burn_progress(&self, id: BodyId) -> f64 {
        self.transfer_burns.progress(id)
    }

    /// Clears transfer burn state (e.g. after acknowledging completion).
    pub fn clear_transfer_burn(&mut self, id: BodyId) {
        self.transfer_burns.clear(id);
    }

    /// Assesses whether a guided transfer to the target orbit is available, impractical, or unavailable.
    pub fn assess_transfer_viability(
        &self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
        config: &TransferViabilityConfig,
    ) -> Result<TransferViabilityReport, ProjectError> {
        self.assess_transfer_viability_with_thrust(id, orbit_type, params, config, None)
    }

    /// Like [`Self::assess_transfer_viability`], but uses `transfer_max_thrust` when provided.
    pub fn assess_transfer_viability_with_thrust(
        &self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
        config: &TransferViabilityConfig,
        transfer_max_thrust: Option<f64>,
    ) -> Result<TransferViabilityReport, ProjectError> {
        let body = self.body(id)?;
        let params = self.prepare_orbit_params(orbit_type, params);
        let max_thrust = transfer_max_thrust
            .filter(|value| *value > 0.0)
            .unwrap_or(body.max_thrust());
        assess_transfer_viability(
            &self.central,
            self.scale,
            body.position(),
            body.velocity(),
            max_thrust,
            body.mass(),
            orbit_type,
            params,
            config,
        )
    }

    /// Tessellated visibility cap mesh for a body in the planet-fixed frame.
    pub fn visibility_cap_mesh_for_body(
        &self,
        id: BodyId,
        spin_angle_rad: f64,
        display_radius: f64,
    ) -> Result<SurfaceMesh, ProjectError> {
        let body = self.body(id)?;
        let observer = self.position_in_planet_fixed_frame(body.position(), spin_angle_rad);
        let rho = crate::visibility::horizon_half_angle(
            body.position().length(),
            self.central.surface_radius(),
        );
        Ok(crate::visibility::visibility_cap_mesh(
            observer,
            display_radius,
            rho,
            self.spin_axis(),
            32,
            64,
        ))
    }

    /// Returns whether the body's current state matches the target orbit within guided-transfer tolerances.
    pub fn orbit_matches_target(
        &self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
    ) -> Result<bool, ProjectError> {
        let params = self.prepare_orbit_params(orbit_type, params);
        self.transfer_orbit_within_tolerance(id, orbit_type, params)
    }

    fn body(&self, id: BodyId) -> Result<&SmallBody, ProjectError> {
        self.bodies
            .iter()
            .find(|body| body.id() == id)
            .ok_or(ProjectError::BodyNotFound(id.0))
    }

    fn body_mut(&mut self, id: BodyId) -> Result<&mut SmallBody, ProjectError> {
        self.bodies
            .iter_mut()
            .find(|body| body.id() == id)
            .ok_or(ProjectError::BodyNotFound(id.0))
    }

    fn prepare_orbit_params(&self, orbit_type: OrbitType, mut params: OrbitParams) -> OrbitParams {
        if matches!(
            orbit_type,
            OrbitType::EclipticPrograde | OrbitType::EclipticRetrograde
        ) {
            params.obliquity_rad = self.star.obliquity_rad;
        }
        params
    }

    fn snap_body_to_orbit(
        &mut self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
    ) -> Result<(), ProjectError> {
        use crate::orbits::{orbit_plane_normal, project_position_onto_orbit_plane};

        let position = self.body(id)?.position();
        let plane_normal = orbit_plane_normal(orbit_type, params);
        let on_plane = project_position_onto_orbit_plane(position, plane_normal);
        let state =
            state_on_orbit_at_position(&self.central, self.scale, orbit_type, params, on_plane)?;
        let body = self.body_mut(id)?;
        let position_error = (state.position - position).length();
        body.set_velocity(state.velocity);
        if position_error <= 0.02 {
            body.set_position(state.position);
        }
        body.set_state(BodyState::Flying);
        Ok(())
    }

    fn apply_guided_transfer_step(&mut self, id: BodyId, dt: f64) -> Result<(), ProjectError> {
        let Some((orbit_type, params)) = self.transfer_burns.target(id) else {
            return Ok(());
        };

        let idx = self
            .bodies
            .iter()
            .position(|body| body.id() == id)
            .ok_or(ProjectError::BodyNotFound(id.0))?;

        let max_thrust = self.bodies[idx].max_thrust();
        let mass = self.bodies[idx].mass();
        if max_thrust <= 0.0 {
            return Err(ProjectError::InvalidVector(
                "max_thrust must be positive during transfer",
            ));
        }

        let position = self.bodies[idx].position();
        let velocity = self.bodies[idx].velocity();
        let surface = self.central.surface_radius();
        if position.length() <= surface + 1e-6 {
            return Ok(());
        }
        let lowering_mission = self.transfer_burns.lowering_altitude(id);
        let remaining_dv = required_delta_v_to_orbit_with_mission(
            &self.central,
            self.scale,
            position,
            velocity,
            orbit_type,
            params,
            lowering_mission,
        )?;
        let remaining_mag = remaining_dv.length();
        self.transfer_burns.set_last_remaining(id, remaining_mag);

        let orbit_matched = self.transfer_orbit_within_tolerance(id, orbit_type, params)?;

        if remaining_mag <= TRANSFER_SNAP_EPSILON && orbit_matched {
            self.snap_body_to_orbit(id, orbit_type, params)?;
            self.transfer_burns.finish(id);
            return Ok(());
        }

        let max_dv = (max_thrust / mass) * dt;
        let speed = velocity.length();
        let mut burn_cap = if speed > 1e-6 && speed < max_dv * 3.0 {
            max_dv.min(speed * 0.35)
        } else {
            max_dv
        };
        if lowering_mission {
            burn_cap = burn_cap.min(speed * 0.08);
        }
        let step_mag = remaining_mag.min(burn_cap);
        let step_dv = remaining_dv * (step_mag / remaining_mag);
        self.bodies[idx].apply_delta_v(step_dv);
        Ok(())
    }

    fn clamp_force_magnitude(max_thrust: f64, magnitude: f64) -> f64 {
        if max_thrust > 0.0 {
            magnitude.min(max_thrust)
        } else {
            magnitude
        }
    }

    fn transfer_orbit_within_tolerance(
        &self,
        id: BodyId,
        orbit_type: OrbitType,
        params: OrbitParams,
    ) -> Result<bool, ProjectError> {
        use crate::orbits::{
            elliptical_shape_from_params, is_prograde_velocity, is_retrograde_orbit,
            orbit_plane_normal, orbital_inclination_rad, radius_on_elliptical_orbit,
            target_circular_radius, target_orbit_inclination, true_anomaly_on_orbit_plane,
        };

        let position = self.body(id)?.position();
        let velocity = self.body(id)?.velocity();
        let spin = self.spin_axis();
        let radius = position.length();
        let inclination = orbital_inclination_rad(position.cross(velocity), spin);

        if matches!(
            orbit_type,
            OrbitType::EllipticalEquatorial | OrbitType::EllipticalInclined | OrbitType::Molniya
        ) {
            let plane = orbit_plane_normal(orbit_type, params);
            let speed = velocity.length();
            let vel_off = if speed > 1e-9 {
                plane.dot(velocity).abs() / speed
            } else {
                0.0
            };
            let target_inc = target_orbit_inclination(orbit_type, params);
            let (_, _, semi_major, eccentricity) =
                elliptical_shape_from_params(&self.central, orbit_type, params)?;
            let nu = true_anomaly_on_orbit_plane(
                position,
                plane,
                spin,
                self.central.prime_meridian_direction(),
            );
            let target_r = radius_on_elliptical_orbit(semi_major, eccentricity, nu);
            let shape_ok = (radius - target_r).abs() / target_r <= 0.03;
            let retro_ok = if is_retrograde_orbit(orbit_type) {
                !is_prograde_velocity(spin, position, velocity)
            } else {
                true
            };
            return Ok(vel_off <= 0.07
                && (inclination - target_inc).abs() <= 0.1
                && shape_ok
                && retro_ok);
        }

        if !matches!(
            orbit_type,
            OrbitType::CircularEquatorial
                | OrbitType::CircularPolar
                | OrbitType::Geostationary
                | OrbitType::LowCircular
                | OrbitType::RetrogradeEquatorial
                | OrbitType::EclipticPrograde
                | OrbitType::EclipticRetrograde
                | OrbitType::Tundra
                | OrbitType::Graveyard
        ) {
            return Ok(true);
        }

        let target_radius = target_circular_radius(&self.central, self.scale, orbit_type, params)?;
        let target_inclination = target_orbit_inclination(orbit_type, params);

        let radius_ok = (radius - target_radius).abs() / target_radius <= 0.01;
        let orientation_ok = match orbit_type {
            OrbitType::EclipticPrograde => {
                let plane = orbit_plane_normal(orbit_type, params);
                let speed = velocity.length();
                let vel_off = if speed > 1e-9 {
                    plane.dot(velocity).abs() / speed
                } else {
                    0.0
                };
                is_prograde_velocity(spin, position, velocity)
                    && vel_off <= 0.07
                    && (inclination - target_inclination).abs() <= 0.1
            }
            OrbitType::RetrogradeEquatorial => {
                let plane = orbit_plane_normal(orbit_type, params);
                let speed = velocity.length();
                let vel_off = if speed > 1e-9 {
                    plane.dot(velocity).abs() / speed
                } else {
                    0.0
                };
                !is_prograde_velocity(spin, position, velocity) && vel_off <= 0.07
            }
            OrbitType::EclipticRetrograde => {
                let plane = orbit_plane_normal(orbit_type, params);
                let speed = velocity.length();
                let vel_off = if speed > 1e-9 {
                    plane.dot(velocity).abs() / speed
                } else {
                    0.0
                };
                let retrograde_inc = std::f64::consts::PI - target_inclination;
                !is_prograde_velocity(spin, position, velocity)
                    && vel_off <= 0.07
                    && (inclination - retrograde_inc).abs() <= 0.1
            }
            OrbitType::CircularEquatorial | OrbitType::Geostationary | OrbitType::Graveyard => {
                let plane = orbit_plane_normal(orbit_type, params);
                let speed = velocity.length();
                let vel_off = if speed > 1e-9 {
                    plane.dot(velocity).abs() / speed
                } else {
                    0.0
                };
                vel_off <= 0.07 && (inclination - target_inclination).abs() <= 0.1
            }
            OrbitType::LowCircular | OrbitType::Tundra | OrbitType::CircularPolar => {
                (inclination - target_inclination).abs() <= 0.1
            }
            _ => inclination <= 0.07,
        };

        Ok(radius_ok && orientation_ok)
    }
}

#[cfg(test)]
mod tests {
    use super::Simulation;
    use crate::math::Vec3;
    use crate::orbits::{
        OrbitParams, OrbitType, orbital_period, specific_orbital_energy, state_on_orbit_at_position,
    };
    use crate::small_body::BodyState;
    use crate::transfer_burn::TransferBurnStatus;

    #[test]
    fn circular_orbit_period_matches_kepler() {
        let mut sim = Simulation::earth_like(86_400.0).unwrap();
        let params = OrbitParams::circular(0.1);
        let id = sim
            .create_body_in_orbit(OrbitType::CircularEquatorial, params, 1.0)
            .unwrap();
        let radius = sim.position(id).unwrap().length();
        let mu = sim.mu();
        let expected_period = orbital_period(mu, radius).unwrap();
        let dt = expected_period / 1_000.0;
        let steps = 1_000;
        for _ in 0..steps {
            sim.step(dt).unwrap();
        }
        let displacement = (sim.position(id).unwrap() - Vec3::new(radius, 0.0, 0.0)).length();
        assert!(displacement < radius * 0.05);
    }

    #[test]
    fn energy_remains_stable_without_thrust() {
        let mut sim = Simulation::earth_like(86_400.0).unwrap();
        let id = sim
            .create_body_in_orbit(
                OrbitType::CircularEquatorial,
                OrbitParams::circular(0.2),
                1.0,
            )
            .unwrap();
        let mu = sim.mu();
        let initial_energy =
            specific_orbital_energy(mu, sim.position(id).unwrap(), sim.velocity(id).unwrap());
        for _ in 0..500 {
            sim.step(10.0).unwrap();
        }
        let final_energy =
            specific_orbital_energy(mu, sim.position(id).unwrap(), sim.velocity(id).unwrap());
        assert!((final_energy - initial_energy).abs() < 1e-6);
    }

    #[test]
    fn guided_transfer_snaps_to_target_orbit() {
        let mut sim = Simulation::earth_like(86_400.0).unwrap();
        let start = OrbitParams::circular(0.1);
        let id = sim
            .create_body_in_orbit(OrbitType::CircularEquatorial, start, 1.0)
            .unwrap();
        sim.set_max_thrust(id, 0.01).unwrap();

        let target = OrbitParams::circular(0.3);
        sim.begin_transfer_to_orbit(id, OrbitType::CircularEquatorial, target)
            .unwrap();

        for _ in 0..100_000 {
            sim.step(5.0).unwrap();
            if sim.transfer_burn_status(id) == TransferBurnStatus::Finished {
                break;
            }
        }

        assert_eq!(sim.transfer_burn_status(id), TransferBurnStatus::Finished);
        let expected = state_on_orbit_at_position(
            sim.central(),
            sim.scale(),
            OrbitType::CircularEquatorial,
            target,
            sim.position(id).unwrap(),
        )
        .unwrap();
        assert!((sim.velocity(id).unwrap() - expected.velocity).length() < 1e-6);
    }

    #[test]
    fn inclination_transfer_does_not_teleport_on_first_step() {
        let mut sim = Simulation::earth_like(86_400.0).unwrap();
        let id = sim
            .create_body_in_orbit(
                OrbitType::CircularEquatorial,
                OrbitParams::circular(0.1),
                1.0,
            )
            .unwrap();
        sim.set_max_thrust(id, 0.001).unwrap();
        let start_pos = sim.position(id).unwrap();
        sim.begin_transfer_to_orbit(
            id,
            OrbitType::LowCircular,
            OrbitParams::circular_inclined(0.1, crate::orbits::LOW_EARTH_INCLINATION_RAD),
        )
        .unwrap();
        sim.step(0.1).unwrap();
        assert_eq!(sim.transfer_burn_status(id), TransferBurnStatus::Burning);
        assert!((sim.position(id).unwrap() - start_pos).length() < 0.02);
    }

    #[test]
    fn manual_thrust_cancels_transfer() {
        let mut sim = Simulation::earth_like(86_400.0).unwrap();
        let id = sim
            .create_body_in_orbit(
                OrbitType::CircularEquatorial,
                OrbitParams::circular(0.1),
                1.0,
            )
            .unwrap();
        sim.set_max_thrust(id, 0.01).unwrap();
        sim.begin_transfer_to_orbit(
            id,
            OrbitType::CircularEquatorial,
            OrbitParams::circular(0.3),
        )
        .unwrap();
        assert_eq!(sim.transfer_burn_status(id), TransferBurnStatus::Burning);

        sim.apply_force(id, Vec3::new(1.0, 0.0, 0.0), 0.001)
            .unwrap();
        assert_eq!(sim.transfer_burn_status(id), TransferBurnStatus::Idle);
    }

    #[test]
    fn surface_contact_on_impact() {
        let mut sim = Simulation::earth_like(86_400.0).unwrap();
        let id = sim
            .create_body(Vec3::new(1.01, 0.0, 0.0), Vec3::new(-10.0, 0.0, 0.0), 1.0)
            .unwrap();
        for _ in 0..100 {
            sim.step(0.01).unwrap();
            if sim.state(id).unwrap() == BodyState::SurfaceContact {
                break;
            }
        }
        assert_eq!(sim.state(id).unwrap(), BodyState::SurfaceContact);
        assert!((sim.position(id).unwrap().length() - 1.0).abs() < 1e-6);
    }
}
