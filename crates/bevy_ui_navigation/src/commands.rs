use bevy_ecs::{entity::Entity, prelude::World, system::Command};

use crate::focusable::{FocusState, Focused};

pub(crate) fn set_focus_state(entity: Entity, new_state: FocusState) -> UpdateFocusable {
    UpdateFocusable { entity, new_state }
}
pub(crate) struct UpdateFocusable {
    entity: Entity,
    new_state: FocusState,
}
impl Command for UpdateFocusable {
    fn write(self, world: &mut World) {
        let mut entity = world.entity_mut(self.entity);
        if matches!(self.new_state, FocusState::Focused) {
            entity.insert(Focused);
        } else {
            entity.remove::<Focused>();
        }
    }
}
