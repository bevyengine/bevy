//! An application that runs with default plugins and displays an empty
//! window, but without an actual renderer.
//! This can be very useful for integration tests or CI.
//!
//! See also the `headless` example which does not display a window.

use bevy::{
    prelude::*,
    render::{settings::GpuSettings, RenderPlugin},
};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(RenderPlugin {
                render_creation: GpuSettings {
                    backends: None,
                    ..default()
                }
                .into(),
            }),
        )
        .run();
}
