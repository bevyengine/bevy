//! Shows how to disable/re-enable a Diagnostic during runtime

use std::time::Duration;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    time::common_conditions::on_timer,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .add_systems(
            Update,
            toggle.run_if(on_timer(Duration::from_secs_f32(10.0))),
        )
        .run();
}

fn toggle(mut store: ResMut<DiagnosticsStore>) {
    for diag in store.iter_mut() {
        info!("toggling diagnostic {}", diag.path());
        diag.is_enabled = !diag.is_enabled;
    }
}
