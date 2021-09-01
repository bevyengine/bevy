mod diagnostic;
mod entity_count_diagnostics_plugin;
mod frame_time_diagnostics_plugin;
mod log_diagnostics_plugin;
pub use diagnostic::*;
pub use entity_count_diagnostics_plugin::EntityCountDiagnosticsPlugin;
pub use frame_time_diagnostics_plugin::FrameTimeDiagnosticsPlugin;
pub use log_diagnostics_plugin::LogDiagnosticsPlugin;

use bevy_app::prelude::*;

/// Adds core diagnostics resources to an App.
#[derive(Default)]
pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Diagnostics>();
    }
}

/// The width which diagnostic names will be printed as
/// Plugin names should not be longer than this value
pub const MAX_DIAGNOSTIC_NAME_WIDTH: usize = 32;
