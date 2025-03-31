//! Shows different built-in plugins that logs diagnostics, like frames per second (FPS), to the console.

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((
            // The diagnostics plugins need to be added after DefaultPlugins as they use e.g. the time plugin for timestamps.
            DefaultPlugins,
            // Adds a system that prints diagnostics to the console.
            // The other diagnostics plugins can still be used without this if you want to use them in an ingame overlay for example.
            LogDiagnosticsPlugin::default(),
            // Adds frame time, FPS and frame count diagnostics.
            FrameTimeDiagnosticsPlugin::default(),
            // Adds an entity count diagnostic.
            bevy::diagnostic::EntityCountDiagnosticsPlugin::default(),
            // Adds cpu and memory usage diagnostics for systems and the entire game process.
            bevy::diagnostic::SystemInformationDiagnosticsPlugin::default(),
        ))
        .run();
}
