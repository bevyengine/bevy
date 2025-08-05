//! Minimal remote entity inspector that connects to a target application
//!
//! This inspector application connects to a running Bevy app with bevy_remote enabled
//! and provides a UI for inspecting entities and components in real-time.
//!
//! Usage:
//! 1. First run the target app: `cargo run --example remote_inspector_target --features="bevy_remote"`
//! 2. Then run this inspector: `cargo run --example entity_inspector_minimal --features="bevy_dev_tools"`

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the remote inspector plugin
        .add_plugins(bevy::dev_tools::inspector::InspectorPlugin)
        .run();
}
