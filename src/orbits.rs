//! Orbit types, initial conditions, and transfer delta-v helpers.

use crate::central_body::CentralBody;
use crate::error::ProjectError;
use crate::math::Vec3;
use crate::star::ecliptic_plane_normal;
use crate::units::SimulationScale;

/// Typical ISS-like low Earth orbit inclination in radians (~51.6°).
pub const LOW_EARTH_INCLINATION_RAD: f64 = 0.901;
/// High-inclination orbits (Molniya / Tundra) in radians (~63.4°).
pub const HIGH_INCLINATION_RAD: f64 = 1.107;
/// Molniya perigee altitude above the surface in Earth radii (~500 km).
pub const MOLNIYA_PERIGEE_ALTITUDE_R: f64 = 0.078;
/// Molniya apogee altitude above the surface in Earth radii (~40,000 km).
pub const MOLNIYA_APOGEE_ALTITUDE_R: f64 = 5.27;
/// Graveyard orbit altitude offset above geostationary in Earth radii (~300 km).
pub const GRAVEYARD_ALTITUDE_OFFSET_R: f64 = 0.05;

/// Known orbit configurations for helper functions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrbitType {
    /// Prograde circular orbit in the equatorial plane (inclination forced to zero).
    CircularEquatorial,
    /// Circular orbit passing over the poles (inclination 90°).
    CircularPolar,
    /// Circular equatorial orbit with period matching planet rotation.
    Geostationary,
    /// Low circular orbit with configurable inclination (0 = equatorial).
    LowCircular,
    /// Retrograde circular orbit in the equatorial plane.
    RetrogradeEquatorial,
    /// Coplanar elliptical orbit in the equatorial plane.
    EllipticalEquatorial,
    /// Prograde circular orbit in the planet–sun (ecliptic) plane.
    EclipticPrograde,
    /// Retrograde circular orbit in the planet–sun (ecliptic) plane.
    EclipticRetrograde,
    /// Elliptical orbit with configurable inclination (ascending node on +X).
    EllipticalInclined,
    /// Inclined circular orbit at geostationary altitude (Tundra-class).
    Tundra,
    /// Highly elliptical inclined orbit (Molniya-class, 12 h period).
    Molniya,
    /// Supersynchronous circular graveyard orbit above GEO.
    Graveyard,
}

/// Parameters for constructing or transferring to an orbit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitParams {
    /// Altitude above surface in Earth radii (for circular types).
    pub altitude_earth_radii: f64,
    /// Perigee altitude above surface in Earth radii (elliptical).
    pub perigee_altitude_earth_radii: f64,
    /// Apogee altitude above surface in Earth radii (elliptical).
    pub apogee_altitude_earth_radii: f64,
    /// True anomaly at epoch in radians (where the body starts on the orbit).
    pub true_anomaly_rad: f64,
    /// Inclination from the equatorial plane in radians (ascending node on +X).
    pub inclination_rad: f64,
    /// Ecliptic obliquity in radians (used for ecliptic orbit types; usually from the star config).
    pub obliquity_rad: f64,
}

impl Default for OrbitParams {
    fn default() -> Self {
        Self {
            altitude_earth_radii: 0.05,
            perigee_altitude_earth_radii: 0.02,
            apogee_altitude_earth_radii: 0.2,
            true_anomaly_rad: 0.0,
            inclination_rad: 0.0,
            obliquity_rad: 0.0,
        }
    }
}

impl OrbitParams {
    /// Parameters for a circular orbit at the given altitude (equatorial).
    pub fn circular(altitude_earth_radii: f64) -> Self {
        Self {
            altitude_earth_radii,
            ..Self::default()
        }
    }

    /// Parameters for a low circular orbit with optional inclination (defaults to ISS-like).
    pub fn low_circular(altitude_earth_radii: f64) -> Self {
        Self {
            altitude_earth_radii,
            inclination_rad: LOW_EARTH_INCLINATION_RAD,
            ..Self::default()
        }
    }

    /// Parameters for a circular orbit at the given altitude and inclination.
    pub fn circular_inclined(altitude_earth_radii: f64, inclination_rad: f64) -> Self {
        Self {
            altitude_earth_radii,
            inclination_rad,
            ..Self::default()
        }
    }

    /// Parameters for an elliptical equatorial orbit.
    pub fn elliptical(perigee_alt: f64, apogee_alt: f64) -> Self {
        Self {
            perigee_altitude_earth_radii: perigee_alt,
            apogee_altitude_earth_radii: apogee_alt,
            ..Self::default()
        }
    }

    /// Parameters for an elliptical inclined orbit.
    pub fn elliptical_inclined(perigee_alt: f64, apogee_alt: f64, inclination_rad: f64) -> Self {
        Self {
            perigee_altitude_earth_radii: perigee_alt,
            apogee_altitude_earth_radii: apogee_alt,
            inclination_rad,
            ..Self::default()
        }
    }

    /// Parameters for a Molniya orbit (defaults to classically inclined 12 h ellipse).
    pub fn molniya(true_anomaly_rad: f64) -> Self {
        Self {
            perigee_altitude_earth_radii: MOLNIYA_PERIGEE_ALTITUDE_R,
            apogee_altitude_earth_radii: MOLNIYA_APOGEE_ALTITUDE_R,
            inclination_rad: HIGH_INCLINATION_RAD,
            true_anomaly_rad,
            ..Self::default()
        }
    }

    /// Parameters for geostationary orbit; altitude fields are ignored.
    pub fn geostationary(true_anomaly_rad: f64) -> Self {
        Self {
            true_anomaly_rad,
            ..Self::default()
        }
    }

    /// Parameters for a Tundra orbit at GEO altitude with optional inclination (defaults to ~63.4°).
    pub fn tundra(inclination_rad: f64) -> Self {
        Self {
            inclination_rad: if inclination_rad.abs() <= f64::EPSILON {
                HIGH_INCLINATION_RAD
            } else {
                inclination_rad
            },
            ..Self::default()
        }
    }
}

/// Suggested default UI field values when the user selects an orbit type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitUiDefaults {
    /// Default circular altitude above the surface (Earth radii).
    pub altitude_earth_radii: f64,
    /// Default perigee altitude (Earth radii).
    pub perigee_altitude_earth_radii: f64,
    /// Default apogee altitude (Earth radii).
    pub apogee_altitude_earth_radii: f64,
    /// Default inclination (radians).
    pub inclination_rad: f64,
}

/// Whether the orbit type uses perigee/apogee fields in the UI (elliptical and Molniya).
pub fn orbit_uses_elliptical_params(orbit_type: OrbitType) -> bool {
    matches!(
        orbit_type,
        OrbitType::EllipticalEquatorial | OrbitType::EllipticalInclined | OrbitType::Molniya
    )
}

/// Whether the orbit type uses an inclination UI field.
pub fn orbit_uses_inclination_param(orbit_type: OrbitType) -> bool {
    matches!(
        orbit_type,
        OrbitType::LowCircular
            | OrbitType::EllipticalInclined
            | OrbitType::Tundra
            | OrbitType::Molniya
    )
}

/// Whether altitude is computed from planet physics (GEO, graveyard, tundra) rather than user input.
pub fn orbit_uses_computed_altitude(orbit_type: OrbitType) -> bool {
    matches!(
        orbit_type,
        OrbitType::Geostationary | OrbitType::Graveyard | OrbitType::Tundra
    )
}

/// Default UI values for spawning or transferring to an orbit type.
pub fn orbit_ui_defaults(orbit_type: OrbitType) -> OrbitUiDefaults {
    match orbit_type {
        OrbitType::Molniya => OrbitUiDefaults {
            altitude_earth_radii: MOLNIYA_PERIGEE_ALTITUDE_R,
            perigee_altitude_earth_radii: MOLNIYA_PERIGEE_ALTITUDE_R,
            apogee_altitude_earth_radii: MOLNIYA_APOGEE_ALTITUDE_R,
            inclination_rad: HIGH_INCLINATION_RAD,
        },
        OrbitType::Tundra => OrbitUiDefaults {
            altitude_earth_radii: 0.0,
            perigee_altitude_earth_radii: 0.0,
            apogee_altitude_earth_radii: 0.0,
            inclination_rad: HIGH_INCLINATION_RAD,
        },
        OrbitType::LowCircular => OrbitUiDefaults {
            altitude_earth_radii: 0.1,
            perigee_altitude_earth_radii: 0.1,
            apogee_altitude_earth_radii: 0.2,
            inclination_rad: LOW_EARTH_INCLINATION_RAD,
        },
        _ => OrbitUiDefaults {
            altitude_earth_radii: 0.1,
            perigee_altitude_earth_radii: 0.1,
            apogee_altitude_earth_radii: 0.2,
            inclination_rad: 0.0,
        },
    }
}

/// Builds [`OrbitParams`] from generic UI fields (altitude, perigee, apogee, inclination).
///
/// Matches the mapping used by Godot `spawn_body_in_orbit` / `begin_transfer_to_orbit`.
pub fn build_orbit_params_from_ui(
    orbit_type: OrbitType,
    altitude_earth_radii: f64,
    perigee_altitude: f64,
    apogee_altitude: f64,
    inclination_rad: f64,
) -> OrbitParams {
    match orbit_type {
        OrbitType::EllipticalEquatorial => {
            OrbitParams::elliptical(perigee_altitude, apogee_altitude)
        }
        OrbitType::EllipticalInclined => {
            OrbitParams::elliptical_inclined(perigee_altitude, apogee_altitude, inclination_rad)
        }
        OrbitType::LowCircular => {
            OrbitParams::circular_inclined(altitude_earth_radii, inclination_rad)
        }
        OrbitType::Tundra => OrbitParams::tundra(inclination_rad),
        OrbitType::Molniya => OrbitParams {
            perigee_altitude_earth_radii: perigee_altitude,
            apogee_altitude_earth_radii: apogee_altitude,
            inclination_rad,
            ..OrbitParams::molniya(std::f64::consts::PI)
        },
        _ => {
            let mut params = OrbitParams::circular(altitude_earth_radii);
            params.inclination_rad = inclination_rad;
            params
        }
    }
}

/// Initial position and velocity for a body in the requested orbit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitState {
    /// Position in simulation units.
    pub position: Vec3,
    /// Velocity in simulation units per second.
    pub velocity: Vec3,
}

/// Computes circular orbital speed at radius `r`.
pub fn circular_orbital_speed(mu: f64, radius: f64) -> Result<f64, ProjectError> {
    if radius <= 0.0 {
        return Err(ProjectError::InvalidOrbit("radius must be positive"));
    }
    Ok((mu / radius).sqrt())
}

/// Geostationary orbital radius for a given angular rate.
pub fn geostationary_radius(mu: f64, angular_rate_rad_s: f64) -> Result<f64, ProjectError> {
    if angular_rate_rad_s <= 0.0 {
        return Err(ProjectError::InvalidOrbit("angular rate must be positive"));
    }
    Ok((mu / (angular_rate_rad_s * angular_rate_rad_s)).powf(1.0 / 3.0))
}

/// Geostationary altitude above the surface in Earth radii.
pub fn geostationary_altitude_earth_radii(
    mu: f64,
    angular_rate_rad_s: f64,
    surface_radius: f64,
) -> Result<f64, ProjectError> {
    Ok(geostationary_radius(mu, angular_rate_rad_s)? - surface_radius)
}

/// Graveyard orbit altitude above the surface in Earth radii.
pub fn graveyard_altitude_earth_radii(
    mu: f64,
    angular_rate_rad_s: f64,
    surface_radius: f64,
) -> Result<f64, ProjectError> {
    Ok(
        geostationary_altitude_earth_radii(mu, angular_rate_rad_s, surface_radius)?
            + GRAVEYARD_ALTITUDE_OFFSET_R,
    )
}

/// Orbital inclination in radians from angular momentum and spin axis (+Y).
pub fn orbital_inclination_rad(angular_momentum: Vec3, spin_axis: Vec3) -> f64 {
    let h = angular_momentum.length();
    if h <= f64::EPSILON {
        return 0.0;
    }
    angular_momentum
        .normalized()
        .dot(spin_axis.normalized())
        .clamp(-1.0, 1.0)
        .acos()
}

/// Builds initial state for the requested orbit type.
pub fn initial_state_for_orbit(
    central: &CentralBody,
    scale: SimulationScale,
    orbit_type: OrbitType,
    params: OrbitParams,
) -> Result<OrbitState, ProjectError> {
    central.validate()?;
    let mu = central.mu(scale);
    let surface = central.surface_radius();

    match orbit_type {
        OrbitType::CircularEquatorial => {
            let radius = surface + params.altitude_earth_radii;
            circular_inclined_state(
                central,
                mu,
                radius,
                params.true_anomaly_rad,
                orbit_normal_from_inclination(0.0),
                false,
            )
        }
        OrbitType::LowCircular => {
            let radius = surface + params.altitude_earth_radii;
            circular_inclined_state(
                central,
                mu,
                radius,
                params.true_anomaly_rad,
                orbit_normal_from_inclination(params.inclination_rad),
                false,
            )
        }
        OrbitType::RetrogradeEquatorial => {
            let radius = surface + params.altitude_earth_radii;
            circular_inclined_state(
                central,
                mu,
                radius,
                params.true_anomaly_rad,
                orbit_normal_from_inclination(0.0),
                true,
            )
        }
        OrbitType::Geostationary => {
            let radius = geostationary_radius(mu, central.angular_rate_rad_s())?;
            if radius <= surface {
                return Err(ProjectError::InvalidOrbit(
                    "geostationary radius is below the surface",
                ));
            }
            geostationary_equatorial_state(central, radius, params.true_anomaly_rad)
        }
        OrbitType::CircularPolar => {
            let radius = surface + params.altitude_earth_radii;
            circular_inclined_state(
                central,
                mu,
                radius,
                params.true_anomaly_rad,
                orbit_normal_from_inclination(std::f64::consts::FRAC_PI_2),
                false,
            )
        }
        OrbitType::EclipticPrograde => {
            let radius = surface + params.altitude_earth_radii;
            circular_inclined_state(
                central,
                mu,
                radius,
                params.true_anomaly_rad,
                ecliptic_plane_normal(params.obliquity_rad),
                false,
            )
        }
        OrbitType::EclipticRetrograde => {
            let radius = surface + params.altitude_earth_radii;
            circular_inclined_state(
                central,
                mu,
                radius,
                params.true_anomaly_rad,
                ecliptic_plane_normal(params.obliquity_rad),
                true,
            )
        }
        OrbitType::EllipticalEquatorial => elliptical_inclined_state(
            central,
            mu,
            surface,
            params.perigee_altitude_earth_radii,
            params.apogee_altitude_earth_radii,
            params.true_anomaly_rad,
            0.0,
        ),
        OrbitType::EllipticalInclined => elliptical_inclined_state(
            central,
            mu,
            surface,
            params.perigee_altitude_earth_radii,
            params.apogee_altitude_earth_radii,
            params.true_anomaly_rad,
            params.inclination_rad,
        ),
        OrbitType::Tundra => {
            let radius = geostationary_radius(mu, central.angular_rate_rad_s())?;
            circular_inclined_state(
                central,
                mu,
                radius,
                params.true_anomaly_rad,
                orbit_normal_from_inclination(params.inclination_rad),
                false,
            )
        }
        OrbitType::Molniya => {
            let perigee = if params.perigee_altitude_earth_radii.abs() <= f64::EPSILON {
                MOLNIYA_PERIGEE_ALTITUDE_R
            } else {
                params.perigee_altitude_earth_radii
            };
            let apogee = if params.apogee_altitude_earth_radii <= perigee + f64::EPSILON {
                MOLNIYA_APOGEE_ALTITUDE_R
            } else {
                params.apogee_altitude_earth_radii
            };
            let true_anomaly = if params.true_anomaly_rad.abs() <= f64::EPSILON {
                std::f64::consts::PI
            } else {
                params.true_anomaly_rad
            };
            elliptical_inclined_state(
                central,
                mu,
                surface,
                perigee,
                apogee,
                true_anomaly,
                params.inclination_rad,
            )
        }
        OrbitType::Graveyard => {
            let radius = geostationary_radius(mu, central.angular_rate_rad_s())?
                + GRAVEYARD_ALTITUDE_OFFSET_R;
            circular_inclined_state(
                central,
                mu,
                radius,
                params.true_anomaly_rad,
                orbit_normal_from_inclination(0.0),
                false,
            )
        }
    }
}

fn orbit_normal_from_inclination(inclination_rad: f64) -> Vec3 {
    Vec3::new(0.0, inclination_rad.cos(), inclination_rad.sin()).normalized()
}

fn ascending_node_direction(spin_axis: Vec3, orbit_normal: Vec3, prime_meridian: Vec3) -> Vec3 {
    if spin_axis.cross(orbit_normal).length_squared() > f64::EPSILON {
        spin_axis.cross(orbit_normal).normalized()
    } else {
        prime_meridian
    }
}

/// True anomaly of `position` on the orbit plane with the given normal (ascending node on +X).
pub fn true_anomaly_on_orbit_plane(
    position: Vec3,
    orbit_normal: Vec3,
    spin_axis: Vec3,
    prime_meridian: Vec3,
) -> f64 {
    let node = ascending_node_direction(spin_axis, orbit_normal, prime_meridian);
    let in_plane = orbit_normal.cross(node).normalized();
    let r_hat = position.normalized();
    in_plane.dot(r_hat).atan2(node.dot(r_hat))
}

/// Unit normal of the target orbital plane (+Y spin axis is equatorial for inclination 0).
pub fn orbit_plane_normal(orbit_type: OrbitType, params: OrbitParams) -> Vec3 {
    orbit_normal_for_type(orbit_type, params)
}

fn orbit_normal_for_type(orbit_type: OrbitType, params: OrbitParams) -> Vec3 {
    match orbit_type {
        OrbitType::CircularEquatorial
        | OrbitType::Geostationary
        | OrbitType::Graveyard
        | OrbitType::RetrogradeEquatorial
        | OrbitType::EllipticalEquatorial => orbit_normal_from_inclination(0.0),
        OrbitType::LowCircular
        | OrbitType::Tundra
        | OrbitType::EllipticalInclined
        | OrbitType::Molniya => orbit_normal_from_inclination(params.inclination_rad),
        OrbitType::CircularPolar => orbit_normal_from_inclination(std::f64::consts::FRAC_PI_2),
        OrbitType::EclipticPrograde | OrbitType::EclipticRetrograde => {
            ecliptic_plane_normal(params.obliquity_rad)
        }
    }
}

/// Whether the orbit type is retrograde relative to the central body spin.
pub fn is_retrograde_orbit(orbit_type: OrbitType) -> bool {
    matches!(
        orbit_type,
        OrbitType::RetrogradeEquatorial | OrbitType::EclipticRetrograde
    )
}

fn is_circular_orbit_type(orbit_type: OrbitType) -> bool {
    matches!(
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
    )
}

/// Whether the target orbit type is elliptical (non-circular conic).
pub fn is_elliptical_orbit_type(orbit_type: OrbitType) -> bool {
    matches!(
        orbit_type,
        OrbitType::EllipticalEquatorial | OrbitType::EllipticalInclined | OrbitType::Molniya
    )
}

/// Perigee, apogee, semi-major axis, and eccentricity for elliptical target parameters.
pub fn elliptical_shape_from_params(
    central: &CentralBody,
    orbit_type: OrbitType,
    params: OrbitParams,
) -> Result<(f64, f64, f64, f64), ProjectError> {
    let surface = central.surface_radius();
    let (perigee_alt, apogee_alt) = match orbit_type {
        OrbitType::Molniya => {
            let perigee = if params.perigee_altitude_earth_radii.abs() <= f64::EPSILON {
                MOLNIYA_PERIGEE_ALTITUDE_R
            } else {
                params.perigee_altitude_earth_radii
            };
            let apogee = if params.apogee_altitude_earth_radii <= perigee + f64::EPSILON {
                MOLNIYA_APOGEE_ALTITUDE_R
            } else {
                params.apogee_altitude_earth_radii
            };
            (perigee, apogee)
        }
        OrbitType::EllipticalEquatorial | OrbitType::EllipticalInclined => (
            params.perigee_altitude_earth_radii,
            params.apogee_altitude_earth_radii,
        ),
        _ => {
            return Err(ProjectError::InvalidOrbit(
                "elliptical shape is only defined for elliptical orbit types",
            ));
        }
    };
    let rp = surface + perigee_alt;
    let ra = surface + apogee_alt;
    if rp <= surface || ra <= rp {
        return Err(ProjectError::InvalidOrbit(
            "apogee must be greater than perigee above the surface",
        ));
    }
    let semi_major = (rp + ra) / 2.0;
    let eccentricity = (ra - rp) / (ra + rp);
    Ok((rp, ra, semi_major, eccentricity))
}

/// Radius on an ellipse at true anomaly (radians).
pub fn radius_on_elliptical_orbit(
    semi_major: f64,
    eccentricity: f64,
    true_anomaly_rad: f64,
) -> f64 {
    let p = semi_major * (1.0 - eccentricity * eccentricity);
    p / (1.0 + eccentricity * true_anomaly_rad.cos())
}

/// Target inclination in radians for circular and elliptical orbit types.
pub fn target_orbit_inclination(orbit_type: OrbitType, params: OrbitParams) -> f64 {
    match orbit_type {
        OrbitType::LowCircular
        | OrbitType::Tundra
        | OrbitType::EllipticalInclined
        | OrbitType::Molniya => params.inclination_rad,
        OrbitType::CircularPolar => std::f64::consts::FRAC_PI_2,
        OrbitType::EclipticPrograde | OrbitType::EclipticRetrograde => params.obliquity_rad.abs(),
        _ => 0.0,
    }
}

/// Characteristic radius used for altitude comparisons (circular radius or semi-major axis).
pub fn target_orbit_reference_radius(
    central: &CentralBody,
    scale: SimulationScale,
    orbit_type: OrbitType,
    params: OrbitParams,
) -> Result<f64, ProjectError> {
    if is_elliptical_orbit_type(orbit_type) {
        let (_, _, semi_major, _) = elliptical_shape_from_params(central, orbit_type, params)?;
        return Ok(semi_major);
    }
    target_circular_radius(central, scale, orbit_type, params)
}

/// Angle between the current orbital plane and a target plane normal (radians).
pub fn orbit_plane_change_rad(position: Vec3, velocity: Vec3, target_plane_normal: Vec3) -> f64 {
    let angular_momentum = position.cross(velocity);
    if angular_momentum.length_squared() <= f64::EPSILON {
        return 0.0;
    }
    let current_normal = angular_momentum.normalized();
    let target = target_plane_normal.normalized();
    current_normal.dot(target).clamp(-1.0, 1.0).acos()
}

/// Target circular orbital radius for the given orbit type and parameters.
pub fn target_circular_radius(
    central: &CentralBody,
    scale: SimulationScale,
    orbit_type: OrbitType,
    params: OrbitParams,
) -> Result<f64, ProjectError> {
    let mu = central.mu(scale);
    let surface = central.surface_radius();
    match orbit_type {
        OrbitType::Geostationary | OrbitType::Tundra => {
            geostationary_radius(mu, central.angular_rate_rad_s())
        }
        OrbitType::Graveyard => {
            Ok(geostationary_radius(mu, central.angular_rate_rad_s())?
                + GRAVEYARD_ALTITUDE_OFFSET_R)
        }
        _ if is_circular_orbit_type(orbit_type) => Ok(surface + params.altitude_earth_radii),
        _ => Err(ProjectError::InvalidOrbit(
            "target radius is only defined for circular orbit types",
        )),
    }
}

/// Canonical position and velocity for the target orbit at the body's current location.
pub fn state_on_orbit_at_position(
    central: &CentralBody,
    scale: SimulationScale,
    orbit_type: OrbitType,
    params: OrbitParams,
    position: Vec3,
) -> Result<OrbitState, ProjectError> {
    let mu = central.mu(scale);
    let surface = central.surface_radius();
    let axis = central.spin_axis_normalized();
    let prime = central.prime_meridian_direction();
    let radius = position.length();
    if radius <= surface {
        return Err(ProjectError::InvalidOrbit(
            "position must be above the surface",
        ));
    }

    match orbit_type {
        OrbitType::Geostationary => {
            let geo_radius = geostationary_radius(mu, central.angular_rate_rad_s())?;
            let normal = orbit_normal_from_inclination(0.0);
            let true_anomaly = true_anomaly_on_orbit_plane(position, normal, axis, prime);
            geostationary_equatorial_state(central, geo_radius, true_anomaly)
        }
        OrbitType::EllipticalEquatorial => elliptical_inclined_state(
            central,
            mu,
            surface,
            params.perigee_altitude_earth_radii,
            params.apogee_altitude_earth_radii,
            true_anomaly_on_orbit_plane(position, orbit_normal_from_inclination(0.0), axis, prime),
            0.0,
        ),
        OrbitType::EllipticalInclined | OrbitType::Molniya => {
            let normal = orbit_normal_for_type(orbit_type, params);
            elliptical_inclined_state(
                central,
                mu,
                surface,
                params.perigee_altitude_earth_radii,
                params.apogee_altitude_earth_radii,
                true_anomaly_on_orbit_plane(position, normal, axis, prime),
                params.inclination_rad,
            )
        }
        _ => {
            let normal = orbit_normal_for_type(orbit_type, params);
            let true_anomaly = true_anomaly_on_orbit_plane(position, normal, axis, prime);
            circular_inclined_state(
                central,
                mu,
                radius,
                true_anomaly,
                normal,
                is_retrograde_orbit(orbit_type),
            )
        }
    }
}

fn circular_inclined_state(
    central: &CentralBody,
    mu: f64,
    radius: f64,
    true_anomaly_rad: f64,
    orbit_normal: Vec3,
    retrograde: bool,
) -> Result<OrbitState, ProjectError> {
    if radius <= central.surface_radius() {
        return Err(ProjectError::InvalidOrbit(
            "orbit radius must be above the surface",
        ));
    }

    let axis = central.spin_axis_normalized();
    let normal = orbit_normal.normalized();
    let node = ascending_node_direction(axis, normal, central.prime_meridian_direction());
    let in_plane = normal.cross(node).normalized();
    let position = (node * true_anomaly_rad.cos() + in_plane * true_anomaly_rad.sin()) * radius;

    let mut tangential = normal.cross(position).normalized();
    let spin_aligned = axis.cross(position);
    if spin_aligned.length_squared() > f64::EPSILON && tangential.dot(spin_aligned) < 0.0 {
        tangential = -tangential;
    }
    if retrograde {
        tangential = -tangential;
    }

    let speed = circular_orbital_speed(mu, radius)?;
    Ok(OrbitState {
        position,
        velocity: tangential * speed,
    })
}

fn geostationary_equatorial_state(
    central: &CentralBody,
    radius: f64,
    true_anomaly_rad: f64,
) -> Result<OrbitState, ProjectError> {
    if radius <= central.surface_radius() {
        return Err(ProjectError::InvalidOrbit(
            "orbit radius must be above the surface",
        ));
    }

    let axis = central.spin_axis_normalized();
    let prime = central.prime_meridian_direction();
    let position =
        (prime * true_anomaly_rad.cos() + axis.cross(prime) * true_anomaly_rad.sin()) * radius;
    let prograde = axis.cross(position).normalized();
    let omega = central.angular_rate_rad_s();
    let speed = omega * radius;

    Ok(OrbitState {
        position,
        velocity: prograde * speed,
    })
}

fn elliptical_inclined_state(
    central: &CentralBody,
    mu: f64,
    surface: f64,
    perigee_alt: f64,
    apogee_alt: f64,
    true_anomaly_rad: f64,
    inclination_rad: f64,
) -> Result<OrbitState, ProjectError> {
    let rp = surface + perigee_alt;
    let ra = surface + apogee_alt;
    if rp <= surface || ra <= rp {
        return Err(ProjectError::InvalidOrbit(
            "apogee must be greater than perigee above the surface",
        ));
    }

    let semi_major = (rp + ra) / 2.0;
    let eccentricity = (ra - rp) / (ra + rp);
    let axis = central.spin_axis_normalized();
    let normal = orbit_normal_from_inclination(inclination_rad);
    let node = if axis.cross(normal).length_squared() > f64::EPSILON {
        axis.cross(normal).normalized()
    } else {
        central.prime_meridian_direction()
    };
    let in_plane = normal.cross(node).normalized();

    let radius = semi_major * (1.0 - eccentricity * eccentricity)
        / (1.0 + eccentricity * true_anomaly_rad.cos());
    let position = (node * true_anomaly_rad.cos() + in_plane * true_anomaly_rad.sin()) * radius;

    let speed = (mu * (2.0 / radius - 1.0 / semi_major)).sqrt();
    let flight_path = (eccentricity * true_anomaly_rad.sin()
        / (1.0 + eccentricity * true_anomaly_rad.cos()))
    .atan();
    let radial_hat = position.normalized();
    let mut tangential_hat = normal.cross(position).normalized();
    let spin_aligned = axis.cross(position);
    if spin_aligned.length_squared() > f64::EPSILON && tangential_hat.dot(spin_aligned) < 0.0 {
        tangential_hat = -tangential_hat;
    }
    let velocity =
        radial_hat * (speed * flight_path.sin()) + tangential_hat * (speed * flight_path.cos());

    Ok(OrbitState { position, velocity })
}

/// Required instantaneous delta-v to reach the target orbit from the current state.
pub fn required_delta_v_to_orbit(
    central: &CentralBody,
    scale: SimulationScale,
    position: Vec3,
    velocity: Vec3,
    orbit_type: OrbitType,
    params: OrbitParams,
) -> Result<Vec3, ProjectError> {
    required_delta_v_to_orbit_with_mission(
        central, scale, position, velocity, orbit_type, params, false,
    )
}

/// Like [`required_delta_v_to_orbit`], with explicit lowering vs raising intent for altitude guidance.
pub fn required_delta_v_to_orbit_with_mission(
    central: &CentralBody,
    scale: SimulationScale,
    position: Vec3,
    velocity: Vec3,
    orbit_type: OrbitType,
    params: OrbitParams,
    lowering_mission: bool,
) -> Result<Vec3, ProjectError> {
    let mu = central.mu(scale);
    let radius = position.length();
    if radius <= central.surface_radius() {
        return Err(ProjectError::InvalidOrbit(
            "position must be above the surface",
        ));
    }

    let plane_normal = orbit_plane_normal(orbit_type, params).normalized();
    let off_plane_vel = plane_normal * velocity.dot(plane_normal);
    let coplanar_vel = off_plane_vel.length() <= 0.01 * velocity.length().max(1e-6);

    if !is_circular_orbit_type(orbit_type) {
        return non_circular_target_delta_v(central, scale, position, velocity, orbit_type, params);
    }

    let target_radius = target_circular_radius(central, scale, orbit_type, params)?;
    let radius_match_tol = 1e-5 * target_radius.max(1.0);

    if (radius - target_radius).abs() <= radius_match_tol {
        let pos_off = plane_normal.dot(position).abs() / radius;
        let reference = if pos_off > 0.08 {
            target_orbit_reference_position(position, plane_normal, target_radius)
        } else {
            position
        };
        let v_target =
            target_circular_velocity_at_position(central, scale, orbit_type, params, reference)?;
        return Ok(v_target - velocity);
    }

    if !coplanar_vel {
        let reference = target_orbit_reference_position(position, plane_normal, target_radius);
        let v_target =
            target_circular_velocity_at_position(central, scale, orbit_type, params, reference)?;
        return Ok(v_target - velocity);
    }

    coplanar_altitude_delta_v(
        central,
        scale,
        mu,
        orbit_type,
        params,
        position,
        velocity,
        radius,
        target_radius,
        lowering_mission,
    )
}

/// Reference position on the target circular orbit (target radius, same orbital angle).
fn target_orbit_reference_position(position: Vec3, plane_normal: Vec3, target_radius: f64) -> Vec3 {
    let on_plane = project_position_onto_orbit_plane(position, plane_normal);
    if on_plane.length_squared() > f64::EPSILON {
        on_plane.normalized() * target_radius
    } else {
        position.normalized() * target_radius
    }
}

/// Delta-v toward a non-circular target (elliptical / Molniya): match target state at current angle.
fn non_circular_target_delta_v(
    central: &CentralBody,
    scale: SimulationScale,
    position: Vec3,
    velocity: Vec3,
    orbit_type: OrbitType,
    params: OrbitParams,
) -> Result<Vec3, ProjectError> {
    let plane_normal = orbit_plane_normal(orbit_type, params).normalized();
    let on_plane = project_position_onto_orbit_plane(position, plane_normal);
    let target_state = state_on_orbit_at_position(central, scale, orbit_type, params, on_plane)?;

    let mut delta_v = target_state.velocity - velocity;
    let pos_err = target_state.position - position;
    let pos_err_mag = pos_err.length();
    if pos_err_mag > f64::EPSILON {
        let speed = velocity.length().max(1e-6);
        let ref_radius = target_state.position.length().max(1.0);
        let gain = (pos_err_mag / ref_radius).min(0.12);
        delta_v += pos_err.normalized() * speed * gain;
    }
    Ok(delta_v)
}

/// Coplanar altitude change: Hohmann when raising; mission-aware lowering guidance.
#[allow(clippy::too_many_arguments)]
fn coplanar_altitude_delta_v(
    central: &CentralBody,
    scale: SimulationScale,
    mu: f64,
    orbit_type: OrbitType,
    params: OrbitParams,
    position: Vec3,
    velocity: Vec3,
    radius: f64,
    target_radius: f64,
    lowering_mission: bool,
) -> Result<Vec3, ProjectError> {
    let prograde = in_plane_prograde_hat(central, orbit_type, params, position)?;
    let hohmann = hohmann_delta_v(mu, radius, target_radius, prograde)?;
    let hohmann_prograde = hohmann.dot(prograde) >= 0.0;
    let target_energy = target_circular_specific_energy(central, mu, orbit_type, target_radius);
    let current_energy = specific_orbital_energy(mu, position, velocity);
    let energy_tol = 1e-5 * mu / target_radius.max(1.0);

    if lowering_mission {
        let plane_normal = orbit_plane_normal(orbit_type, params).normalized();
        let target_pos = target_orbit_reference_position(position, plane_normal, target_radius);
        let target_state =
            state_on_orbit_at_position(central, scale, orbit_type, params, target_pos)?;

        let mut delta_v = target_state.velocity - velocity;
        let pos_err = target_pos - position;
        let pos_err_mag = pos_err.length();
        if pos_err_mag > f64::EPSILON {
            let speed = velocity.length().max(1e-6);
            let gain = (pos_err_mag / target_radius.max(1.0)).min(0.12);
            delta_v += pos_err.normalized() * speed * gain;
        }

        return Ok(delta_v);
    }

    if target_radius + 1e-9 < radius && hohmann_prograde {
        if current_energy > target_energy + energy_tol {
            let speed = velocity.length().max(1e-6);
            let dv_mag = ((current_energy - target_energy) / speed).min(speed * 0.1);
            return Ok(-prograde * dv_mag);
        }
        return Ok(Vec3::ZERO);
    }

    Ok(hohmann)
}

/// Circular target velocity at the actual `position` (same radius, target plane).
pub fn target_circular_velocity_at_position(
    central: &CentralBody,
    scale: SimulationScale,
    orbit_type: OrbitType,
    params: OrbitParams,
    position: Vec3,
) -> Result<Vec3, ProjectError> {
    let mu = central.mu(scale);
    let radius = position.length();
    if radius <= central.surface_radius() {
        return Err(ProjectError::InvalidOrbit(
            "position must be above the surface",
        ));
    }

    let plane_normal = orbit_plane_normal(orbit_type, params).normalized();
    let radial = position.normalized();
    let mut tangential = plane_normal.cross(radial).normalized();
    let spin_aligned = central.spin_axis_normalized().cross(position);
    if spin_aligned.length_squared() > f64::EPSILON && tangential.dot(spin_aligned) < 0.0 {
        tangential = -tangential;
    }
    if is_retrograde_orbit(orbit_type) {
        tangential = -tangential;
    }

    let speed = match orbit_type {
        OrbitType::Geostationary => {
            let geo_radius = geostationary_radius(mu, central.angular_rate_rad_s())?;
            if (radius - geo_radius).abs() / geo_radius <= 0.01 {
                central.angular_rate_rad_s() * radius
            } else {
                circular_orbital_speed(mu, radius)?
            }
        }
        _ => circular_orbital_speed(mu, radius)?,
    };

    Ok(tangential * speed)
}

/// First Hohmann impulse at `r1` toward a circular orbit at `r2` (sign follows `v_transfer - v1`).
fn hohmann_delta_v(mu: f64, r1: f64, r2: f64, prograde_hat: Vec3) -> Result<Vec3, ProjectError> {
    if r1 <= 0.0 || r2 <= 0.0 {
        return Err(ProjectError::InvalidOrbit("invalid transfer radii"));
    }

    let v1 = (mu / r1).sqrt();
    let v_transfer1 = (mu * (2.0 / r1 - 2.0 / (r1 + r2))).sqrt();
    let dv1 = v_transfer1 - v1;

    Ok(prograde_hat * dv1)
}

/// Unit tangential direction (prograde in the target plane) at `position`.
fn in_plane_prograde_hat(
    central: &CentralBody,
    orbit_type: OrbitType,
    params: OrbitParams,
    position: Vec3,
) -> Result<Vec3, ProjectError> {
    let plane_normal = orbit_plane_normal(orbit_type, params).normalized();
    let radial = position.normalized();
    let mut tang = plane_normal.cross(radial).normalized();
    if tang.length_squared() <= f64::EPSILON {
        tang = central
            .prime_meridian_direction()
            .cross(plane_normal)
            .normalized();
    }
    let spin = central.spin_axis_normalized();
    let spin_aligned = spin.cross(position);
    if spin_aligned.length_squared() > f64::EPSILON && tang.dot(spin_aligned) < 0.0 {
        tang = -tang;
    }
    if is_retrograde_orbit(orbit_type) {
        tang = -tang;
    }
    Ok(tang)
}

/// Projects a position onto the orbital plane with the given unit normal, preserving radius.
pub fn project_position_onto_orbit_plane(position: Vec3, plane_normal: Vec3) -> Vec3 {
    let radius = position.length();
    if radius <= f64::EPSILON {
        return position;
    }
    let normal = plane_normal.normalized();
    let projected = position - normal * position.dot(normal);
    if projected.length_squared() <= f64::EPSILON {
        return position;
    }
    projected.normalized() * radius
}

/// Whether velocity is prograde relative to the central body spin axis at this position.
pub fn is_prograde_velocity(spin_axis: Vec3, position: Vec3, velocity: Vec3) -> bool {
    let tangential = spin_axis.cross(position);
    tangential.length_squared() > f64::EPSILON && tangential.dot(velocity) > 0.0
}

/// Specific orbital energy of a circular target orbit at the given radius.
pub fn target_circular_specific_energy(
    central: &CentralBody,
    mu: f64,
    orbit_type: OrbitType,
    target_radius: f64,
) -> f64 {
    match orbit_type {
        OrbitType::Geostationary => {
            let speed = central.angular_rate_rad_s() * target_radius;
            0.5 * speed * speed - mu / target_radius
        }
        _ => -mu / (2.0 * target_radius),
    }
}

/// Unit direction for a burn that moves toward the target orbit; multiply by force magnitude.
pub fn thrust_direction_to_orbit(
    central: &CentralBody,
    scale: SimulationScale,
    position: Vec3,
    velocity: Vec3,
    orbit_type: OrbitType,
    params: OrbitParams,
) -> Result<Vec3, ProjectError> {
    let delta_v =
        required_delta_v_to_orbit(central, scale, position, velocity, orbit_type, params)?;
    if delta_v.length_squared() <= f64::EPSILON {
        return Err(ProjectError::InvalidVector(
            "already on target orbit within tolerance",
        ));
    }
    Ok(delta_v.normalized())
}

/// Orbital period for a circular orbit at the given radius.
pub fn orbital_period(mu: f64, radius: f64) -> Result<f64, ProjectError> {
    if radius <= 0.0 {
        return Err(ProjectError::InvalidOrbit("radius must be positive"));
    }
    Ok(std::f64::consts::TAU * (radius * radius * radius / mu).sqrt())
}

/// Specific orbital energy for a given state.
pub fn specific_orbital_energy(mu: f64, position: Vec3, velocity: Vec3) -> f64 {
    0.5 * velocity.length_squared() - mu / position.length()
}

#[cfg(test)]
mod tests {
    use super::{
        LOW_EARTH_INCLINATION_RAD, MOLNIYA_APOGEE_ALTITUDE_R, MOLNIYA_PERIGEE_ALTITUDE_R,
        OrbitParams, OrbitType, build_orbit_params_from_ui, geostationary_radius,
        initial_state_for_orbit, orbit_ui_defaults, orbit_uses_computed_altitude,
        orbit_uses_elliptical_params, orbit_uses_inclination_param, orbital_period,
    };
    use crate::central_body::CentralBody;
    use crate::units::SimulationScale;

    #[test]
    fn geostationary_orbit_has_matching_period() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let mu = central.mu(scale);
        let radius = geostationary_radius(mu, central.angular_rate_rad_s()).unwrap();
        let period = orbital_period(mu, radius).unwrap();
        assert!((period - central.rotation_period_s).abs() < 1.0);
    }

    #[test]
    fn earth_geostationary_altitude_is_realistic() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let altitude = central.geostationary_altitude_earth_radii(scale).unwrap();
        assert!((altitude - 5.63).abs() < 0.1);
    }

    #[test]
    fn geostationary_speed_matches_spin_rate() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let state = initial_state_for_orbit(
            &central,
            scale,
            OrbitType::Geostationary,
            OrbitParams::geostationary(0.0),
        )
        .unwrap();
        let radius = state.position.length();
        let speed = state.velocity.length();
        assert!((speed - central.angular_rate_rad_s() * radius).abs() < 1e-9);
    }

    #[test]
    fn equatorial_orbit_lies_in_xz_plane() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let state = initial_state_for_orbit(
            &central,
            scale,
            OrbitType::CircularEquatorial,
            OrbitParams::circular(0.1),
        )
        .unwrap();
        assert!(state.position.y.abs() < 1e-9);
        assert!(state.velocity.y.abs() < 1e-9);
    }

    #[test]
    fn polar_orbit_passes_through_poles() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let state = initial_state_for_orbit(
            &central,
            scale,
            OrbitType::CircularPolar,
            OrbitParams::circular(0.1),
        )
        .unwrap();
        let axis = central.spin_axis_normalized();
        let angular_momentum = state.position.cross(state.velocity);
        assert!(angular_momentum.dot(axis).abs() < 1e-9);
    }

    #[test]
    fn circular_equatorial_is_prograde() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let state = initial_state_for_orbit(
            &central,
            scale,
            OrbitType::CircularEquatorial,
            OrbitParams::circular(0.1),
        )
        .unwrap();
        let axis = central.spin_axis_normalized();
        let angular = axis.dot(state.position.cross(state.velocity));
        assert!(angular > 0.0);
    }

    #[test]
    fn low_circular_low_circular_helper_has_iss_inclination() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let state = initial_state_for_orbit(
            &central,
            scale,
            OrbitType::LowCircular,
            OrbitParams::low_circular(0.1),
        )
        .unwrap();
        let axis = central.spin_axis_normalized();
        let normal = state.position.cross(state.velocity).normalized();
        let inclination = normal.dot(axis).acos();
        assert!((inclination - LOW_EARTH_INCLINATION_RAD).abs() < 0.05);
    }

    #[test]
    fn low_circular_zero_inclination_is_equatorial() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let state = initial_state_for_orbit(
            &central,
            scale,
            OrbitType::LowCircular,
            OrbitParams::circular_inclined(0.1, 0.0),
        )
        .unwrap();
        assert!(state.position.y.abs() < 1e-9);
        assert!(state.velocity.y.abs() < 1e-9);
    }

    #[test]
    fn low_circular_iss_inclination_is_not_equatorial() {
        let mut sim = crate::simulation::Simulation::earth_like(86_400.0).unwrap();
        let id = sim
            .create_body_in_orbit(
                OrbitType::LowCircular,
                OrbitParams::circular_inclined(0.1, LOW_EARTH_INCLINATION_RAD),
                1.0,
            )
            .unwrap();
        let mut max_y = 0.0_f64;
        for _ in 0..500 {
            let pos = sim.position(id).unwrap();
            max_y = max_y.max(pos.y.abs());
            sim.step(100.0).unwrap();
        }
        assert!(max_y > 0.5);
    }

    #[test]
    fn ecliptic_orbit_lies_in_sun_plane() {
        let obliquity = 0.41;
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let mut params = OrbitParams::circular(0.1);
        params.obliquity_rad = obliquity;
        let state =
            initial_state_for_orbit(&central, scale, OrbitType::EclipticPrograde, params).unwrap();
        let normal = crate::star::ecliptic_plane_normal(obliquity);
        assert!(state.position.normalized().dot(normal).abs() < 1e-6);
        assert!(state.velocity.normalized().dot(normal).abs() < 1e-6);
    }

    #[test]
    fn orbit_ui_metadata_matches_orbit_families() {
        assert!(orbit_uses_elliptical_params(OrbitType::Molniya));
        assert!(orbit_uses_inclination_param(OrbitType::LowCircular));
        assert!(orbit_uses_computed_altitude(OrbitType::Geostationary));
        assert!(!orbit_uses_elliptical_params(OrbitType::CircularEquatorial));
    }

    #[test]
    fn build_orbit_params_from_ui_matches_low_circular() {
        let params = build_orbit_params_from_ui(OrbitType::LowCircular, 0.1, 0.0, 0.0, 0.5);
        assert!((params.inclination_rad - 0.5).abs() < 1e-12);
        assert!((params.altitude_earth_radii - 0.1).abs() < 1e-12);
    }

    #[test]
    fn molniya_ui_defaults_use_library_constants() {
        let defaults = orbit_ui_defaults(OrbitType::Molniya);
        assert!((defaults.perigee_altitude_earth_radii - MOLNIYA_PERIGEE_ALTITUDE_R).abs() < 1e-12);
        assert!((defaults.apogee_altitude_earth_radii - MOLNIYA_APOGEE_ALTITUDE_R).abs() < 1e-12);
    }
}
