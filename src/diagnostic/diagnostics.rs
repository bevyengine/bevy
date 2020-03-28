use super::{Diagnostic, DiagnosticId, Diagnostics};
use crate::{
    core::Time,
    prelude::{Resources, SystemBuilder},
};
use legion::prelude::Schedulable;
use std::time::Duration;
use uuid::Uuid;

pub const FPS: DiagnosticId = DiagnosticId(Uuid::from_bytes([
    157, 191, 0, 72, 223, 223, 70, 128, 137, 117, 54, 177, 132, 13, 170, 124,
]));

pub const FRAME_TIME: DiagnosticId = DiagnosticId(Uuid::from_bytes([
    216, 184, 55, 12, 28, 116, 69, 201, 187, 137, 176, 77, 83, 89, 251, 241,
]));

pub fn frame_time_diagnostic_system(
    resources: &Resources,
    max_history_length: usize,
) -> Box<dyn Schedulable> {
    let mut diagnostics = resources.get_mut::<Diagnostics>().unwrap();
    diagnostics.add(Diagnostic::new(
        FRAME_TIME,
        "frame_time",
        max_history_length,
    ));
    diagnostics.add(Diagnostic::new(FPS, "fps", max_history_length));
    SystemBuilder::new("FrameTimeDiagnostic")
        .read_resource::<Time>()
        .write_resource::<Diagnostics>()
        .build(move |_, _world, (time, ref mut diagnostics), _queries| {
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
        })
}

pub fn print_diagnostics_system(wait: Duration) -> Box<dyn Schedulable> {
    let mut elasped = 0.0;
    let wait_seconds = wait.as_secs_f64();
    SystemBuilder::new("PrintDiagnostics")
        .read_resource::<Time>()
        .read_resource::<Diagnostics>()
        .build(move |_, _world, (time, diagnostics), _queries| {
            elasped += time.delta_seconds_f64;
            if elasped >= wait_seconds {
                elasped = 0.0;
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
        })
}

pub fn print_diagnostics_debug_system(wait: Duration) -> Box<dyn Schedulable> {
    let mut elasped = 0.0;
    let wait_seconds = wait.as_secs_f64();
    SystemBuilder::new("PrintDiagnostics")
        .read_resource::<Time>()
        .read_resource::<Diagnostics>()
        .build(move |_, _world, (time, diagnostics), _queries| {
            elasped += time.delta_seconds_f64;
            if elasped >= wait_seconds {
                elasped = 0.0;
                for diagnostic in diagnostics.iter() {
                    println!("{:#?}\n", diagnostic);
                }
            }
        })
}