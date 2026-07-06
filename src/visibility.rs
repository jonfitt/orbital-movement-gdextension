//! Visible surface area from an orbital altitude.

use crate::math::Vec3;
use crate::surface_geometry::{SurfaceMesh, surface_vertex};

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

/// Builds a tessellated spherical-cap mesh for the visible area from an observer.
pub fn visibility_cap_mesh(
    observer_direction: Vec3,
    surface_radius: f64,
    horizon_half_angle: f64,
    reference_up: Vec3,
    ring_count: usize,
    segment_count: usize,
) -> SurfaceMesh {
    if observer_direction.length_squared() <= f64::EPSILON || horizon_half_angle <= 0.0 {
        return SurfaceMesh::default();
    }

    let ring_count = ring_count.max(1);
    let segment_count = segment_count.max(3);
    let center_dir = observer_direction.normalized();
    let (tangent, bitangent) = tangent_frame(center_dir, reference_up);
    let rho = horizon_half_angle.min(std::f64::consts::FRAC_PI_2 * 0.98);

    let mut mesh = SurfaceMesh {
        vertices: Vec::with_capacity(1 + ring_count * segment_count),
        normals: Vec::with_capacity(1 + ring_count * segment_count),
        indices: Vec::new(),
    };

    mesh.vertices.push(surface_vertex(center_dir, surface_radius));
    mesh.normals.push(center_dir);

    for ring in 1..=ring_count {
        let theta = rho * ring as f64 / ring_count as f64;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();
        for seg in 0..segment_count {
            let phi = std::f64::consts::TAU * seg as f64 / segment_count as f64;
            let direction = (center_dir * cos_theta
                + tangent * (sin_theta * phi.cos())
                + bitangent * (sin_theta * phi.sin()))
            .normalized();
            mesh.vertices.push(surface_vertex(direction, surface_radius));
            mesh.normals.push(direction);
        }
    }

    for seg in 0..segment_count {
        let next_seg = (seg + 1) % segment_count;
        mesh.indices.extend_from_slice(&[0, (1 + seg) as u32, (1 + next_seg) as u32]);
    }

    for ring in 1..ring_count {
        let ring_base = 1 + (ring - 1) * segment_count;
        let next_ring_base = 1 + ring * segment_count;
        for seg in 0..segment_count {
            let next_seg = (seg + 1) % segment_count;
            let i0 = ring_base + seg;
            let i1 = ring_base + next_seg;
            let i2 = next_ring_base + seg;
            let i3 = next_ring_base + next_seg;
            mesh.indices
                .extend_from_slice(&[i0 as u32, i2 as u32, i1 as u32, i1 as u32, i2 as u32, i3 as u32]);
        }
    }

    mesh
}

fn tangent_frame(center_dir: Vec3, reference_up: Vec3) -> (Vec3, Vec3) {
    let mut tangent = center_dir.cross(reference_up);
    if tangent.length_squared() < 1e-4 {
        tangent = center_dir.cross(Vec3::X);
    }
    tangent = tangent.normalized();
    let bitangent = center_dir.cross(tangent).normalized();
    (tangent, bitangent)
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
