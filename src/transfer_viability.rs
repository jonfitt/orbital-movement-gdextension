//! Transfer viability assessment for game/UI filtering.
//!
//! Guided transfers that are physically possible but too slow for gameplay are marked
//! [`TransferAvailability::Impractical`]. Transfers that cannot be started are
//! [`TransferAvailability::Unavailable`].

use crate::central_body::CentralBody;
use crate::error::ProjectError;
use crate::math::Vec3;
use crate::orbits::{
    OrbitParams, OrbitType, circular_orbital_speed, elliptical_shape_from_params,
    is_elliptical_orbit_type, orbit_plane_change_rad, orbit_plane_normal,
    required_delta_v_to_orbit_with_mission, specific_orbital_energy, target_orbit_reference_radius,
};
use crate::units::SimulationScale;

/// Whether a transfer should be offered in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferAvailability {
    /// Guided transfer is supported and expected to complete in reasonable time.
    Available,
    /// Physically possible but likely too slow for gameplay (low thrust, large plane change, etc.).
    Impractical,
    /// Cannot start (invalid target, escape trajectory, no thrust, etc.).
    Unavailable,
}

/// Thresholds for classifying guided transfers as impractical in a game context.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransferViabilityConfig {
    /// Guided burn time above this (simulation seconds) is impractical.
    pub max_practical_burn_time_s: f64,
    /// Estimated burn steps above this are impractical.
    pub max_practical_burn_steps: u32,
    /// Initial corrective Δv above this multiple of local circular speed is impractical.
    pub max_practical_delta_v_speed_ratio: f64,
    /// Simulation time step used to estimate burn step count.
    pub estimate_dt_s: f64,
}

impl Default for TransferViabilityConfig {
    fn default() -> Self {
        Self {
            max_practical_burn_time_s: 5.0 * 86_400.0,
            max_practical_burn_steps: 2_000_000,
            max_practical_delta_v_speed_ratio: 3.0,
            estimate_dt_s: 1.0,
        }
    }
}

/// Result of assessing a single source → target transfer.
#[derive(Debug, Clone, PartialEq)]
pub struct TransferViabilityReport {
    /// Availability for UI / gameplay.
    pub availability: TransferAvailability,
    /// Human-readable reason when not [`TransferAvailability::Available`].
    pub reason: Option<String>,
    /// Corrective Δv magnitude at assessment time.
    pub initial_delta_v: f64,
    /// Thrust-limited minimum burn time: `|Δv| / (max_thrust / mass)`.
    pub theoretical_min_burn_time_s: f64,
    /// Heuristic guided burn duration (≥ theoretical minimum).
    pub estimated_guided_burn_time_s: f64,
    /// Rough step count at `estimate_dt_s`.
    pub estimated_burn_steps: u32,
    /// Plane-change angle between current and target orbit (radians).
    pub plane_change_rad: f64,
    /// Whether the transfer primarily lowers altitude.
    pub lowering_altitude: bool,
}

/// Assesses guided-transfer viability from the current state toward a target orbit.
#[allow(clippy::too_many_arguments)]
pub fn assess_transfer_viability(
    central: &CentralBody,
    scale: SimulationScale,
    position: Vec3,
    velocity: Vec3,
    max_thrust: f64,
    mass: f64,
    orbit_type: OrbitType,
    params: OrbitParams,
    config: &TransferViabilityConfig,
) -> Result<TransferViabilityReport, ProjectError> {
    let surface = central.surface_radius();
    let radius = position.length();
    if radius <= surface + 1e-6 {
        return Ok(unavailable_report(
            config,
            0.0,
            0.0,
            0.0,
            false,
            "body is on or below the surface",
        ));
    }

    if max_thrust <= 0.0 {
        return Ok(unavailable_report(
            config,
            0.0,
            0.0,
            0.0,
            false,
            "max_thrust must be positive for guided transfers",
        ));
    }

    if mass <= 0.0 {
        return Err(ProjectError::InvalidMass);
    }

    if is_elliptical_orbit_type(orbit_type)
        && let Err(e) = elliptical_shape_from_params(central, orbit_type, params)
    {
        return Ok(unavailable_report(
            config,
            0.0,
            0.0,
            0.0,
            false,
            &e.to_string(),
        ));
    }

    let mu = central.mu(scale);
    let binding_scale = mu / radius.max(surface);
    let energy = specific_orbital_energy(mu, position, velocity);
    if energy >= -1e-6 * binding_scale {
        return Ok(unavailable_report(
            config,
            0.0,
            0.0,
            0.0,
            false,
            "trajectory is not bound (escape or hyperbolic)",
        ));
    }

    let target_radius = match target_orbit_reference_radius(central, scale, orbit_type, params) {
        Ok(r) => r,
        Err(e) => {
            return Ok(unavailable_report(
                config,
                0.0,
                0.0,
                0.0,
                false,
                &e.to_string(),
            ));
        }
    };

    let lowering_altitude = target_radius + 1e-9 < radius;
    let plane_normal = orbit_plane_normal(orbit_type, params);
    let plane_change_rad = orbit_plane_change_rad(position, velocity, plane_normal);

    let remaining = required_delta_v_to_orbit_with_mission(
        central,
        scale,
        position,
        velocity,
        orbit_type,
        params,
        lowering_altitude,
    );
    let initial_delta_v = match remaining {
        Ok(dv) => dv.length(),
        Err(e) => {
            return Ok(unavailable_report(
                config,
                0.0,
                0.0,
                plane_change_rad,
                lowering_altitude,
                &e.to_string(),
            ));
        }
    };

    let accel = max_thrust / mass;
    let theoretical_min_burn_time_s = initial_delta_v / accel;
    let guidance_factor = estimate_guidance_time_factor(
        radius,
        target_radius,
        plane_change_rad,
        lowering_altitude,
        is_elliptical_orbit_type(orbit_type),
    );
    let estimated_guided_burn_time_s = theoretical_min_burn_time_s * guidance_factor;
    let estimated_burn_steps = (estimated_guided_burn_time_s / config.estimate_dt_s)
        .ceil()
        .max(1.0) as u32;

    let local_speed = circular_orbital_speed(mu, radius).unwrap_or(velocity.length().max(1e-6));
    if initial_delta_v > config.max_practical_delta_v_speed_ratio * local_speed {
        return Ok(impractical_report(
            config,
            initial_delta_v,
            theoretical_min_burn_time_s,
            estimated_guided_burn_time_s,
            plane_change_rad,
            lowering_altitude,
            estimated_burn_steps,
            "required delta-v is very large relative to orbital speed",
        ));
    }

    if estimated_guided_burn_time_s > config.max_practical_burn_time_s {
        return Ok(impractical_report(
            config,
            initial_delta_v,
            theoretical_min_burn_time_s,
            estimated_guided_burn_time_s,
            plane_change_rad,
            lowering_altitude,
            estimated_burn_steps,
            "estimated guided burn time exceeds practical gameplay limit",
        ));
    }

    if estimated_burn_steps > config.max_practical_burn_steps {
        return Ok(impractical_report(
            config,
            initial_delta_v,
            theoretical_min_burn_time_s,
            estimated_guided_burn_time_s,
            plane_change_rad,
            lowering_altitude,
            estimated_burn_steps,
            "estimated burn steps exceed practical simulation budget",
        ));
    }

    // Large plane changes at GEO-class altitude are especially slow with low thrust.
    let geo_radius = central
        .geostationary_orbital_radius(scale)
        .unwrap_or(f64::INFINITY);
    if radius >= geo_radius * 0.9
        && plane_change_rad > std::f64::consts::FRAC_PI_4
        && estimated_guided_burn_time_s > 86_400.0
    {
        return Ok(impractical_report(
            config,
            initial_delta_v,
            theoretical_min_burn_time_s,
            estimated_guided_burn_time_s,
            plane_change_rad,
            lowering_altitude,
            estimated_burn_steps,
            "large plane change at geostationary altitude is impractical with current thrust",
        ));
    }

    Ok(TransferViabilityReport {
        availability: TransferAvailability::Available,
        reason: None,
        initial_delta_v,
        theoretical_min_burn_time_s,
        estimated_guided_burn_time_s,
        estimated_burn_steps,
        plane_change_rad,
        lowering_altitude,
    })
}

fn estimate_guidance_time_factor(
    source_radius: f64,
    target_radius: f64,
    plane_change_rad: f64,
    lowering: bool,
    elliptical_target: bool,
) -> f64 {
    let mut factor = 1.0;
    if plane_change_rad > 0.05 {
        factor *= 1.0 + plane_change_rad * 12.0;
    }
    let altitude_ratio = source_radius / target_radius.max(1e-6);
    if lowering {
        factor *= 40.0;
    } else if !(0.95..=1.05).contains(&altitude_ratio) {
        factor *= 4.0;
        if altitude_ratio > 2.0 {
            factor *= 2.0;
        }
    }
    if elliptical_target {
        factor *= 2.5;
    }
    factor.clamp(1.0, 500_000.0)
}

fn unavailable_report(
    config: &TransferViabilityConfig,
    initial_delta_v: f64,
    theoretical_min_burn_time_s: f64,
    plane_change_rad: f64,
    lowering_altitude: bool,
    reason: &str,
) -> TransferViabilityReport {
    let estimated_guided_burn_time_s = theoretical_min_burn_time_s;
    let estimated_burn_steps = (estimated_guided_burn_time_s / config.estimate_dt_s)
        .ceil()
        .max(1.0) as u32;
    TransferViabilityReport {
        availability: TransferAvailability::Unavailable,
        reason: Some(reason.to_string()),
        initial_delta_v,
        theoretical_min_burn_time_s,
        estimated_guided_burn_time_s,
        estimated_burn_steps,
        plane_change_rad,
        lowering_altitude,
    }
}

#[allow(clippy::too_many_arguments)]
fn impractical_report(
    config: &TransferViabilityConfig,
    initial_delta_v: f64,
    theoretical_min_burn_time_s: f64,
    estimated_guided_burn_time_s: f64,
    plane_change_rad: f64,
    lowering_altitude: bool,
    estimated_burn_steps: u32,
    reason: &str,
) -> TransferViabilityReport {
    let _ = config;
    TransferViabilityReport {
        availability: TransferAvailability::Impractical,
        reason: Some(reason.to_string()),
        initial_delta_v,
        theoretical_min_burn_time_s,
        estimated_guided_burn_time_s,
        estimated_burn_steps,
        plane_change_rad,
        lowering_altitude,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orbits::LOW_EARTH_INCLINATION_RAD;

    #[test]
    fn leo_equatorial_to_iss_is_available() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let state = crate::orbits::initial_state_for_orbit(
            &central,
            scale,
            OrbitType::CircularEquatorial,
            OrbitParams::circular(0.1),
        )
        .unwrap();
        let report = assess_transfer_viability(
            &central,
            scale,
            state.position,
            state.velocity,
            0.001,
            1.0,
            OrbitType::LowCircular,
            OrbitParams::circular_inclined(0.1, LOW_EARTH_INCLINATION_RAD),
            &TransferViabilityConfig::default(),
        )
        .unwrap();
        assert_eq!(report.availability, TransferAvailability::Available);
    }

    #[test]
    fn escape_trajectory_is_unavailable() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let mu = central.mu(scale);
        let r = 1.1;
        let escape_speed = (2.0 * mu / r).sqrt();
        let report = assess_transfer_viability(
            &central,
            scale,
            Vec3::new(r, 0.0, 0.0),
            Vec3::new(0.0, 0.0, escape_speed),
            0.001,
            1.0,
            OrbitType::CircularEquatorial,
            OrbitParams::circular(0.1),
            &TransferViabilityConfig::default(),
        )
        .unwrap();
        assert_eq!(report.availability, TransferAvailability::Unavailable);
    }

    #[test]
    fn tiny_thrust_geo_plane_change_is_impractical() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let geo = crate::orbits::initial_state_for_orbit(
            &central,
            scale,
            OrbitType::Geostationary,
            OrbitParams::geostationary(0.0),
        )
        .unwrap();
        let config = TransferViabilityConfig {
            max_practical_burn_time_s: 86_400.0,
            ..TransferViabilityConfig::default()
        };
        let report = assess_transfer_viability(
            &central,
            scale,
            geo.position,
            geo.velocity,
            0.00001,
            1.0,
            OrbitType::CircularPolar,
            OrbitParams::circular(0.1),
            &config,
        )
        .unwrap();
        assert!(
            matches!(
                report.availability,
                TransferAvailability::Impractical | TransferAvailability::Unavailable
            ),
            "expected impractical or unavailable, got {:?}",
            report.availability
        );
    }
}
