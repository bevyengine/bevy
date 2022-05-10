use crate::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_app::prelude::*;
use bevy_core::Time;
use bevy_ecs::system::{Res, ResMut};

/// Adds "frame time" diagnostic to an App, specifically "frame time", "fps" and "frame count"
#[derive(Default)]
pub struct FrameTimeDiagnosticsPlugin;

pub struct FrameTimeDiagnosticsState {
    frame_count: f64,
}

impl Plugin for FrameTimeDiagnosticsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_startup_system(Self::setup_system)
            .insert_resource(FrameTimeDiagnosticsState { frame_count: 0.0 })
            .add_system(Self::diagnostic_system);
    }
}

impl FrameTimeDiagnosticsPlugin {
    pub const FPS: DiagnosticId = DiagnosticId::from_u128(288146834822086093791974408528866909483);
    pub const FRAME_COUNT: DiagnosticId =
        DiagnosticId::from_u128(54021991829115352065418785002088010277);
    pub const FRAME_TIME: DiagnosticId =
        DiagnosticId::from_u128(73441630925388532774622109383099159699);

    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(Self::FRAME_TIME, "frame_time", 20).with_suffix("s"));
        diagnostics.add(Diagnostic::new(Self::FPS, "fps", 20));
        diagnostics.add(Diagnostic::new(Self::FRAME_COUNT, "frame_count", 1));
    }

    pub fn diagnostic_system(
        mut diagnostics: ResMut<Diagnostics>,
        time: Res<Time>,
        mut state: ResMut<FrameTimeDiagnosticsState>,
    ) {
        state.frame_count += 1.0;
        diagnostics.add_measurement(Self::FRAME_COUNT, state.frame_count);

        if time.delta_seconds_f64() == 0.0 {
            return;
        }

        diagnostics.add_measurement(Self::FRAME_TIME, time.delta_seconds_f64());
        if let Some(fps) = diagnostics
            .get(Self::FRAME_TIME)
            .and_then(|frame_time_diagnostic| {
                frame_time_diagnostic
                    .average()
                    .and_then(|frame_time_average| {
                        if frame_time_average > 0.0 {
                            Some(1.0 / frame_time_average)
                        } else {
                            None
                        }
                    })
            })
        {
            diagnostics.add_measurement(Self::FPS, fps);
        }
    }
}
