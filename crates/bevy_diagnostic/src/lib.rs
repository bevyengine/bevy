#![allow(clippy::type_complexity)]

mod diagnostic;
mod entity_count_diagnostics_plugin;
mod frame_time_diagnostics_plugin;
mod log_diagnostics_plugin;
mod system_information_diagnostics_plugin;

use bevy_app::prelude::*;
pub use diagnostic::*;
pub use entity_count_diagnostics_plugin::EntityCountDiagnosticsPlugin;
pub use frame_time_diagnostics_plugin::FrameTimeDiagnosticsPlugin;
pub use log_diagnostics_plugin::LogDiagnosticsPlugin;
pub use system_information_diagnostics_plugin::SystemInformationDiagnosticsPlugin;

/// Adds core diagnostics resources to an App.
#[derive(Default)]
pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiagnosticsStore>().add_systems(
            Startup,
            system_information_diagnostics_plugin::internal::log_system_info,
        );
    }
}

/// The width which diagnostic names will be printed as
/// Plugin names should not be longer than this value
pub const MAX_DIAGNOSTIC_NAME_WIDTH: usize = 32;
