//! Minimal Bevy window that exercises the core library.

use orbital_movement_gdextension::greet;

fn main() {
    let message = greet("Bevy visualizer").unwrap_or_else(|error| format!("error: {error}"));
    println!("{message}");
    println!("<!-- TODO: Replace this stub with an interactive Bevy scene. -->");

    bevy::app::App::new()
        .add_plugins(bevy::prelude::DefaultPlugins)
        .add_systems(bevy::prelude::Startup, setup)
        .run();
}

fn setup(mut commands: bevy::prelude::Commands) {
    commands.spawn(bevy::prelude::Camera2d);
}
