use crate::{
    Diagnostic, DiagnosticPath, Diagnostics, FrameCount, RegisterDiagnostic,
    DEFAULT_MAX_HISTORY_LENGTH,
};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_time::{Real, Time};

/// Adds "frame time" diagnostic to an App, specifically "frame time", "fps" and "frame count"
///
/// # See also
///
/// [`LogDiagnosticsPlugin`](crate::LogDiagnosticsPlugin) to output diagnostics to the console.
pub struct FrameTimeDiagnosticsPlugin {
    /// The total number of values to keep for averaging.
    pub max_history_length: usize,
    /// The smoothing factor for the exponential moving average. Usually `2.0 / (history_length + 1.0)`.
    pub smoothing_factor: f64,
}

impl Default for FrameTimeDiagnosticsPlugin {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_HISTORY_LENGTH)
    }
}

impl FrameTimeDiagnosticsPlugin {
    /// Creates a new `FrameTimeDiagnosticsPlugin` with the specified `max_history_length` and a
    /// reasonable `smoothing_factor`.
    pub fn new(max_history_length: usize) -> Self {
        Self {
            max_history_length,
            smoothing_factor: 2.0 / (max_history_length as f64 + 1.0),
        }
    }
}

impl Plugin for FrameTimeDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.register_diagnostic(
            Diagnostic::new(Self::FRAME_TIME)
                .with_suffix("ms")
                .with_max_history_length(self.max_history_length)
                .with_smoothing_factor(self.smoothing_factor),
        )
        .register_diagnostic(
            Diagnostic::new(Self::FPS)
                .with_max_history_length(self.max_history_length)
                .with_smoothing_factor(self.smoothing_factor),
        )
        // An average frame count would be nonsensical, so we set the max history length
        // to zero and disable smoothing.
        .register_diagnostic(
            Diagnostic::new(Self::FRAME_COUNT)
                .with_smoothing_factor(0.0)
                .with_max_history_length(0),
        )
        .add_systems(Update, Self::diagnostic_system);
    }
}

impl FrameTimeDiagnosticsPlugin {
    /// Frames per second.
    pub const FPS: DiagnosticPath = DiagnosticPath::const_new("fps");

    /// Total frames since application start.
    pub const FRAME_COUNT: DiagnosticPath = DiagnosticPath::const_new("frame_count");

    /// Frame time in ms.
    pub const FRAME_TIME: DiagnosticPath = DiagnosticPath::const_new("frame_time");

    /// Updates frame count, frame time and fps measurements.
    pub fn diagnostic_system(
        mut diagnostics: Diagnostics,
        time: Res<Time<Real>>,
        frame_count: Res<FrameCount>,
    ) {
        diagnostics.add_measurement(&Self::FRAME_COUNT, || frame_count.0 as f64);

        let delta_seconds = time.delta_secs_f64();
        if delta_seconds == 0.0 {
            return;
        }

        diagnostics.add_measurement(&Self::FRAME_TIME, || delta_seconds * 1000.0);

        diagnostics.add_measurement(&Self::FPS, || 1.0 / delta_seconds);
    }
}
