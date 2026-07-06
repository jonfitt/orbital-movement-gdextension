//! Ground track and visibility corridor on the planet surface.

use crate::error::ProjectError;
use crate::math::Vec3;
use crate::orbits::{orbital_period, specific_orbital_energy};
use crate::surface_geometry::{
    SurfaceMesh, SurfaceTessellationConfig, ground_track_is_closed, tessellate_spherical_corridor,
    tessellate_spherical_polyline,
};
use crate::transfer_burn::TransferBurnStatus;
use crate::visibility::horizon_half_angle;

/// Surface projection of an orbit sampled in the planet-fixed frame.
#[derive(Debug, Clone, PartialEq)]
pub struct OrbitalSurfaceTrack {
    /// Sub-satellite points on the surface (planet-fixed frame).
    pub ground_track: Vec<Vec3>,
    /// Port-side visibility boundary on the surface.
    pub visibility_port: Vec<Vec3>,
    /// Starboard-side visibility boundary on the surface.
    pub visibility_starboard: Vec<Vec3>,
    /// True when thrust or a guided transfer is reshaping the path (snapshot only).
    pub ephemeral: bool,
}

impl OrbitalSurfaceTrack {
    /// Whether the ground track nearly closes on the planet surface.
    pub fn is_closed(&self) -> bool {
        ground_track_is_closed(&self.ground_track, 0.15)
    }

    /// Tessellated sub-satellite polyline on the sphere.
    pub fn tessellate_ground_line(
        &self,
        radius: f64,
        config: &SurfaceTessellationConfig,
    ) -> Vec<Vec3> {
        tessellate_spherical_polyline(&self.ground_track, radius, self.is_closed(), config)
    }

    /// Tessellated visibility corridor mesh on the sphere.
    pub fn tessellate_corridor(
        &self,
        radius: f64,
        config: &SurfaceTessellationConfig,
    ) -> SurfaceMesh {
        tessellate_spherical_corridor(
            &self.visibility_port,
            &self.visibility_starboard,
            &self.ground_track,
            radius,
            config,
        )
    }
}

/// Projects a position onto the sphere of `surface_radius`.
pub fn project_to_surface(position: Vec3, surface_radius: f64) -> Vec3 {
    if position.length_squared() <= f64::EPSILON {
        return Vec3::ZERO;
    }
    position.normalized() * surface_radius
}

/// Parameters for [`orbital_surface_track`].
#[derive(Debug, Clone, Copy)]
pub struct OrbitalSurfaceTrackInput {
    /// Standard gravitational parameter.
    pub mu: f64,
    /// Planet surface radius.
    pub surface_radius: f64,
    /// Planet spin axis (need not be normalized).
    pub spin_axis: Vec3,
    /// Spin rate in radians per second.
    pub angular_rate_rad_s: f64,
    /// Current spin angle in radians.
    pub spin_angle_rad: f64,
    /// Body position in the inertial frame.
    pub position: Vec3,
    /// Body velocity in the inertial frame.
    pub velocity: Vec3,
    /// Whether thrust or a transfer is reshaping the path.
    pub ephemeral: bool,
    /// Maximum number of ground-track samples along one period.
    pub max_points: usize,
}

/// Builds the ground track and visibility corridor for a two-body osculating orbit.
pub fn orbital_surface_track(
    input: &OrbitalSurfaceTrackInput,
) -> Result<OrbitalSurfaceTrack, ProjectError> {
    let OrbitalSurfaceTrackInput {
        mu,
        surface_radius,
        spin_axis,
        angular_rate_rad_s,
        spin_angle_rad,
        position,
        velocity,
        ephemeral,
        max_points,
    } = *input;
    let max_points = max_points.clamp(8, 2048);
    let radius = position.length();
    if radius <= surface_radius {
        return Err(ProjectError::InvalidOrbit(
            "position must be above the surface",
        ));
    }

    let h = position.cross(velocity);
    let h_mag = h.length();
    if h_mag <= f64::EPSILON {
        return Err(ProjectError::InvalidOrbit(
            "degenerate orbit: zero angular momentum",
        ));
    }

    let orbit_normal = h / h_mag;
    let r_hat = position / radius;
    let energy = specific_orbital_energy(mu, position, velocity);
    let eccentricity_vector = velocity.cross(h) / mu - r_hat;
    let eccentricity = eccentricity_vector.length();
    let semi_latus_rectum = h_mag * h_mag / mu;

    let periapsis_hat = if eccentricity > 1e-8 {
        eccentricity_vector / eccentricity
    } else {
        r_hat
    };
    let in_plane_perp = orbit_normal.cross(periapsis_hat).normalized();
    let true_anomaly_0 = in_plane_perp.dot(r_hat).atan2(periapsis_hat.dot(r_hat));

    let (sample_count, anomaly_span, period_s) = if eccentricity < 1.0 - 1e-8 {
        let semi_major = semi_latus_rectum / (1.0 - eccentricity * eccentricity);
        let period = orbital_period(mu, semi_major)?;
        (max_points, std::f64::consts::TAU, period)
    } else if energy >= 0.0 {
        // Hyperbolic / escape: sample a limited arc around the current position.
        (max_points.min(256), std::f64::consts::FRAC_PI_2, 0.0)
    } else {
        let semi_major = semi_latus_rectum / (1.0 - eccentricity * eccentricity);
        let period = orbital_period(mu, semi_major)?;
        (max_points, std::f64::consts::TAU, period)
    };

    let mut ground_track = Vec::with_capacity(sample_count);
    let mut visibility_port = Vec::with_capacity(sample_count);
    let mut visibility_starboard = Vec::with_capacity(sample_count);

    for index in 0..sample_count {
        let fraction = index as f64 / sample_count as f64;
        let true_anomaly = true_anomaly_0 + fraction * anomaly_span;
        let cos_nu = true_anomaly.cos();
        let orbit_radius = semi_latus_rectum / (1.0 + eccentricity * cos_nu);
        if orbit_radius <= surface_radius {
            continue;
        }

        let inertial = (periapsis_hat * cos_nu + in_plane_perp * true_anomaly.sin()) * orbit_radius;
        let time_along_orbit = period_s * fraction;
        let spin_at_sample = spin_angle_rad + angular_rate_rad_s * time_along_orbit;
        let planet_fixed = inertial.rotate_about_axis(spin_axis.normalized(), -spin_at_sample);

        let subsatellite = project_to_surface(planet_fixed, surface_radius);
        ground_track.push(subsatellite);

        let observer_radius = inertial.length();
        let rho = horizon_half_angle(observer_radius, surface_radius);
        if rho <= f64::EPSILON {
            visibility_port.push(subsatellite);
            visibility_starboard.push(subsatellite);
            continue;
        }

        let along_track = if index + 1 < sample_count {
            let next_fraction = (index + 1) as f64 / sample_count as f64;
            let next_nu = true_anomaly_0 + next_fraction * anomaly_span;
            let next_cos = next_nu.cos();
            let next_radius = semi_latus_rectum / (1.0 + eccentricity * next_cos);
            if next_radius <= surface_radius {
                Vec3::ZERO
            } else {
                let next_inertial =
                    (periapsis_hat * next_cos + in_plane_perp * next_nu.sin()) * next_radius;
                let next_time = period_s * next_fraction;
                let next_spin = spin_angle_rad + angular_rate_rad_s * next_time;
                let next_subsatellite = project_to_surface(
                    next_inertial.rotate_about_axis(spin_axis.normalized(), -next_spin),
                    surface_radius,
                );
                next_subsatellite - subsatellite
            }
        } else if index > 0 {
            subsatellite - ground_track[index - 1]
        } else if sample_count > 1 {
            let next_fraction = 1.0 / sample_count as f64;
            let next_nu = true_anomaly_0 + next_fraction * anomaly_span;
            let next_cos = next_nu.cos();
            let next_radius = semi_latus_rectum / (1.0 + eccentricity * next_cos);
            if next_radius <= surface_radius {
                Vec3::ZERO
            } else {
                let next_inertial =
                    (periapsis_hat * next_cos + in_plane_perp * next_nu.sin()) * next_radius;
                let next_spin = spin_angle_rad + angular_rate_rad_s * period_s * next_fraction;
                let next_subsatellite = project_to_surface(
                    next_inertial.rotate_about_axis(spin_axis.normalized(), -next_spin),
                    surface_radius,
                );
                next_subsatellite - subsatellite
            }
        } else {
            Vec3::ZERO
        };

        let subsat_hat = subsatellite.normalized();
        let along_hat = along_track.reject_from(subsat_hat);
        let (port_hat, starboard_hat) = if along_hat.length_squared() > f64::EPSILON {
            let along_hat = along_hat.normalized();
            let cross_hat = subsat_hat.cross(along_hat).normalized();
            // Match visible-cap geometry: horizon boundary at ±rho in the cross-track direction.
            let cos_rho = rho.cos();
            let sin_rho = rho.sin();
            (
                (subsat_hat * cos_rho + cross_hat * sin_rho).normalized(),
                (subsat_hat * cos_rho - cross_hat * sin_rho).normalized(),
            )
        } else {
            (subsat_hat, subsat_hat)
        };
        visibility_port.push(port_hat * surface_radius);
        visibility_starboard.push(starboard_hat * surface_radius);
    }

    if ground_track.is_empty() {
        return Err(ProjectError::InvalidOrbit(
            "orbit does not pass above the surface",
        ));
    }

    Ok(OrbitalSurfaceTrack {
        ground_track,
        visibility_port,
        visibility_starboard,
        ephemeral,
    })
}

/// Whether the body's trajectory is actively being changed by thrust.
pub fn is_ephemeral_trajectory(thrust: Vec3, transfer_status: TransferBurnStatus) -> bool {
    thrust.length_squared() > f64::EPSILON || transfer_status == TransferBurnStatus::Burning
}

#[cfg(test)]
mod tests {
    use super::{OrbitalSurfaceTrackInput, orbital_surface_track, project_to_surface};
    use crate::central_body::CentralBody;
    use crate::orbits::{OrbitParams, OrbitType, initial_state_for_orbit};
    use crate::transfer_burn::TransferBurnStatus;
    use crate::units::SimulationScale;

    #[test]
    fn project_to_surface_preserves_direction() {
        let point = project_to_surface(crate::math::Vec3::new(1.0, 2.0, 3.0), 1.0);
        assert!((point.length() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn equatorial_orbit_ground_track_is_closed_loop() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let state = initial_state_for_orbit(
            &central,
            scale,
            OrbitType::CircularEquatorial,
            OrbitParams::circular(0.1),
        )
        .unwrap();
        let track = orbital_surface_track(&OrbitalSurfaceTrackInput {
            mu: central.mu(scale),
            surface_radius: central.surface_radius(),
            spin_axis: central.spin_axis_normalized(),
            angular_rate_rad_s: 0.0,
            spin_angle_rad: 0.0,
            position: state.position,
            velocity: state.velocity,
            ephemeral: false,
            max_points: 180,
        })
        .unwrap();
        assert!(track.ground_track.len() >= 100);
        assert_eq!(track.ground_track.len(), track.visibility_port.len());
        for point in &track.ground_track {
            assert!(
                point.y.abs() < 0.05,
                "equatorial track should stay near the equator"
            );
        }
        let first = track.ground_track[0];
        let last = track.ground_track[track.ground_track.len() - 1];
        assert!(
            (first - last).length() < 0.1,
            "non-rotating equatorial orbit should nearly close over one period"
        );
    }

    #[test]
    fn visibility_corridor_width_matches_horizon_angle() {
        let central = CentralBody::default();
        let scale = SimulationScale::earth_radii();
        let surface_radius = central.surface_radius();
        let state = initial_state_for_orbit(
            &central,
            scale,
            OrbitType::LowCircular,
            OrbitParams::circular(0.05),
        )
        .unwrap();
        let track = orbital_surface_track(&OrbitalSurfaceTrackInput {
            mu: central.mu(scale),
            surface_radius,
            spin_axis: central.spin_axis_normalized(),
            angular_rate_rad_s: 0.0,
            spin_angle_rad: 0.0,
            position: state.position,
            velocity: state.velocity,
            ephemeral: false,
            max_points: 64,
        })
        .unwrap();
        let observer_radius = state.position.length();
        let rho = crate::visibility::horizon_half_angle(observer_radius, surface_radius);
        let center_hat = track.ground_track[0].normalized();
        let port_hat = track.visibility_port[0].normalized();
        let starboard_hat = track.visibility_starboard[0].normalized();
        let port_angle = center_hat.dot(port_hat).clamp(-1.0, 1.0).acos();
        let starboard_angle = center_hat.dot(starboard_hat).clamp(-1.0, 1.0).acos();
        let separation = port_hat.dot(starboard_hat).clamp(-1.0, 1.0).acos();
        assert!((port_angle - rho).abs() < 1e-6);
        assert!((starboard_angle - rho).abs() < 1e-6);
        assert!((separation - 2.0 * rho).abs() < 1e-6);
    }

    #[test]
    fn ephemeral_flag_follows_thrust_state() {
        use super::is_ephemeral_trajectory;
        assert!(!is_ephemeral_trajectory(
            crate::math::Vec3::ZERO,
            TransferBurnStatus::Idle
        ));
        assert!(is_ephemeral_trajectory(
            crate::math::Vec3::X,
            TransferBurnStatus::Idle
        ));
        assert!(is_ephemeral_trajectory(
            crate::math::Vec3::ZERO,
            TransferBurnStatus::Burning
        ));
    }
}
