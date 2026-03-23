use core::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_ecs::{
    bundle::{Bundle, NoBundleEffect},
    component::Component,
};

use crate::sync_world::{EntityRecord, PendingSyncEntity, SyncToRenderWorld};

/// Plugin that registers a component for automatic sync to the render world. See [`SyncWorldPlugin`] for more information.
///
/// This plugin is automatically added by [`ExtractComponentPlugin`], and only needs to be added for manual extraction implementations.
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`SyncComponentPlugin`].
///
/// # Implementation details
///
/// It adds [`SyncToRenderWorld`] as a required component to make the [`SyncWorldPlugin`] aware of the component, and
/// handles cleanup of the component in the render world when it is removed from an entity.
///
/// [`ExtractComponentPlugin`]: crate::extract_component::ExtractComponentPlugin
/// [`SyncWorldPlugin`]: crate::sync_world::SyncWorldPlugin
pub struct SyncComponentPlugin<C, F = ()>(PhantomData<(C, F)>);

impl<C: SyncComponent<F>, F> Default for SyncComponentPlugin<C, F> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Trait that links components from the main world with output components in
/// the render world. It is used by [`SyncComponentPlugin`].
///
/// This trait is a subtrait of [`ExtractComponent`], which uses it to determine
/// which components to extract.
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`SyncComponentPlugin`].
///
/// [`ExtractComponent`]: crate::extract_component::ExtractComponent
pub trait SyncComponent<F = ()>: Component {
    /// Describes what components should be removed from the render world if the
    /// implementing component is removed.
    ///
    /// It is also used by the [`ExtractComponent`] trait to determine which
    /// components are generated during extraction.
    ///
    /// [`ExtractComponent`]: crate::extract_component::ExtractComponent
    type Out: Bundle<Effect: NoBundleEffect>;
    // TODO: https://github.com/rust-lang/rust/issues/29661
    // type Out: Component = Self;
}

impl<C: SyncComponent<F>, F: Send + Sync + 'static> Plugin for SyncComponentPlugin<C, F> {
    fn build(&self, app: &mut App) {
        app.register_required_components::<C, SyncToRenderWorld>();

        app.world_mut()
            .register_component_hooks::<C>()
            .on_remove(|mut world, context| {
                let mut pending = world.resource_mut::<PendingSyncEntity>();
                pending.push(EntityRecord::ComponentRemoved(
                    context.entity,
                    |mut entity| {
                        entity.remove::<C::Out>();
                    },
                ));
            });
    }
}
