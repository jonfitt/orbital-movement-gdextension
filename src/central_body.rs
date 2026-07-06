//! Central gravitating body (planet) configuration.

use crate::error::ProjectError;
use crate::math::Vec3;
use crate::units::SimulationScale;

/// A large gravitating body modeled as a point mass with a spherical surface radius.
#[derive(Debug, Clone, PartialEq)]
pub struct CentralBody {
    /// Mass in Earth masses.
    pub mass_earth: f64,
    /// Surface radius in Earth radii (simulation distance units).
    pub radius_earth: f64,
    /// Normalized spin axis (north pole direction). Defaults to +Y.
    pub spin_axis: Vec3,
    /// Sidereal rotation period in simulation seconds.
    pub rotation_period_s: f64,
}

impl Default for CentralBody {
    fn default() -> Self {
        Self {
            mass_earth: 1.0,
            radius_earth: 1.0,
            spin_axis: Vec3::Y,
            rotation_period_s: 86_400.0,
        }
    }
}

impl CentralBody {
    /// Creates an Earth-like central body with the given rotation period.
    ///
    /// The spin axis is always +Y (north pole), matching the Godot Y-up world frame.
    /// Use [`StarConfig::obliquity_rad`] for axial tilt of the sun's ecliptic plane.
    pub fn earth_like(rotation_period_s: f64) -> Result<Self, ProjectError> {
        let body = Self {
            mass_earth: 1.0,
            radius_earth: 1.0,
            spin_axis: Vec3::Y,
            rotation_period_s,
        };
        body.validate()?;
        Ok(body)
    }

    /// Validates configuration values.
    pub fn validate(&self) -> Result<(), ProjectError> {
        if self.mass_earth <= 0.0 {
            return Err(ProjectError::InvalidCentralBody("mass must be positive"));
        }
        if self.radius_earth <= 0.0 {
            return Err(ProjectError::InvalidCentralBody("radius must be positive"));
        }
        if self.rotation_period_s <= 0.0 {
            return Err(ProjectError::InvalidCentralBody(
                "rotation period must be positive",
            ));
        }
        if self.spin_axis.length_squared() <= f64::EPSILON {
            return Err(ProjectError::InvalidCentralBody(
                "spin axis must be non-zero",
            ));
        }
        Ok(())
    }

    /// Normalized spin axis.
    pub fn spin_axis_normalized(&self) -> Vec3 {
        self.spin_axis.normalized()
    }

    /// Angular rotation rate in radians per simulation second.
    pub fn angular_rate_rad_s(&self) -> f64 {
        std::f64::consts::TAU / self.rotation_period_s
    }

    /// Surface radius in simulation distance units.
    pub fn surface_radius(&self) -> f64 {
        self.radius_earth
    }

    /// Gravitational parameter μ for this body and scale.
    pub fn mu(&self, scale: SimulationScale) -> f64 {
        scale.gravitational_parameter(self.mass_earth)
    }

    /// Geostationary orbital radius in simulation units (center distance).
    pub fn geostationary_orbital_radius(
        &self,
        scale: SimulationScale,
    ) -> Result<f64, ProjectError> {
        crate::orbits::geostationary_radius(self.mu(scale), self.angular_rate_rad_s())
    }

    /// Geostationary altitude above the surface in Earth radii.
    pub fn geostationary_altitude_earth_radii(
        &self,
        scale: SimulationScale,
    ) -> Result<f64, ProjectError> {
        let radius = self.geostationary_orbital_radius(scale)?;
        Ok(radius - self.surface_radius())
    }

    /// Unit vector in the equatorial plane pointing from center to the prime meridian at epoch.
    pub fn prime_meridian_direction(&self) -> Vec3 {
        let axis = self.spin_axis_normalized();
        if axis.cross(Vec3::X).length_squared() > f64::EPSILON {
            axis.cross(Vec3::X).cross(axis).normalized()
        } else {
            axis.cross(Vec3::Z).cross(axis).normalized()
        }
    }

    /// Prograde direction at a position in the equatorial plane (right-hand rule about spin axis).
    pub fn prograde_direction_at(&self, position: Vec3) -> Vec3 {
        self.spin_axis_normalized().cross(position).normalized()
    }
}
