//! Velocity Verlet integration for orbital motion.

use crate::gravity::gravitational_acceleration;
use crate::math::Vec3;

/// State used during integration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionState {
    /// Position in simulation units.
    pub position: Vec3,
    /// Velocity in simulation units per second.
    pub velocity: Vec3,
}

/// Total acceleration from gravity and thrust.
pub fn total_acceleration(position: Vec3, thrust: Vec3, mass: f64, mu: f64) -> Vec3 {
    let mut accel = gravitational_acceleration(position, mu);
    if mass > 0.0 {
        accel += thrust / mass;
    }
    accel
}

/// Advances motion by one Velocity Verlet step.
pub fn velocity_verlet_step(state: &mut MotionState, thrust: Vec3, mass: f64, mu: f64, dt: f64) {
    let accel0 = total_acceleration(state.position, thrust, mass, mu);
    state.position += state.velocity * dt + accel0 * (0.5 * dt * dt);
    let accel1 = total_acceleration(state.position, thrust, mass, mu);
    state.velocity += (accel0 + accel1) * (0.5 * dt);
}

#[cfg(test)]
mod tests {
    use super::{MotionState, velocity_verlet_step};
    use crate::math::Vec3;

    #[test]
    fn step_advances_position() {
        let mut state = MotionState {
            position: Vec3::new(2.0, 0.0, 0.0),
            velocity: Vec3::new(0.0, 1.0, 0.0),
        };
        velocity_verlet_step(&mut state, Vec3::ZERO, 1.0, 1.0, 0.1);
        assert!(state.position.y.abs() > 0.0);
    }
}
