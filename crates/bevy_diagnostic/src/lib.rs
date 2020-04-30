mod diagnostic;
pub mod diagnostics;
pub use diagnostic::*;

use bevy_app::{AppBuilder, AppPlugin};
use diagnostics::{
    frame_time_diagnostic_system, print_diagnostics_system, setup_frame_time_diagnostic_system,
    PrintDiagnosticsState,
};
use legion::prelude::IntoSystem;
use std::time::Duration;

pub struct DiagnosticsPlugin {
    pub print_wait_duration: Duration,
    pub print_diagnostics: bool,
    pub add_defaults: bool,
}

impl Default for DiagnosticsPlugin {
    fn default() -> Self {
        DiagnosticsPlugin {
            print_wait_duration: Duration::from_secs_f64(1.0),
            print_diagnostics: false,
            add_defaults: true,
        }
    }
}

impl AppPlugin for DiagnosticsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource_init::<Diagnostics>();
        if self.add_defaults {
            app.add_startup_system(setup_frame_time_diagnostic_system.system())
                .add_system(frame_time_diagnostic_system.system());
        }

        if self.print_diagnostics {
            app.add_resource(PrintDiagnosticsState::new(self.print_wait_duration))
                .add_system(print_diagnostics_system.system());
        }
    }
}
