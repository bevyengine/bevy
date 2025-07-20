//! A comprehensive Bevy inspector that connects to remote applications via bevy_remote.
//! 
//! This example demonstrates a full-featured entity inspector similar to the Flecs editor,
//! built using only Bevy UI and bevy_remote for data communication.
//!
//! To test this inspector:
//! 1. Run a Bevy application with bevy_remote enabled (e.g., `cargo run --example server --features bevy_remote`)
//! 2. Run this inspector: `cargo run --example inspector`
//! 3. The inspector will automatically connect and display entities from the remote application

use bevy::prelude::*;
use bevy_editor::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Inspector".to_string(),
                resolution: (1200.0, 800.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EditorPlugin)
        .run();
}
