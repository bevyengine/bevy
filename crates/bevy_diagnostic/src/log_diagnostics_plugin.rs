use super::{Diagnostic, DiagnosticPath, DiagnosticsStore};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_time::{Real, Time, Timer, TimerMode};
use bevy_utils::tracing::{debug, info};
use bevy_utils::Duration;

/// An App Plugin that logs diagnostics to the console.
///
/// Diagnostics are collected by plugins such as
/// [`FrameTimeDiagnosticsPlugin`](crate::FrameTimeDiagnosticsPlugin)
/// or can be provided by the user.
///
/// When no diagnostics are provided, this plugin does nothing.
pub struct LogDiagnosticsPlugin {
    pub debug: bool,
    pub wait_duration: Duration,
    pub filter: Option<Vec<DiagnosticPath>>,
}

/// State used by the [`LogDiagnosticsPlugin`]
#[derive(Resource)]
struct LogDiagnosticsState {
    timer: Timer,
    filter: Option<Vec<DiagnosticPath>>,
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
    pub fn filtered(filter: Vec<DiagnosticPath>) -> Self {
        LogDiagnosticsPlugin {
            filter: Some(filter),
            ..Default::default()
        }
    }

    fn for_each_diagnostic(
        state: &LogDiagnosticsState,
        diagnostics: &DiagnosticsStore,
        mut callback: impl FnMut(&Diagnostic),
    ) {
        if let Some(filter) = &state.filter {
            for path in filter {
                if let Some(diagnostic) = diagnostics.get(path) {
                    if diagnostic.is_enabled {
                        callback(diagnostic);
                    }
                }
            }
        } else {
            for diagnostic in diagnostics.iter() {
                if diagnostic.is_enabled {
                    callback(diagnostic);
                }
            }
        }
    }

    fn log_diagnostic(path_width: usize, diagnostic: &Diagnostic) {
        let Some(value) = diagnostic.smoothed() else {
            return;
        };

        if diagnostic.get_max_history_length() > 1 {
            let Some(average) = diagnostic.average() else {
                return;
            };

            info!(
                target: "bevy diagnostic",
                // Suffix is only used for 's' or 'ms' currently,
                // so we reserve two columns for it; however,
                // Do not reserve columns for the suffix in the average
                // The ) hugging the value is more aesthetically pleasing
                "{path:<path_width$}: {value:>11.6}{suffix:2} (avg {average:>.6}{suffix:})",
                path = diagnostic.path(),
                suffix = diagnostic.suffix,
            );
        } else {
            info!(
                target: "bevy diagnostic",
                "{path:<path_width$}: {value:>.6}{suffix:}",
                path = diagnostic.path(),
                suffix = diagnostic.suffix,
            );
        }
    }

    fn log_diagnostics(state: &LogDiagnosticsState, diagnostics: &DiagnosticsStore) {
        let mut path_width = 0;
        Self::for_each_diagnostic(state, diagnostics, |diagnostic| {
            let width = diagnostic.path().as_str().len();
            path_width = path_width.max(width);
        });

        Self::for_each_diagnostic(state, diagnostics, |diagnostic| {
            Self::log_diagnostic(path_width, diagnostic);
        });
    }

    fn log_diagnostics_system(
        mut state: ResMut<LogDiagnosticsState>,
        time: Res<Time<Real>>,
        diagnostics: Res<DiagnosticsStore>,
    ) {
        if state.timer.tick(time.delta()).finished() {
            Self::log_diagnostics(&state, &diagnostics);
        }
    }

    fn log_diagnostics_debug_system(
        mut state: ResMut<LogDiagnosticsState>,
        time: Res<Time<Real>>,
        diagnostics: Res<DiagnosticsStore>,
    ) {
        if state.timer.tick(time.delta()).finished() {
            Self::for_each_diagnostic(&state, &diagnostics, |diagnostic| {
                debug!("{:#?}\n", diagnostic);
            });
        }
    }
}
