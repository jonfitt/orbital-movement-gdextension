//! Gravitational acceleration from a central point mass.

use crate::math::Vec3;

/// Computes gravitational acceleration toward the origin from a central mass with parameter μ.
///
/// `a = -μ r / |r|³`
pub fn gravitational_acceleration(position: Vec3, mu: f64) -> Vec3 {
    let r_sq = position.length_squared();
    if r_sq <= f64::EPSILON {
        return Vec3::ZERO;
    }
    let r = r_sq.sqrt();
    -position * (mu / (r_sq * r))
}

#[cfg(test)]
mod tests {
    use super::gravitational_acceleration;
    use crate::math::Vec3;

    #[test]
    fn acceleration_points_toward_origin() {
        let pos = Vec3::new(1.0, 0.0, 0.0);
        let accel = gravitational_acceleration(pos, 1.0);
        assert!(accel.x < 0.0);
        assert!(accel.y.abs() < 1e-12);
    }
}
