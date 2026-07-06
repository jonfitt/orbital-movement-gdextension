//! Fixed star / light source with apparent motion due to planetary rotation.

use crate::math::Vec3;

/// Unit normal of the ecliptic plane (tilted from equatorial XZ about +X by obliquity).
pub fn ecliptic_plane_normal(obliquity_rad: f64) -> Vec3 {
    Vec3::new(0.0, -obliquity_rad.cos(), obliquity_rad.sin()).normalized()
}

/// Configuration for a distant light source (star).
#[derive(Debug, Clone, PartialEq)]
pub struct StarConfig {
    /// Distance from planet center in Earth radii.
    pub distance_earth_radii: f64,
    /// Tilt of the ecliptic plane from the equatorial plane (XZ), rotated about +X.
    pub obliquity_rad: f64,
    /// Position on the ecliptic circle at simulation epoch (0 = +X, equinox direction).
    pub orbital_longitude_rad: f64,
}

impl Default for StarConfig {
    fn default() -> Self {
        Self {
            distance_earth_radii: 100.0,
            obliquity_rad: 0.0,
            orbital_longitude_rad: 0.0,
        }
    }
}

impl StarConfig {
    /// Creates a star on the ecliptic plane at the given obliquity and orbital longitude.
    pub fn new(distance_earth_radii: f64, obliquity_rad: f64, orbital_longitude_rad: f64) -> Self {
        Self {
            distance_earth_radii,
            obliquity_rad,
            orbital_longitude_rad,
        }
    }

    /// Fixed inertial position in the planet-centered Y-up frame (+Y north pole, +X prime meridian).
    pub fn inertial_position(&self, spin_axis: Vec3) -> Vec3 {
        let _ = spin_axis.normalized();
        let cos_l = self.orbital_longitude_rad.cos();
        let sin_l = self.orbital_longitude_rad.sin();
        let sin_e = self.obliquity_rad.sin();
        let cos_e = self.obliquity_rad.cos();
        Vec3::new(cos_l, sin_l * sin_e, sin_l * cos_e).normalized() * self.distance_earth_radii
    }

    /// Apparent position in the co-rotating frame fixed to the planet surface.
    ///
    /// `spin_angle_rad` is the planet rotation angle (right-hand rule about the spin axis).
    pub fn apparent_position(&self, spin_axis: Vec3, spin_angle_rad: f64) -> Vec3 {
        let axis = spin_axis.normalized();
        self.inertial_position(axis)
            .rotate_about_axis(axis, -spin_angle_rad)
    }
}

#[cfg(test)]
mod tests {
    use super::StarConfig;
    use crate::math::Vec3;

    #[test]
    fn apparent_position_returns_after_full_rotation() {
        let star = StarConfig::new(50.0, 0.2, 0.3);
        let axis = Vec3::Y;
        let start = star.apparent_position(axis, 0.0);
        let end = star.apparent_position(axis, std::f64::consts::TAU);
        assert!((start - end).length() < 1e-10);
    }

    #[test]
    fn solstice_sun_is_north_of_equator() {
        let star = StarConfig::new(100.0, 0.41, std::f64::consts::FRAC_PI_2);
        let direction = star.inertial_position(Vec3::Y).normalized();
        assert!(direction.y > 0.2);
        assert!(direction.x.abs() < 1e-9);
    }

    #[test]
    fn equinox_sun_lies_on_prime_meridian_equator() {
        let star = StarConfig::new(100.0, 0.41, 0.0);
        let direction = star.inertial_position(Vec3::Y).normalized();
        assert!((direction.x - 1.0).abs() < 1e-9);
        assert!(direction.y.abs() < 1e-9);
    }
}
