use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;

use crate::world_sync::{EntityRecord, PendingSyncEntity, SyncToRenderWorld};

pub struct SyncComponentPlugin<C: Component>(PhantomData<C>);

impl<C: Component> Default for SyncComponentPlugin<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C: Component> Plugin for SyncComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        app.register_required_components::<C, SyncToRenderWorld>();

        app.world_mut().register_component_hooks::<C>().on_remove(
            |mut world, entity, _component_id| {
                let mut pending = world.resource_mut::<PendingSyncEntity>();
                pending.push(EntityRecord::ComponentRemoved(entity));
            },
        );
    }
}
