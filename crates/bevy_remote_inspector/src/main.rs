//! Out-of-process Bevy Entity Inspector
//!
//! A standalone Bevy application that connects to other Bevy applications via bevy_remote
//! to provide live entity and component inspection capabilities.

use bevy::prelude::*;

mod http_client;
mod ui;
mod inspector;

use inspector::InspectorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Remote Inspector".to_string(),
                resolution: (1200.0, 800.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(InspectorPlugin)
        .run();
}