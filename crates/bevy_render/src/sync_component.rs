use core::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;

use crate::sync_world::{EntityRecord, PendingSyncEntity, SyncToRenderWorld};

/// Plugin that registers a component for automatic sync to the render world. See [`SyncWorldPlugin`] for more information.
///
/// This plugin is automatically added by [`ExtractComponentPlugin`], and only needs to be added for manual extraction implementations.
///
/// # Implementation details
///
/// It adds [`SyncToRenderWorld`] as a required component to make the [`SyncWorldPlugin`] aware of the component, and
/// handles cleanup of the component in the render world when it is removed from an entity.
///
/// # Warning
/// When the component is removed from the main world entity, all components are removed from the entity in the render world.
/// This is done in order to handle components with custom extraction logic and derived state.
///
/// [`ExtractComponentPlugin`]: crate::extract_component::ExtractComponentPlugin
/// [`SyncWorldPlugin`]: crate::sync_world::SyncWorldPlugin
pub struct SyncComponentPlugin<C: Component>(PhantomData<C>);

impl<C: Component> Default for SyncComponentPlugin<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C: Component> Plugin for SyncComponentPlugin<C> {
    fn build(&self, app: &mut App) {
        app.register_required_components::<C, SyncToRenderWorld>();

        app.world_mut()
            .register_component_hooks::<C>()
            .on_remove(|mut world, context| {
                let mut pending = world.resource_mut::<PendingSyncEntity>();
                pending.push(EntityRecord::ComponentRemoved(context.entity));
            });
    }
}
