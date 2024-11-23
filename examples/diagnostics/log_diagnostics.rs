//! Shows different built-in plugins that logs diagnostics, like frames per second (FPS), to the console.

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // Adds frame time diagnostics
            FrameTimeDiagnosticsPlugin,
            // Adds a system that prints diagnostics to the console
            LogDiagnosticsPlugin::default(),
            // Any plugin can register diagnostics. Uncomment this to add an entity count diagnostics:
            // bevy::diagnostic::EntityCountDiagnosticsPlugin::default(),
            // Uncomment this to add an asset count diagnostics:
            // bevy::asset::diagnostic::AssetCountDiagnosticsPlugin::<Texture>::default(),
            // Uncomment this to add system info diagnostics:
            // bevy::diagnostic::SystemInformationDiagnosticsPlugin::default()
        ))
        .run();
}
