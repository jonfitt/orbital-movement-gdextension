//! Visible surface area from an orbital altitude.

/// Horizon half-angle (radians) from an observer at radius `observer_radius`.
pub fn horizon_half_angle(observer_radius: f64, surface_radius: f64) -> f64 {
    if observer_radius <= surface_radius {
        return 0.0;
    }
    (surface_radius / observer_radius).acos()
}

/// Spherical cap area visible from altitude, in squared surface-radius units.
pub fn visible_surface_area(observer_radius: f64, surface_radius: f64) -> f64 {
    if observer_radius <= surface_radius {
        return 0.0;
    }
    let ratio = surface_radius / observer_radius;
    2.0 * std::f64::consts::PI * surface_radius * surface_radius * (1.0 - ratio)
}

#[cfg(test)]
mod tests {
    use super::visible_surface_area;

    #[test]
    fn nearly_half_sphere_at_large_altitude() {
        let area = visible_surface_area(10_000.0, 1.0);
        let half_sphere = 2.0 * std::f64::consts::PI;
        assert!((area - half_sphere).abs() < 0.01);
    }
}
