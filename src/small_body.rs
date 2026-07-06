//! Small orbiting bodies affected only by the central mass.

use crate::math::Vec3;

/// Identifier for a small body in a simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyId(pub u32);

/// Runtime state of a small body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyState {
    /// Free flight under gravity and thrust.
    #[default]
    Flying,
    /// Contact with the central body surface; motion clamped until a new burn is applied.
    SurfaceContact,
}

/// A small object with position, velocity, mass, and thrust limits.
#[derive(Debug, Clone, PartialEq)]
pub struct SmallBody {
    id: BodyId,
    position: Vec3,
    velocity: Vec3,
    mass: f64,
    max_thrust: f64,
    state: BodyState,
    thrust: Vec3,
}

impl SmallBody {
    /// Creates a new flying body.
    pub fn new(id: BodyId, position: Vec3, velocity: Vec3, mass: f64) -> Self {
        Self {
            id,
            position,
            velocity,
            mass,
            max_thrust: 0.0,
            state: BodyState::Flying,
            thrust: Vec3::ZERO,
        }
    }

    /// Body identifier.
    pub fn id(&self) -> BodyId {
        self.id
    }

    /// Current position in simulation units (Earth radii from center).
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Current velocity in simulation units per second.
    pub fn velocity(&self) -> Vec3 {
        self.velocity
    }

    /// Mass used for thrust and transfer acceleration calculations.
    pub fn mass(&self) -> f64 {
        self.mass
    }

    /// Maximum thrust force this body may use (simulation force units).
    pub fn max_thrust(&self) -> f64 {
        self.max_thrust
    }

    /// Sets the maximum thrust force for transfers and optional manual thrust clamping.
    pub fn set_max_thrust(&mut self, max_thrust: f64) {
        self.max_thrust = max_thrust;
    }

    /// Current state.
    pub fn state(&self) -> BodyState {
        self.state
    }

    /// Accumulated thrust force for the next integration step.
    pub fn thrust(&self) -> Vec3 {
        self.thrust
    }

    /// Sets position.
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    /// Sets velocity.
    pub fn set_velocity(&mut self, velocity: Vec3) {
        self.velocity = velocity;
    }

    /// Sets state.
    pub fn set_state(&mut self, state: BodyState) {
        self.state = state;
    }

    /// Adds a force vector for the next integration step.
    pub fn add_force(&mut self, force: Vec3) {
        self.thrust += force;
    }

    /// Clears accumulated thrust.
    pub fn clear_thrust(&mut self) {
        self.thrust = Vec3::ZERO;
    }

    /// Applies an instantaneous delta-v and resumes flight.
    pub fn apply_delta_v(&mut self, delta_v: Vec3) {
        self.velocity += delta_v;
        self.state = BodyState::Flying;
    }
}
