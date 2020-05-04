mod diagnostic;
pub mod diagnostics;
mod frame_time_diagnostics_plugin;
pub use diagnostic::*;
pub use frame_time_diagnostics_plugin::FrameTimeDiagnosticsPlugin;

use bevy_app::{AppBuilder, AppPlugin};
use diagnostics::{print_diagnostics_system, PrintDiagnosticsState};
use legion::prelude::IntoSystem;
use std::time::Duration;

pub struct PrintDiagnostics {
    pub wait_duration: Duration,
    pub filter: Option<Vec<DiagnosticId>>,
}

pub struct DiagnosticsPlugin {
    pub print_diagnostics: Option<PrintDiagnostics>,
}

impl Default for DiagnosticsPlugin {
    fn default() -> Self {
        DiagnosticsPlugin {
            print_diagnostics: Some(PrintDiagnostics {
                wait_duration: Duration::from_secs_f64(1.0),
                filter: None,
            }),
        }
    }
}

impl AppPlugin for DiagnosticsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Diagnostics>();
        if let Some(ref print_diagnostics) = self.print_diagnostics {
            app.add_resource(PrintDiagnosticsState::new(print_diagnostics.wait_duration))
                .add_system(print_diagnostics_system.system());
        }
    }
}
