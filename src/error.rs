//! Error types for orbital_movement_gdextension.

use thiserror::Error;

/// Errors returned by the core library.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProjectError {
    /// A required name or label was empty.
    #[error("name must not be empty")]
    EmptyName,
}
