use crate::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_app::{prelude::*, PluginGroupBuilder};
use bevy_core::FrameCount;
use bevy_ecs::prelude::*;
use bevy_time::Time;

/// This plugin group will add "frame time" diagnostics to an App, namely:
/// * [`FrameTimeDiagnosticsPlugin`](crate::FrameTimeDiagnosticsPlugin)
/// * [`FpsDiagnosticsPlugin`](crate::FpsDiagnosticsPlugin)
/// * [`FrameCountDiagnosticsPlugin`](crate::FrameCountDiagnosticsPlugin)
#[derive(Default)]
pub struct BasicPerformanceDiagnosticsPlugins;

impl PluginGroup for BasicPerformanceDiagnosticsPlugins {
    fn build(self) -> bevy_app::PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(FpsDiagnosticsPlugin)
            .add(FrameTimeDiagnosticsPlugin)
            .add(FrameCountDiagnosticsPlugin)
    }
}

/// Adds "frame time" diagnostic to an App
#[derive(Default)]
pub struct FrameTimeDiagnosticsPlugin;

impl Plugin for FrameTimeDiagnosticsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_startup_system(Self::setup_system)
            .add_system(Self::frame_time_diagnostic_system);
    }
}

impl FrameTimeDiagnosticsPlugin {
    /// Used as a key to retrieve the frame time diagnostic from [Diagnostics]
    pub const FRAME_TIME: DiagnosticId =
        DiagnosticId::from_u128(73441630925388532774622109383099159699);

    /// Adds the frame time diagnostic to the [Diagnostics] resource
    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(Self::FRAME_TIME, "frame_time", 20).with_suffix("ms"));
    }
    /// Updates the frame time diagnostic
    pub fn frame_time_diagnostic_system(mut diagnostics: ResMut<Diagnostics>, time: Res<Time>) {
        let delta_seconds = time.raw_delta_seconds_f64();
        if delta_seconds == 0.0 {
            return;
        }
        diagnostics.add_measurement(Self::FRAME_TIME, || delta_seconds * 1000.0);
    }
}
/// Adds "frame per second" diagnostic to an App
#[derive(Default)]
pub struct FpsDiagnosticsPlugin;

impl Plugin for FpsDiagnosticsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_startup_system(Self::setup_system)
            .add_system(Self::fps_diagnostic_system);
    }
}
impl FpsDiagnosticsPlugin {
    /// Used as a key to retrieve the fps diagnostic from [Diagnostics]
    pub const FPS: DiagnosticId = DiagnosticId::from_u128(288146834822086093791974408528866909483);

    /// Adds the fps diagnostic to the [Diagnostics] resource
    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(Self::FPS, "fps", 20));
    }
    /// Updates the fps diagnostic
    pub fn fps_diagnostic_system(mut diagnostics: ResMut<Diagnostics>, time: Res<Time>) {
        let delta_seconds = time.raw_delta_seconds_f64();
        if delta_seconds == 0.0 {
            return;
        }
        diagnostics.add_measurement(Self::FPS, || 1.0 / delta_seconds);
    }
}
/// Adds "frame count" diagnostic to an App
#[derive(Default)]
pub struct FrameCountDiagnosticsPlugin;
impl Plugin for FrameCountDiagnosticsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_startup_system(Self::setup_system)
            .add_system(Self::frame_count_diagnostic_system);
    }
}
impl FrameCountDiagnosticsPlugin {
    /// Used as a key to retrieve the frame count diagnostic from [Diagnostics]
    pub const FRAME_COUNT: DiagnosticId =
        DiagnosticId::from_u128(54021991829115352065418785002088010277);

    /// Adds the frame count diagnostic to the [Diagnostics] resource
    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(Self::FRAME_COUNT, "frame_count", 20).with_suffix("ms"));
    }
    /// Updates the frame count diagnostic
    pub fn frame_count_diagnostic_system(
        mut diagnostics: ResMut<Diagnostics>,
        frame_count: Res<FrameCount>,
    ) {
        diagnostics.add_measurement(Self::FRAME_COUNT, || frame_count.0 as f64);
    }
}
