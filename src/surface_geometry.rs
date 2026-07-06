//! Spherical surface geometry helpers for ground tracks and visibility meshes.

use crate::math::Vec3;

/// Triangle mesh on a sphere (vertices and normals are outward directions × radius).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SurfaceMesh {
    /// Mesh vertex positions.
    pub vertices: Vec<Vec3>,
    /// Outward unit normals at each vertex.
    pub normals: Vec<Vec3>,
    /// Triangle indices (groups of three).
    pub indices: Vec<u32>,
}

/// Controls how finely polylines and corridors are subdivided on a sphere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceTessellationConfig {
    /// Maximum angular span of a single mesh segment (radians).
    pub max_segment_angle_rad: f64,
    /// Minimum subdivision count per segment or corridor width.
    pub min_subdivisions: usize,
    /// Maximum subdivision count per segment or corridor width.
    pub max_subdivisions: usize,
}

impl Default for SurfaceTessellationConfig {
    fn default() -> Self {
        Self {
            max_segment_angle_rad: 2.5_f64.to_radians(),
            min_subdivisions: 4,
            max_subdivisions: 48,
        }
    }
}

/// Angular distance between two directions (radians).
pub fn angular_distance(from: Vec3, to: Vec3) -> f64 {
    from.normalized()
        .dot(to.normalized())
        .clamp(-1.0, 1.0)
        .acos()
}

/// Spherical linear interpolation between two unit directions.
pub fn slerp_direction(from: Vec3, to: Vec3, weight: f64) -> Vec3 {
    let from = from.normalized();
    let to = to.normalized();
    if from.dot(to) > 1.0 - 1e-12 {
        return from;
    }
    let omega = angular_distance(from, to);
    let sin_omega = omega.sin();
    if sin_omega <= f64::EPSILON {
        return from;
    }
    let w0 = ((1.0 - weight) * omega).sin() / sin_omega;
    let w1 = (weight * omega).sin() / sin_omega;
    (from * w0 + to * w1).normalized()
}

/// Subdivision count for a spherical arc of `angle_rad`.
pub fn subdivisions_for_angle(angle_rad: f64, config: &SurfaceTessellationConfig) -> usize {
    if angle_rad <= 0.0 {
        return config.min_subdivisions;
    }
    let steps = (angle_rad / config.max_segment_angle_rad).ceil() as usize;
    steps.clamp(config.min_subdivisions, config.max_subdivisions)
}

/// Whether a ground track nearly closes on the sphere.
pub fn ground_track_is_closed(ground_track: &[Vec3], closure_angle_rad: f64) -> bool {
    if ground_track.len() < 3 {
        return false;
    }
    let first = ground_track[0];
    let last = ground_track[ground_track.len() - 1];
    angular_distance(first, last) < closure_angle_rad
}

/// Projects a direction onto a sphere of `radius`.
pub fn surface_vertex(direction: Vec3, radius: f64) -> Vec3 {
    direction.normalized() * radius
}

/// Builds a polyline that follows the sphere between sample directions.
pub fn tessellate_spherical_polyline(
    points: &[Vec3],
    radius: f64,
    closed: bool,
    config: &SurfaceTessellationConfig,
) -> Vec<Vec3> {
    if points.is_empty() {
        return Vec::new();
    }
    if points.len() == 1 {
        return vec![surface_vertex(points[0], radius)];
    }

    let count = points.len();
    let segment_count = if closed { count } else { count - 1 };
    let mut vertices = Vec::new();

    for index in 0..segment_count {
        let next_index = (index + 1) % count;
        let segment_angle = angular_distance(points[index], points[next_index]);
        let steps = subdivisions_for_angle(segment_angle, config);
        let start_step = if index == 0 || closed { 0 } else { 1 };
        for step in start_step..steps {
            let weight = step as f64 / steps as f64;
            let direction = slerp_direction(points[index], points[next_index], weight);
            vertices.push(surface_vertex(direction, radius));
        }
    }

    if closed {
        if let Some(&first) = vertices.first() {
            vertices.push(first);
        }
    } else {
        vertices.push(surface_vertex(points[count - 1], radius));
    }

    vertices
}

fn build_surface_stations(
    port_points: &[Vec3],
    starboard_points: &[Vec3],
    closed: bool,
    config: &SurfaceTessellationConfig,
) -> (Vec<Vec3>, Vec<Vec3>) {
    let mut port_stations = Vec::new();
    let mut starboard_stations = Vec::new();
    let count = port_points.len().min(starboard_points.len());
    if count < 2 {
        return (port_stations, starboard_stations);
    }

    let segment_count = if closed { count } else { count - 1 };
    for index in 0..segment_count {
        let next_index = (index + 1) % count;
        let segment_angle = angular_distance(port_points[index], port_points[next_index]);
        let steps = subdivisions_for_angle(segment_angle, config);
        let start_step = if index == 0 || closed { 0 } else { 1 };
        for step in start_step..steps {
            let weight = step as f64 / steps as f64;
            port_stations.push(slerp_direction(
                port_points[index],
                port_points[next_index],
                weight,
            ));
            starboard_stations.push(slerp_direction(
                starboard_points[index],
                starboard_points[next_index],
                weight,
            ));
        }
    }

    if !closed {
        port_stations.push(port_points[count - 1].normalized());
        starboard_stations.push(starboard_points[count - 1].normalized());
    }

    (port_stations, starboard_stations)
}

fn max_cross_track_angle(port_points: &[Vec3], starboard_points: &[Vec3]) -> f64 {
    let count = port_points.len().min(starboard_points.len());
    let mut max_angle = 0.0_f64;
    for index in 0..count {
        max_angle = f64::max(
            max_angle,
            angular_distance(port_points[index], starboard_points[index]),
        );
    }
    max_angle
}

/// Tessellates a visibility corridor between port and starboard rails on the sphere.
pub fn tessellate_spherical_corridor(
    port_points: &[Vec3],
    starboard_points: &[Vec3],
    ground_track: &[Vec3],
    radius: f64,
    config: &SurfaceTessellationConfig,
) -> SurfaceMesh {
    let count = ground_track
        .len()
        .min(port_points.len())
        .min(starboard_points.len());
    if count < 2 {
        return SurfaceMesh::default();
    }

    let closed = ground_track_is_closed(ground_track, 0.15);
    let cross_subdivisions =
        subdivisions_for_angle(max_cross_track_angle(port_points, starboard_points), config);
    let (port_stations, starboard_stations) =
        build_surface_stations(port_points, starboard_points, closed, config);
    let along_count = port_stations.len();
    if along_count < 2 {
        return SurfaceMesh::default();
    }

    let row_stride = cross_subdivisions + 1;
    let mut mesh = SurfaceMesh {
        vertices: Vec::with_capacity(along_count * row_stride),
        normals: Vec::with_capacity(along_count * row_stride),
        indices: Vec::new(),
    };

    for along_index in 0..along_count {
        let port_dir = port_stations[along_index];
        let starboard_dir = starboard_stations[along_index];
        for cross_index in 0..row_stride {
            let cross_weight = cross_index as f64 / cross_subdivisions as f64;
            let direction = slerp_direction(port_dir, starboard_dir, cross_weight);
            mesh.vertices.push(surface_vertex(direction, radius));
            mesh.normals.push(direction.normalized());
        }
    }

    let along_segment_count = if closed { along_count } else { along_count - 1 };
    for along_index in 0..along_segment_count {
        let next_along = (along_index + 1) % along_count;
        let row_a = along_index * row_stride;
        let row_b = next_along * row_stride;
        for cross_index in 0..cross_subdivisions {
            let i00 = row_a + cross_index;
            let i01 = row_a + cross_index + 1;
            let i10 = row_b + cross_index;
            let i11 = row_b + cross_index + 1;
            mesh.indices.extend_from_slice(&[
                i00 as u32, i10 as u32, i01 as u32, i01 as u32, i10 as u32, i11 as u32,
            ]);
        }
    }

    mesh
}

#[cfg(test)]
mod tests {
    use super::{
        SurfaceTessellationConfig, angular_distance, ground_track_is_closed,
        tessellate_spherical_corridor, tessellate_spherical_polyline,
    };
    use crate::math::Vec3;

    #[test]
    fn slerp_endpoints_match() {
        let a = Vec3::X;
        let b = Vec3::Y;
        assert!(angular_distance(super::slerp_direction(a, b, 0.0), a) < 1e-12);
        assert!(angular_distance(super::slerp_direction(a, b, 1.0), b) < 1e-12);
    }

    #[test]
    fn polyline_subdivision_increases_with_segment_angle() {
        let config = SurfaceTessellationConfig::default();
        let short = tessellate_spherical_polyline(&[Vec3::X, Vec3::Y], 1.0, false, &config);
        let far = tessellate_spherical_polyline(&[Vec3::X, -Vec3::X], 1.0, false, &config);
        assert!(far.len() > short.len());
    }

    #[test]
    fn corridor_mesh_has_triangles() {
        let config = SurfaceTessellationConfig::default();
        let ground = vec![Vec3::X, Vec3::Y, Vec3::Z];
        let port = vec![
            Vec3::new(1.0, 0.1, 0.0),
            Vec3::new(0.0, 1.0, 0.1),
            Vec3::new(0.0, 0.1, 1.0),
        ];
        let starboard = vec![
            Vec3::new(1.0, -0.1, 0.0),
            Vec3::new(0.0, 1.0, -0.1),
            Vec3::new(0.0, -0.1, 1.0),
        ];
        let mesh = tessellate_spherical_corridor(&port, &starboard, &ground, 1.0, &config);
        assert!(!mesh.vertices.is_empty());
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.indices.len() % 3, 0);
    }

    #[test]
    fn closed_track_detection_uses_angular_distance() {
        let almost_closed = vec![Vec3::X, Vec3::Y, Vec3::Z, Vec3::new(0.99, 0.01, 0.0)];
        assert!(ground_track_is_closed(&almost_closed, 0.2));
        assert!(!ground_track_is_closed(&[Vec3::X, Vec3::Y, Vec3::Z], 0.01));
    }
}
