//! An application that runs with default plugins, but without an actual renderer.
//! This can be very useful for integration tests or CI.

use bevy::{prelude::*, render::settings::WgpuSettings};

fn main() {
    App::new()
        .insert_resource(WgpuSettings {
            backends: None,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .run();
}
