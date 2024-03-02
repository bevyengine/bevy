use bevy_app::prelude::*;
use bevy_ecs::entity::Entities;

use crate::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};

/// Adds "entity count" diagnostic to an App.
///
/// # See also
///
/// [`LogDiagnosticsPlugin`](crate::LogDiagnosticsPlugin) to output diagnostics to the console.
#[derive(Default)]
pub struct EntityCountDiagnosticsPlugin;

impl Plugin for EntityCountDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.register_diagnostic(Diagnostic::new(Self::ENTITY_COUNT))
            .add_systems(Update, Self::diagnostic_system);
    }
}

impl EntityCountDiagnosticsPlugin {
    pub const ENTITY_COUNT: DiagnosticPath = DiagnosticPath::const_new("entity_count");

    pub fn diagnostic_system(mut diagnostics: Diagnostics, entities: &Entities) {
        diagnostics.add_measurement(&Self::ENTITY_COUNT, || entities.len() as f64);
    }
}
