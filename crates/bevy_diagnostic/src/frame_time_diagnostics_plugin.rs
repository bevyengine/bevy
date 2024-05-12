use crate::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy_app::prelude::*;
use bevy_core::FrameCount;
use bevy_ecs::prelude::*;
use bevy_time::{Real, Time};

/// Adds "frame time" diagnostic to an App, specifically "frame time", "fps" and "frame count"
///
/// # See also
///
/// [`LogDiagnosticsPlugin`](crate::LogDiagnosticsPlugin) to output diagnostics to the console.
#[derive(Default)]
pub struct FrameTimeDiagnosticsPlugin;

impl Plugin for FrameTimeDiagnosticsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_diagnostic(Diagnostic::new(Self::FRAME_TIME).with_suffix("ms"))
            .register_diagnostic(Diagnostic::new(Self::FPS))
            .register_diagnostic(Diagnostic::new(Self::FRAME_COUNT).with_smoothing_factor(0.0))
            .add_systems(Update, Self::diagnostic_system);
    }
}

impl FrameTimeDiagnosticsPlugin {
    pub const FPS: DiagnosticPath = DiagnosticPath::const_new("fps");
    pub const FRAME_COUNT: DiagnosticPath = DiagnosticPath::const_new("frame_count");
    pub const FRAME_TIME: DiagnosticPath = DiagnosticPath::const_new("frame_time");

    pub fn diagnostic_system(
        mut diagnostics: Diagnostics,
        time: Res<Time<Real>>,
        frame_count: Res<FrameCount>,
    ) {
        diagnostics.add_measurement(&Self::FRAME_COUNT, || frame_count.0 as f64);

        let delta_seconds = time.delta_seconds_f64();
        if delta_seconds == 0.0 {
            return;
        }

        diagnostics.add_measurement(&Self::FRAME_TIME, || delta_seconds * 1000.0);

        diagnostics.add_measurement(&Self::FPS, || 1.0 / delta_seconds);
    }
}
