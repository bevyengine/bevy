use super::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_app::{stage, AppPlugin};
use bevy_core::time::{Time, Timer};
use legion::prelude::*;
use std::time::Duration;

pub struct PrintDiagnosticsPlugin {
    pub debug: bool,
    pub wait_duration: Duration,
    pub filter: Option<Vec<DiagnosticId>>,
}

pub struct PrintDiagnosticsState {
    timer: Timer,
    filter: Option<Vec<DiagnosticId>>,
}

impl Default for PrintDiagnosticsPlugin {
    fn default() -> Self {
        PrintDiagnosticsPlugin {
            debug: false,
            wait_duration: Duration::from_secs(1),
            filter: None,
        }
    }
}

impl AppPlugin for PrintDiagnosticsPlugin {
    fn build(&self, app: &mut bevy_app::AppBuilder) {
        app.add_resource(PrintDiagnosticsState {
            timer: Timer::new(self.wait_duration),
            filter: self.filter.clone(),
        });

        if self.debug {
            app.add_system_to_stage(
                stage::POST_UPDATE,
                Self::print_diagnostics_debug_system.system(),
            );
        } else {
            app.add_system_to_stage(stage::POST_UPDATE, Self::print_diagnostics_system.system());
        }
    }
}

impl PrintDiagnosticsPlugin {
    pub fn filtered(filter: Vec<DiagnosticId>) -> Self {
        PrintDiagnosticsPlugin {
            filter: Some(filter),
            ..Default::default()
        }
    }

    fn print_diagnostic(diagnostic: &Diagnostic) {
        if let Some(value) = diagnostic.value() {
            print!("{:<65}: {:<10.6}", diagnostic.name, value);
            if let Some(average) = diagnostic.average() {
                print!("  (avg {:.6})", average);
            }

            println!("\n");
        }
    }

    pub fn print_diagnostics_system(
        mut state: ResMut<PrintDiagnosticsState>,
        time: Res<Time>,
        diagnostics: Res<Diagnostics>,
    ) {
        state.timer.tick(time.delta_seconds);
        if state.timer.finished {
            println!("Diagnostics:");
            println!("{}", "-".repeat(93));
            if let Some(ref filter) = state.filter {
                for diagnostic in filter.iter().map(|id| diagnostics.get(*id).unwrap()) {
                    Self::print_diagnostic(diagnostic);
                }
            } else {
                for diagnostic in diagnostics.iter() {
                    Self::print_diagnostic(diagnostic);
                }
            }

            state.timer.reset();
        }
    }

    pub fn print_diagnostics_debug_system(
        mut state: ResMut<PrintDiagnosticsState>,
        time: Res<Time>,
        diagnostics: Res<Diagnostics>,
    ) {
        state.timer.tick(time.delta_seconds);
        if state.timer.finished {
            println!("Diagnostics (Debug):");
            println!("{}", "-".repeat(93));
            if let Some(ref filter) = state.filter {
                for diagnostic in filter.iter().map(|id| diagnostics.get(*id).unwrap()) {
                    println!("{:#?}\n", diagnostic);
                }
            } else {
                for diagnostic in diagnostics.iter() {
                    println!("{:#?}\n", diagnostic);
                }
            }

            state.timer.reset();
        }
    }
}
