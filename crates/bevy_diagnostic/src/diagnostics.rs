use super::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_core::time::Time;
use legion::prelude::*;
use std::time::Duration;

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
