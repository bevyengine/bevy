use bevy_app::{App, Plugin};
use bevy_ecs::{
    system::{IntoExclusiveSystem, ResMut},
    world::World,
};

use crate::{Diagnostic, DiagnosticId, Diagnostics};

/// Adds "entity count" diagnostic to an App
#[derive(Default)]
pub struct EntityCountDiagnosticsPlugin;

impl Plugin for EntityCountDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::setup_system)
            .add_system(Self::diagnostic_system.exclusive_system());
    }
}

impl EntityCountDiagnosticsPlugin {
    pub const ENTITY_COUNT: DiagnosticId =
        DiagnosticId::from_u128(187513512115068938494459732780662867798);

    pub fn setup_system(mut diagnostics: ResMut<Diagnostics>) {
        diagnostics.add(Diagnostic::new(Self::ENTITY_COUNT, "entity_count", 20));
    }

    pub fn diagnostic_system(world: &mut World) {
        let entity_count = world.entities().len();
        if let Some(mut diagnostics) = world.get_resource_mut::<Diagnostics>() {
            diagnostics.add_measurement(Self::ENTITY_COUNT, entity_count as f64);
        }
    }
}
