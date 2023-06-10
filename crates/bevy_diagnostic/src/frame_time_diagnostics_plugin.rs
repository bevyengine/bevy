use crate::{Diagnostic, DiagnosticId, Diagnostics, RegisterDiagnostic};
use bevy_app::prelude::*;
use bevy_core::FrameCount;
use bevy_ecs::prelude::*;
use bevy_time::Time;

/// Adds "frame time" diagnostic to an App, specifically "frame time", "fps" and "frame count"
#[derive(Default)]
pub struct FrameTimeDiagnosticsPlugin;

impl Plugin for FrameTimeDiagnosticsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_diagnostic(
            Diagnostic::new(Self::FRAME_TIME, "frame_time", 20).with_suffix("ms"),
        )
        .register_diagnostic(Diagnostic::new(Self::FPS, "fps", 20))
        .register_diagnostic(
            Diagnostic::new(Self::FRAME_COUNT, "frame_count", 1).with_smoothing_factor(0.0),
        )
        .add_systems(Update, Self::diagnostic_system);
    }
}

impl FrameTimeDiagnosticsPlugin {
    pub const FPS: DiagnosticId = DiagnosticId::from_u128(288146834822086093791974408528866909483);
    pub const FRAME_COUNT: DiagnosticId =
        DiagnosticId::from_u128(54021991829115352065418785002088010277);
    pub const FRAME_TIME: DiagnosticId =
        DiagnosticId::from_u128(73441630925388532774622109383099159699);

    pub fn diagnostic_system(
        mut diagnostics: Diagnostics,
        time: Res<Time>,
        frame_count: Res<FrameCount>,
    ) {
        diagnostics.add_measurement(Self::FRAME_COUNT, || frame_count.0 as f64);

        let delta_seconds = time.raw_delta_seconds_f64();
        if delta_seconds == 0.0 {
            return;
        }

        diagnostics.add_measurement(Self::FRAME_TIME, || delta_seconds * 1000.0);

        diagnostics.add_measurement(Self::FPS, || 1.0 / delta_seconds);
    }
}
