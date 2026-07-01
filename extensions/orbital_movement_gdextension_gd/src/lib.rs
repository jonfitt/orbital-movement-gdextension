//! Godot 4 GDExtension for [`orbital_movement_gdextension`].

use orbital_movement_gdextension::greet;
use godot::prelude::*;

struct OrbitalMovementGdextensionExtension;

#[gdextension]
unsafe impl ExtensionLibrary for OrbitalMovementGdextensionExtension {}

/// Example Godot class that delegates to the core library.
#[derive(GodotClass)]
#[class(base = RefCounted)]
struct OrbitalMovementGdextensionApi {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for OrbitalMovementGdextensionApi {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base }
    }
}

#[godot_api]
impl OrbitalMovementGdextensionApi {
    /// Return a greeting from the Rust core library.
    #[func]
    fn greet(&self, name: String) -> String {
        match greet(name.as_str()) {
            Ok(message) => message,
            Err(error) => format!("error: {error}"),
        }
    }
}
