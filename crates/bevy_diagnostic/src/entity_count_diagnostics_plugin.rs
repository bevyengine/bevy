use crate::{Diagnostic, DiagnosticId, Diagnostics};
use bevy_app::prelude::*;
use bevy_ecs::{Entity, IntoSystem, Query, ResMut};

/// Adds "entity count" diagnostic to an App
#[derive(Default)]
pub struct EntityCountDiagnosticsPlugin;

impl Plugin for EntityCountDiagnosticsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(Self::setup_system.system())
            .add_system(Self::diagnostic_system.system());
    }
}

impl EntityCountDiagnosticsPlugin {
    pub const ENTITY_COUNT: DiagnosticId =
        DiagnosticId::from_u128(187513512115068938494459732780662867798);

    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(Self::ENTITY_COUNT, "entity_count", 20));
    }

    pub fn diagnostic_system(mut diagnostics: ResMut<Diagnostics>, query: Query<Entity>) {
        let mut count = 0.;
        for _entity in &mut query.iter() {
            count += 1.;
        }
        diagnostics.add_measurement(Self::ENTITY_COUNT, count);
    }
}
