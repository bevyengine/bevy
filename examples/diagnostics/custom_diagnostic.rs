//! This example illustrates how to create a custom diagnostic.

use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, LogDiagnosticsPlugin, RegisterDiagnostic},
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
        .register_diagnostic(
            Diagnostic::new(SYSTEM_ITERATION_COUNT, "system_iteration_count", 10)
                .with_suffix(" iterations"),
        )
        .add_systems(Update, my_system)
        .run();
}

// All diagnostics should have a unique DiagnosticId.
// For each new diagnostic, generate a new random number.
pub const SYSTEM_ITERATION_COUNT: DiagnosticId =
    DiagnosticId::from_u128(337040787172757619024841343456040760896);

fn my_system(mut diagnostics: Diagnostics) {
    // Add a measurement of 10.0 for our diagnostic each time this system runs.
    diagnostics.add_measurement(SYSTEM_ITERATION_COUNT, || 10.0);
}
