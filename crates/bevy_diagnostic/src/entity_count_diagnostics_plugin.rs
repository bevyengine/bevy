use bevy_app::prelude::*;
use bevy_ecs::entity::Entities;

use crate::{Diagnostic, DiagnosticId, Diagnostics, RegisterDiagnostic};

/// Adds "entity count" diagnostic to an App
#[derive(Default)]
pub struct EntityCountDiagnosticsPlugin;

impl Plugin for EntityCountDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.register_diagnostic(Diagnostic::new(Self::ENTITY_COUNT, "entity_count", 20))
            .add_systems(Update, Self::diagnostic_system);
    }
}

impl EntityCountDiagnosticsPlugin {
    pub const ENTITY_COUNT: DiagnosticId =
        DiagnosticId::from_u128(187513512115068938494459732780662867798);

    pub fn diagnostic_system(mut diagnostics: Diagnostics, entities: &Entities) {
        diagnostics.add_measurement(Self::ENTITY_COUNT, || entities.len() as f64);
    }
}
