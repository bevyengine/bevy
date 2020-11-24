use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        // Adds frame time diagnostics
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // Adds a system that prints diagnostics to the console
        .add_plugin(PrintDiagnosticsPlugin::default())
        // Any plugin can register diagnostics
        // Uncomment this to add some render resource diagnostics:
        // .add_plugin(bevy::wgpu::diagnostic::WgpuResourceDiagnosticsPlugin::default())
        .run();
}
