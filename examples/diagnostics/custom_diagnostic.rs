//! This example illustrates how to create a custom diagnostic.

use bevy::{
    diagnostic::{
        Diagnostic, DiagnosticPath, Diagnostics, LogDiagnosticsPlugin, RegisterDiagnostic,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // The "print diagnostics" plugin is optional.
            // It just visualizes our diagnostics in the console.
            LogDiagnosticsPlugin::default(),
        ))
        // Diagnostics must be initialized before measurements can be added.
        .register_diagnostic(Diagnostic::new(SYSTEM_ITERATION_COUNT).with_suffix(" iterations"))
        .add_systems(Update, my_system)
        .run();
}

// All diagnostics should have a unique DiagnosticPath.
const SYSTEM_ITERATION_COUNT: DiagnosticPath = DiagnosticPath::const_new("system_iteration_count");

fn my_system(mut diagnostics: Diagnostics) {
    // Add a measurement of 10.0 for our diagnostic each time this system runs.
    diagnostics.add_measurement(&SYSTEM_ITERATION_COUNT, || 10.0);
}
