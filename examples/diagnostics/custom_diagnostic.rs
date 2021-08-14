use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics, LogDiagnosticsPlugin},
    prelude::*,
};

/// This example illustrates how to create a custom diagnostic
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // The "print diagnostics" plugin is optional. It just visualizes our diagnostics in the
        // console
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_startup_system(setup_diagnostic_system)
        .add_system(my_system)
        .run();
}

// All diagnostics should have a unique DiagnosticId. for each new diagnostic, generate a new random
// number
pub const SYSTEM_ITERATION_COUNT: DiagnosticId =
    DiagnosticId::from_u128(337040787172757619024841343456040760896);

fn setup_diagnostic_system(mut diagnostics: ResMut<Diagnostics>) {
    // Diagnostics must be initialized before measurements can be added.
    // In general it's a good idea to set them up in a "startup system".
    diagnostics.add(Diagnostic::new(
        SYSTEM_ITERATION_COUNT,
        "system_iteration_count",
        10,
    ));
}

fn my_system(mut diagnostics: ResMut<Diagnostics>) {
    // Add a measurement of 10.0 for our diagnostic each time this system runs
    diagnostics.add_measurement(SYSTEM_ITERATION_COUNT, 10.0);
}
