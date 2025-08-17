use super::{Diagnostic, DiagnosticPath, DiagnosticsStore};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_platform::collections::HashSet;
use bevy_time::{Real, Time, Timer, TimerMode};
use core::time::Duration;
use log::{debug, info};

/// An App Plugin that logs diagnostics to the console.
///
/// Diagnostics are collected by plugins such as
/// [`FrameTimeDiagnosticsPlugin`](crate::FrameTimeDiagnosticsPlugin)
/// or can be provided by the user.
///
/// When no diagnostics are provided, this plugin does nothing.
pub struct LogDiagnosticsPlugin {
    /// If `true` then the `Debug` representation of each `Diagnostic` is logged.
    /// If `false` then a (smoothed) current value and historical average are logged.
    ///
    /// Defaults to `false`.
    pub debug: bool,
    /// Time to wait between logging diagnostics and logging them again.
    pub wait_duration: Duration,
    /// If `Some` then only these diagnostics are logged.
    pub filter: Option<HashSet<DiagnosticPath>>,
}

/// State used by the [`LogDiagnosticsPlugin`]
#[derive(Resource)]
pub struct LogDiagnosticsState {
    timer: Timer,
    filter: Option<HashSet<DiagnosticPath>>,
}

impl LogDiagnosticsState {
    /// Sets a new duration for the log timer
    pub fn set_timer_duration(&mut self, duration: Duration) {
        self.timer.set_duration(duration);
        self.timer.set_elapsed(Duration::ZERO);
    }

    /// Add a filter to the log state, returning `true` if the [`DiagnosticPath`]
    /// was not present
    pub fn add_filter(&mut self, diagnostic_path: DiagnosticPath) -> bool {
        if let Some(filter) = &mut self.filter {
            filter.insert(diagnostic_path)
        } else {
            self.filter = Some(HashSet::from_iter([diagnostic_path]));
            true
        }
    }

    /// Extends the filter of the log state with multiple [`DiagnosticPaths`](DiagnosticPath)
    pub fn extend_filter(&mut self, iter: impl IntoIterator<Item = DiagnosticPath>) {
        if let Some(filter) = &mut self.filter {
            filter.extend(iter);
        } else {
            self.filter = Some(HashSet::from_iter(iter));
        }
    }

    /// Removes a filter from the log state, returning `true` if it was present
    pub fn remove_filter(&mut self, diagnostic_path: &DiagnosticPath) -> bool {
        if let Some(filter) = &mut self.filter {
            filter.remove(diagnostic_path)
        } else {
            false
        }
    }

    /// Clears the filters of the log state
    pub fn clear_filter(&mut self) {
        if let Some(filter) = &mut self.filter {
            filter.clear();
        }
    }

    /// Enables filtering with empty filters
    pub fn enable_filtering(&mut self) {
        self.filter = Some(HashSet::new());
    }

    /// Disables filtering
    pub fn disable_filtering(&mut self) {
        self.filter = None;
    }
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
    /// Filter logging to only the paths in `filter`.
    pub fn filtered(filter: HashSet<DiagnosticPath>) -> Self {
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
            for path in filter.iter() {
                if let Some(diagnostic) = diagnostics.get(path)
                    && diagnostic.is_enabled
                {
                    callback(diagnostic);
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
                target: "bevy_diagnostic",
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
                target: "bevy_diagnostic",
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
        if state.timer.tick(time.delta()).is_finished() {
            Self::log_diagnostics(&state, &diagnostics);
        }
    }

    fn log_diagnostics_debug_system(
        mut state: ResMut<LogDiagnosticsState>,
        time: Res<Time<Real>>,
        diagnostics: Res<DiagnosticsStore>,
    ) {
        if state.timer.tick(time.delta()).is_finished() {
            Self::for_each_diagnostic(&state, &diagnostics, |diagnostic| {
                debug!("{diagnostic:#?}\n");
            });
        }
    }
}
