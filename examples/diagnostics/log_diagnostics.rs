use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Adds frame time diagnostics
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // Adds a system that prints diagnostics to the console
        .add_plugin(LogDiagnosticsPlugin::default())
        // Any plugin can register diagnostics
        // Uncomment this to add some render resource diagnostics:
        // .add_plugin(bevy::wgpu::diagnostic::WgpuResourceDiagnosticsPlugin::default())
        // Uncomment this to add an entity count diagnostics:
        // .add_plugin(bevy::diagnostic::EntityCountDiagnosticsPlugin::default())
        // Uncomment this to add an asset count diagnostics:
        // .add_plugin(bevy::asset::diagnostic::AssetCountDiagnosticsPlugin::<Texture>::default())
        .run();
}
