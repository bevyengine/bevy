use super::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_core::time::Time;
use legion::prelude::*;
use std::time::Duration;

pub const FPS: DiagnosticId = DiagnosticId::from_u128(288146834822086093791974408528866909483);
pub const FRAME_TIME: DiagnosticId =
    DiagnosticId::from_u128(54021991829115352065418785002088010276);

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
    filter: Option<Vec<DiagnosticId>>,
}

impl PrintDiagnosticsState {
    pub fn new(wait: Duration) -> Self {
        PrintDiagnosticsState {
            elapsed: 0.,
            wait_seconds: wait.as_secs_f64(),
            filter: None,
        }
    }

    pub fn new_filtered(wait: Duration, filter: Vec<DiagnosticId>) -> Self {
        PrintDiagnosticsState {
            elapsed: 0.,
            wait_seconds: wait.as_secs_f64(),
            filter: Some(filter),
        }
    }
}

fn print_diagnostic(diagnostic: &Diagnostic) {
    if let Some(value) = diagnostic.value() {
        print!("{:<20}: {:<19.6}", diagnostic.name, value);
        if let Some(average) = diagnostic.average() {
            print!("  (avg {:.6})", average);
        }

        println!("\n");
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
        if let Some(ref filter) = state.filter {
            for diagnostic in filter.iter().map(|id| diagnostics.get(*id).unwrap()) {
                print_diagnostic(diagnostic);
            }
        } else {
            for diagnostic in diagnostics.iter() {
                print_diagnostic(diagnostic);
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
        if let Some(ref filter) = state.filter {
            for diagnostic in filter.iter().map(|id| diagnostics.get(*id).unwrap()) {
                println!("{:#?}\n", diagnostic);
            }
        } else {
            for diagnostic in diagnostics.iter() {
                println!("{:#?}\n", diagnostic);
            }
        }
    }
}
