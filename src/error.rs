//! Error types for orbital_movement_gdextension.

use thiserror::Error;

/// Errors returned by the core library.
#[derive(Debug, Error, PartialEq)]
pub enum ProjectError {
    /// A required name or label was empty.
    #[error("name must not be empty")]
    EmptyName,

    /// The requested body id does not exist.
    #[error("body id {0} not found")]
    BodyNotFound(u32),

    /// Orbit parameters are invalid for the requested configuration.
    #[error("invalid orbit parameters: {0}")]
    InvalidOrbit(&'static str),

    /// A vector argument was zero or invalid.
    #[error("invalid vector: {0}")]
    InvalidVector(&'static str),

    /// The central body configuration is invalid.
    #[error("invalid central body: {0}")]
    InvalidCentralBody(&'static str),

    /// The simulation time step is invalid.
    #[error("invalid time step: {0}")]
    InvalidTimeStep(&'static str),

    /// The body mass must be positive when applying forces.
    #[error("body mass must be positive")]
    InvalidMass,
}
