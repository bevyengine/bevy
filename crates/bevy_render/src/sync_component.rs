use core::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;

use crate::world_sync::{EntityRecord, PendingSyncEntity, SyncToRenderWorld};

/// Plugin that registers a component for automatic sync to the render world. See [`WorldSyncPlugin`] for more information.
///
/// This plugin is automatically added by [`ExtractComponentPlugin`], and only needs to be added for manual extraction implementations.
///
/// # Implementation details
///
/// It adds [`SyncToRenderWorld`] as a required component to make the [`WorldSyncPlugin`] aware of the component, and
/// handles cleanup of the component in the render world when it is removed from an entity.
///
/// NOTE: When the component is removed from the main world entity, all components are removed from the entity in the render world.
///       This is in order to handle components with custom extraction logic.
///
/// [`ExtractComponentPlugin`]: crate::extract_component::ExtractComponentPlugin
/// [`WorldSyncPlugin`]: crate::world_sync::WorldSyncPlugin
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
