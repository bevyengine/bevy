use super::{Diagnostic, DiagnosticId, DiagnosticsStore};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_log::{debug, info};
use bevy_time::{Time, Timer, TimerMode};
use bevy_utils::Duration;

/// An App Plugin that logs diagnostics to the console
pub struct LogDiagnosticsPlugin {
    pub debug: bool,
    pub wait_duration: Duration,
    pub filter: Option<Vec<DiagnosticId>>,
}

/// State used by the [`LogDiagnosticsPlugin`]
#[derive(Resource)]
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
            timer: Timer::new(self.wait_duration, TimerMode::Repeating),
            filter: self.filter.clone(),
        });

        if self.debug {
            app.add_systems(PostUpdate, Self::log_diagnostics_debug_system);
        } else {
            app.add_systems(PostUpdate, Self::log_diagnostics_system);
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
        if let Some(value) = diagnostic.smoothed() {
            if diagnostic.get_max_history_length() > 1 {
                if let Some(average) = diagnostic.average() {
                    info!(
                        target: "bevy diagnostic",
                        // Suffix is only used for 's' or 'ms' currently,
                        // so we reserve two columns for it; however,
                        // Do not reserve columns for the suffix in the average
                        // The ) hugging the value is more aesthetically pleasing
                        "{name:<name_width$}: {value:>11.6}{suffix:2} (avg {average:>.6}{suffix:})",
                        name = diagnostic.name,
                        suffix = diagnostic.suffix,
                        name_width = crate::MAX_DIAGNOSTIC_NAME_WIDTH,
                    );
                    return;
                }
            }
            info!(
                target: "bevy diagnostic",
                "{name:<name_width$}: {value:>.6}{suffix:}",
                name = diagnostic.name,
                suffix = diagnostic.suffix,
                name_width = crate::MAX_DIAGNOSTIC_NAME_WIDTH,
            );
        }
    }

    fn log_diagnostics_system(
        mut state: ResMut<LogDiagnosticsState>,
        time: Res<Time>,
        diagnostics: Res<DiagnosticsStore>,
    ) {
        if state.timer.tick(time.raw_delta()).finished() {
            if let Some(ref filter) = state.filter {
                for diagnostic in filter.iter().flat_map(|id| {
                    diagnostics
                        .get(*id)
                        .filter(|diagnostic| diagnostic.is_enabled)
                }) {
                    Self::log_diagnostic(diagnostic);
                }
            } else {
                for diagnostic in diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.is_enabled)
                {
                    Self::log_diagnostic(diagnostic);
                }
            }
        }
    }

    fn log_diagnostics_debug_system(
        mut state: ResMut<LogDiagnosticsState>,
        time: Res<Time>,
        diagnostics: Res<DiagnosticsStore>,
    ) {
        if state.timer.tick(time.raw_delta()).finished() {
            if let Some(ref filter) = state.filter {
                for diagnostic in filter.iter().flat_map(|id| {
                    diagnostics
                        .get(*id)
                        .filter(|diagnostic| diagnostic.is_enabled)
                }) {
                    debug!("{:#?}\n", diagnostic);
                }
            } else {
                for diagnostic in diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.is_enabled)
                {
                    debug!("{:#?}\n", diagnostic);
                }
            }
        }
    }
}
