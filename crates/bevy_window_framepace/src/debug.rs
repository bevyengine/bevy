//! Adds diagnostics for frame pacing.

use bevy_app::prelude::*;
use bevy_diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};
use bevy_ecs::prelude::*;

use crate::FramePaceStats;

/// Adds [`Diagnostics`] data from [`FramePaceStats`].
pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::diagnostic_system)
            .register_diagnostic(Diagnostic::new(Self::FRAMEPACE_FRAMETIME).with_suffix("ms"))
            .register_diagnostic(Diagnostic::new(Self::FRAMEPACE_OVERSLEEP).with_suffix("µs"));
    }
}

impl DiagnosticsPlugin {
    /// Frame time before the frame limiter sleeps, in milliseconds.
    pub const FRAMEPACE_FRAMETIME: DiagnosticPath =
        DiagnosticPath::const_new("framepace/frametime");
    /// Time the frame limiter slept past its target, in microseconds.
    pub const FRAMEPACE_OVERSLEEP: DiagnosticPath =
        DiagnosticPath::const_new("framepace/oversleep");

    fn diagnostic_system(mut diagnostics: Diagnostics, stats: Res<FramePaceStats>) {
        if let Ok(frametime) = stats.frametime.try_lock() {
            let frametime_millis = frametime.as_secs_f64() * 1_000.0;
            diagnostics.add_measurement(&Self::FRAMEPACE_FRAMETIME, || frametime_millis);
        }
        if let Ok(oversleep) = stats.oversleep.try_lock() {
            let oversleep_micros = oversleep.as_secs_f64() * 1_000_000.0;
            diagnostics.add_measurement(&Self::FRAMEPACE_OVERSLEEP, || oversleep_micros);
        }
    }
}
