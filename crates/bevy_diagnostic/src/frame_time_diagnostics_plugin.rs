use crate::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_app::prelude::*;
use bevy_core::Time;
use bevy_ecs::{IntoQuerySystem, Res, ResMut};

/// Adds "frame time" diagnostic to an App, specifically "frame time" and "fps"
#[derive(Default)]
pub struct FrameTimeDiagnosticsPlugin;

impl Plugin for FrameTimeDiagnosticsPlugin {
    fn build(&self, app: &mut bevy_app::AppBuilder) {
        app.add_startup_system(Self::setup_system.system())
            .add_system(Self::diagnostic_system.system());
    }
}

impl FrameTimeDiagnosticsPlugin {
    pub const FPS: DiagnosticId = DiagnosticId::from_u128(288146834822086093791974408528866909483);
    pub const FRAME_TIME: DiagnosticId =
        DiagnosticId::from_u128(54021991829115352065418785002088010276);

    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(Self::FRAME_TIME, "frame_time", 20));
        diagnostics.add(Diagnostic::new(Self::FPS, "fps", 20));
    }

    pub fn diagnostic_system(mut diagnostics: ResMut<Diagnostics>, time: Res<Time>) {
        if time.delta_seconds_f64 == 0.0 {
            return;
        }

        diagnostics.add_measurement(Self::FRAME_TIME, time.delta_seconds_f64);
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
