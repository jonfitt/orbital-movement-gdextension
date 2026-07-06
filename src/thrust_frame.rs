//! Local orbital-frame thrust directions (prograde, normal, radial).

use crate::error::ProjectError;
use crate::math::Vec3;

/// Semantic thrust directions in the local orbital reference frame.
///
/// Frame convention (right-handed, relative to motion):
/// - **Prograde / Retrograde**: along / against instantaneous velocity.
/// - **Left / Right**: normal to the orbital plane (`left = radial × prograde`, along +h).
/// - **Up / Down**: radial outward from / inward toward the planet center.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ThrustDirection {
    /// Along velocity.
    Prograde = 1,
    /// Against velocity.
    Retrograde = 2,
    /// Left of travel (along orbital angular momentum, `r × v`).
    Left = 4,
    /// Right of travel (opposite angular momentum).
    Right = 8,
    /// Radial outward from planet center.
    Up = 16,
    /// Radial inward toward planet center.
    Down = 32,
}

impl ThrustDirection {
    /// All directions in display order.
    pub const ALL: [Self; 6] = [
        Self::Prograde,
        Self::Retrograde,
        Self::Left,
        Self::Right,
        Self::Up,
        Self::Down,
    ];

    /// Parses a single direction from a bit flag value.
    pub fn from_bit(bit: u32) -> Option<Self> {
        match bit {
            1 => Some(Self::Prograde),
            2 => Some(Self::Retrograde),
            4 => Some(Self::Left),
            8 => Some(Self::Right),
            16 => Some(Self::Up),
            32 => Some(Self::Down),
            _ => None,
        }
    }
}

/// Bit flags for combining [`ThrustDirection`] values in Godot or other hosts.
pub mod thrust_flags {
    use super::ThrustDirection;

    /// Along velocity.
    pub const PROGRADE: u32 = ThrustDirection::Prograde as u32;
    /// Against velocity.
    pub const RETROGRADE: u32 = ThrustDirection::Retrograde as u32;
    /// Left of travel.
    pub const LEFT: u32 = ThrustDirection::Left as u32;
    /// Right of travel.
    pub const RIGHT: u32 = ThrustDirection::Right as u32;
    /// Radial outward.
    pub const UP: u32 = ThrustDirection::Up as u32;
    /// Radial inward.
    pub const DOWN: u32 = ThrustDirection::Down as u32;
}

/// Unit vectors for each local thrust axis at the given state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalThrustFrame {
    /// Prograde unit vector.
    pub prograde: Vec3,
    /// Retrograde unit vector.
    pub retrograde: Vec3,
    /// Left unit vector.
    pub left: Vec3,
    /// Right unit vector.
    pub right: Vec3,
    /// Radial outward unit vector.
    pub up: Vec3,
    /// Radial inward unit vector.
    pub down: Vec3,
}

/// Builds the local thrust frame from position and velocity.
pub fn local_thrust_frame(
    position: Vec3,
    velocity: Vec3,
) -> Result<LocalThrustFrame, ProjectError> {
    let radial = position.normalized();
    if radial.length_squared() <= f64::EPSILON {
        return Err(ProjectError::InvalidVector("position must be non-zero"));
    }

    let speed_sq = velocity.length_squared();
    if speed_sq <= f64::EPSILON {
        return Err(ProjectError::InvalidVector(
            "velocity must be non-zero to define prograde",
        ));
    }

    let prograde = velocity.normalized();
    let mut right = prograde.cross(radial);
    if right.length_squared() <= f64::EPSILON {
        // Degenerate when velocity is purely radial; pick an arbitrary normal.
        right = if radial.cross(Vec3::Y).length_squared() > f64::EPSILON {
            prograde.cross(Vec3::Y)
        } else {
            prograde.cross(Vec3::X)
        };
    }
    let right = right.normalized();
    let left = -right;

    Ok(LocalThrustFrame {
        prograde,
        retrograde: -prograde,
        left,
        right,
        up: radial,
        down: -radial,
    })
}

/// Unit vector for a single thrust direction.
pub fn thrust_direction_unit(
    position: Vec3,
    velocity: Vec3,
    direction: ThrustDirection,
) -> Result<Vec3, ProjectError> {
    let frame = local_thrust_frame(position, velocity)?;
    Ok(match direction {
        ThrustDirection::Prograde => frame.prograde,
        ThrustDirection::Retrograde => frame.retrograde,
        ThrustDirection::Left => frame.left,
        ThrustDirection::Right => frame.right,
        ThrustDirection::Up => frame.up,
        ThrustDirection::Down => frame.down,
    })
}

/// Combines selected thrust directions (bit flags) into a single unit vector.
///
/// Opposing components cancel. Returns an error if no direction is selected or
/// the combined vector vanishes.
pub fn thrust_direction_from_flags(
    position: Vec3,
    velocity: Vec3,
    flags: u32,
) -> Result<Vec3, ProjectError> {
    if flags == 0 {
        return Err(ProjectError::InvalidVector(
            "at least one thrust direction flag is required",
        ));
    }

    let frame = local_thrust_frame(position, velocity)?;
    let mut combined = Vec3::ZERO;

    for direction in ThrustDirection::ALL {
        if flags & (direction as u32) != 0 {
            combined += match direction {
                ThrustDirection::Prograde => frame.prograde,
                ThrustDirection::Retrograde => frame.retrograde,
                ThrustDirection::Left => frame.left,
                ThrustDirection::Right => frame.right,
                ThrustDirection::Up => frame.up,
                ThrustDirection::Down => frame.down,
            };
        }
    }

    if combined.length_squared() <= f64::EPSILON {
        return Err(ProjectError::InvalidVector(
            "selected thrust directions cancel out",
        ));
    }

    Ok(combined.normalized())
}

#[cfg(test)]
mod tests {
    use super::{
        ThrustDirection, local_thrust_frame, thrust_direction_from_flags, thrust_direction_unit,
    };
    use crate::math::Vec3;

    #[test]
    fn prograde_matches_velocity_direction() {
        let pos = Vec3::new(1.1, 0.0, 0.0);
        let vel = Vec3::new(0.0, 0.0, -0.01);
        let prograde = thrust_direction_unit(pos, vel, ThrustDirection::Prograde).unwrap();
        assert!(prograde.dot(vel.normalized()) > 0.999);
    }

    #[test]
    fn combined_prograde_and_up_is_normalized() {
        let pos = Vec3::new(1.1, 0.0, 0.0);
        let vel = Vec3::new(0.0, 0.0, -0.01);
        let dir = thrust_direction_from_flags(
            pos,
            vel,
            (ThrustDirection::Prograde as u32) | (ThrustDirection::Up as u32),
        )
        .unwrap();
        assert!((dir.length() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn frame_is_orthogonal_for_transverse_velocity() {
        let pos = Vec3::new(1.2, 0.0, 0.0);
        let vel = Vec3::new(0.0, 0.0, -0.01);
        let frame = local_thrust_frame(pos, vel).unwrap();
        assert!(frame.prograde.dot(frame.right).abs() < 1e-6);
        assert!(frame.prograde.dot(frame.up).abs() < 1e-6);
        assert!(frame.right.dot(frame.up).abs() < 1e-6);
    }

    #[test]
    fn equatorial_y_up_prograde_is_in_xz_plane() {
        let pos = Vec3::new(1.1, 0.0, 0.0);
        let vel = Vec3::new(0.0, 0.0, -0.01);
        let frame = local_thrust_frame(pos, vel).unwrap();
        assert!(frame.prograde.y.abs() < 1e-9);
        assert!(frame.up.y.abs() < 1e-9);
        assert!((frame.up.x - 1.0).abs() < 1e-9);
        assert!(frame.left.y.abs() > 0.99);
    }

    #[test]
    fn polar_y_up_up_thrust_points_along_pole() {
        let pos = Vec3::new(0.0, 1.2, 0.0);
        let vel = Vec3::new(-0.01, 0.0, 0.0);
        let up = thrust_direction_unit(pos, vel, ThrustDirection::Up).unwrap();
        assert!((up.y - 1.0).abs() < 1e-9);
    }
}
