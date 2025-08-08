//! Minimal remote entity inspector that connects to a target application
//!
//! This inspector provides real-time viewing of entities and components in remote Bevy apps.
//! Features include live component value updates, interactive text selection, copy/paste support,
//! virtual scrolling for performance, and automatic connection retry with robust error handling.
//!
//! ## Features
//! - **Real-time Updates**: Component values update live as they change in the target app
//! - **Interactive UI**: Click to select entities, text selection with copy/paste support
//! - **Connection Resilience**: Auto-retry logic handles connection failures gracefully
//! - **Performance**: Virtual scrolling efficiently handles large numbers of entities
//! - **All Components**: Automatically discovers and displays all component types
//!
//! ## Usage
//! 1. Start a target app with bevy_remote: 
//!    ```
//!    cargo run --example remote_inspector_target
//!    ```
//! 2. Run this inspector:
//!    ```
//!    cargo run --example entity_inspector_minimal --features="bevy_dev_tools"
//!    ```
//!
//! The inspector automatically connects to localhost:15702 and begins displaying entities.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the remote inspector plugin
        .add_plugins(bevy::dev_tools::inspector::InspectorPlugin)
        .run();
}
