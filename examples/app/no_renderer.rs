//! An application that runs with default plugins and displays an empty
//! window, but without an actual renderer.
//! This can be very useful for integration tests or CI.
//!
//! See also the `headless` example which does not display a window.

use bevy::{
    gpu::{settings::WgpuSettings, GpuPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(GpuPlugin {
            wgpu_settings: WgpuSettings {
                backends: None,
                ..default()
            },
        }))
        .run();
}
