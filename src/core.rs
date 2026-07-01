//! Minimal example API for generated projects.

use crate::ProjectError;

/// Return a greeting string for the given name.
///
/// Replace this with your library's real entry points.
pub fn greet(name: &str) -> Result<String, ProjectError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ProjectError::EmptyName);
    }
    Ok(format!("Hello from orbital_movement_gdextension, {trimmed}!"))
}

#[cfg(test)]
mod tests {
    use super::greet;

    #[test]
    fn greet_returns_message() {
        let message = greet("world").expect("greet should succeed");
        assert!(message.contains("world"));
    }

    #[test]
    fn greet_rejects_empty_name() {
        assert!(greet("   ").is_err());
    }
}
