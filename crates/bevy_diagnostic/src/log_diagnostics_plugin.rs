use super::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_app::prelude::*;
use bevy_core::{Time, Timer};
use bevy_ecs::system::{Res, ResMut};
use bevy_log::{debug, info};
use bevy_utils::Duration;

/// An App Plugin that logs diagnostics to the console
pub struct LogDiagnosticsPlugin {
    pub debug: bool,
    pub wait_duration: Duration,
    pub filter: Option<Vec<DiagnosticId>>,
}

/// State used by the [LogDiagnosticsPlugin]
struct LogDiagnosticsState {
    timer: Timer,
    filter: Option<Vec<DiagnosticId>>,
}

impl Default for LogDiagnosticsPlugin {
    fn default() -> Self {
        LogDiagnosticsPlugin {
            debug: false,
            wait_duration: Duration::from_secs(1),
            filter: None,
        }
    }
}

impl Plugin for LogDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LogDiagnosticsState {
            timer: Timer::new(self.wait_duration, true),
            filter: self.filter.clone(),
        });

        if self.debug {
            app.add_system_to_stage(CoreStage::PostUpdate, Self::log_diagnostics_debug_system);
        } else {
            app.add_system_to_stage(CoreStage::PostUpdate, Self::log_diagnostics_system);
        }
    }
}

impl LogDiagnosticsPlugin {
    pub fn filtered(filter: Vec<DiagnosticId>) -> Self {
        LogDiagnosticsPlugin {
            filter: Some(filter),
            ..Default::default()
        }
    }

    fn log_diagnostic(diagnostic: &Diagnostic) {
        if let Some(value) = diagnostic.value() {
            if let Some(average) = diagnostic.average() {
                info!(
                    target: "bevy diagnostic",
                    "{:<name_width$}: {:>12} (avg {:>})",
                    diagnostic.name,
                    // Suffix is only used for 's' as in seconds currently,
                    // so we reserve one column for it
                    format!("{:.6}{:1}", value, diagnostic.suffix),
                    // Do not reserve one column for the suffix in the average
                    // The ) hugging the value is more aesthetically pleasing
                    format!("{:.6}{:}", average, diagnostic.suffix),
                    name_width = crate::MAX_DIAGNOSTIC_NAME_WIDTH,
                );
            } else {
                info!(
                    target: "bevy diagnostic",
                    "{:<name_width$}: {:>}",
                    diagnostic.name,
                    format!("{:.6}{:}", value, diagnostic.suffix),
                    name_width = crate::MAX_DIAGNOSTIC_NAME_WIDTH,
                );
            }
        }
    }

    fn log_diagnostics_system(
        mut state: ResMut<LogDiagnosticsState>,
        time: Res<Time>,
        diagnostics: Res<Diagnostics>,
    ) {
        if state.timer.tick(time.delta()).finished() {
            if let Some(ref filter) = state.filter {
                for diagnostic in filter.iter().map(|id| diagnostics.get(*id).unwrap()) {
                    Self::log_diagnostic(diagnostic);
                }
            } else {
                for diagnostic in diagnostics.iter() {
                    Self::log_diagnostic(diagnostic);
                }
            }
        }
    }

    fn log_diagnostics_debug_system(
        mut state: ResMut<LogDiagnosticsState>,
        time: Res<Time>,
        diagnostics: Res<Diagnostics>,
    ) {
        if state.timer.tick(time.delta()).finished() {
            if let Some(ref filter) = state.filter {
                for diagnostic in filter.iter().map(|id| diagnostics.get(*id).unwrap()) {
                    debug!("{:#?}\n", diagnostic);
                }
            } else {
                for diagnostic in diagnostics.iter() {
                    debug!("{:#?}\n", diagnostic);
                }
            }
        }
    }
}
