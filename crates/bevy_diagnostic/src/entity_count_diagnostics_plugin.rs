use bevy_app::prelude::*;
use bevy_ecs::entity::Entities;

use crate::{
    Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic, DEFAULT_MAX_HISTORY_LENGTH,
};

/// Adds "entity count" diagnostic to an App.
///
/// # See also
///
/// [`LogDiagnosticsPlugin`](crate::LogDiagnosticsPlugin) to output diagnostics to the console.
pub struct EntityCountDiagnosticsPlugin {
    /// The total number of values to keep.
    pub max_history_length: usize,
}

impl Default for EntityCountDiagnosticsPlugin {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_HISTORY_LENGTH)
    }
}

impl EntityCountDiagnosticsPlugin {
    /// Creates a new `EntityCountDiagnosticsPlugin` with the specified `max_history_length`.
    pub fn new(max_history_length: usize) -> Self {
        Self { max_history_length }
    }
}

impl Plugin for EntityCountDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.register_diagnostic(
            Diagnostic::new(Self::ENTITY_COUNT).with_max_history_length(self.max_history_length),
        )
        .add_systems(Update, Self::diagnostic_system);
    }
}

impl EntityCountDiagnosticsPlugin {
    /// Number of currently allocated entities.
    pub const ENTITY_COUNT: DiagnosticPath = DiagnosticPath::const_new("entity_count");

    /// Updates entity count measurement.
    pub fn diagnostic_system(mut diagnostics: Diagnostics, entities: &Entities) {
        diagnostics.add_measurement(&Self::ENTITY_COUNT, || entities.len() as f64);
    }
}
