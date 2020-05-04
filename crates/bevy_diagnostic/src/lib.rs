mod diagnostic;
mod print_diagnostics_plugin;
mod frame_time_diagnostics_plugin;
pub use diagnostic::*;
pub use diagnostic::*;
pub use frame_time_diagnostics_plugin::FrameTimeDiagnosticsPlugin;
pub use print_diagnostics_plugin::PrintDiagnosticsPlugin;

use bevy_app::{AppBuilder, AppPlugin};

pub struct PrintDiagnostics {}

#[derive(Default)]
pub struct DiagnosticsPlugin;

impl AppPlugin for DiagnosticsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Diagnostics>();
    }
}
