mod diagnostic;
mod frame_time_diagnostics_plugin;
mod print_diagnostics_plugin;
pub use diagnostic::*;
pub use frame_time_diagnostics_plugin::FrameTimeDiagnosticsPlugin;
pub use print_diagnostics_plugin::PrintDiagnosticsPlugin;

use bevy_app::prelude::*;

/// Adds core diagnostics resources to an App.
#[derive(Default)]
pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Diagnostics>();
    }
}
