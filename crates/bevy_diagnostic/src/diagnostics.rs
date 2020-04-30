use super::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_core::time::Time;
use legion::prelude::*;
use std::time::Duration;
use uuid::Uuid;

pub const FPS: DiagnosticId = DiagnosticId(Uuid::from_bytes([
    157, 191, 0, 72, 223, 223, 70, 128, 137, 117, 54, 177, 132, 13, 170, 124,
]));

pub const FRAME_TIME: DiagnosticId = DiagnosticId(Uuid::from_bytes([
    216, 184, 55, 12, 28, 116, 69, 201, 187, 137, 176, 77, 83, 89, 251, 241,
]));

pub fn setup_frame_time_diagnostic_system(mut diagnostics: ResourceMut<Diagnostics>) {
    diagnostics.add(Diagnostic::new(FRAME_TIME, "frame_time", 10));
    diagnostics.add(Diagnostic::new(FPS, "fps", 10));
}

pub fn frame_time_diagnostic_system(
    mut diagnostics: ResourceMut<Diagnostics>,
    time: Resource<Time>,
) {
    if time.delta_seconds_f64 == 0.0 {
        return;
    }

    diagnostics.add_measurement(FRAME_TIME, time.delta_seconds_f64);
    if let Some(fps) = diagnostics
        .get(FRAME_TIME)
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
        diagnostics.add_measurement(FPS, fps);
    }
}

pub struct PrintDiagnosticsState {
    elapsed: f64,
    wait_seconds: f64,
}

impl PrintDiagnosticsState {
    pub fn new(wait: Duration) -> Self {
        PrintDiagnosticsState {
            elapsed: 0.,
            wait_seconds: wait.as_secs_f64(),
        }
    }
}

pub fn print_diagnostics_system(
    mut state: ResourceMut<PrintDiagnosticsState>,
    time: Resource<Time>,
    diagnostics: Resource<Diagnostics>,
) {
    state.elapsed += time.delta_seconds_f64;
    if state.elapsed >= state.wait_seconds {
        state.elapsed = 0.0;
        for diagnostic in diagnostics.iter() {
            if let Some(value) = diagnostic.value() {
                print!("{:<10}: {:<9.6}", diagnostic.name, value);
                if let Some(average) = diagnostic.average() {
                    print!("  (avg {:.6})", average);
                }

                println!("\n");
            }
        }
    }
}

pub fn print_diagnostics_debug_system(
    mut state: ResourceMut<PrintDiagnosticsState>,
    time: Resource<Time>,
    diagnostics: Resource<Diagnostics>,
) {
    state.elapsed += time.delta_seconds_f64;
    if state.elapsed >= state.wait_seconds {
        state.elapsed = 0.0;
        for diagnostic in diagnostics.iter() {
            println!("{:#?}\n", diagnostic);
        }
    }
}
