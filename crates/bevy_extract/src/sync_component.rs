use core::marker::PhantomData;

use bevy_app::{App, AppLabel, Plugin};
use bevy_ecs::{
    bundle::{Bundle, NoBundleEffect},
    component::Component,
    lifecycle::Remove,
    observer::On,
    system::ResMut,
};

use crate::sync_world::{EntityRecord, PendingSyncEntity, SyncToSubWorld};

/// Plugin that registers a component for automatic sync to the sub world. See [`SyncWorldPlugin`] for more information.
///
/// This plugin is automatically added by [`ExtractBaseComponentPlugin`], and only needs to be added for manual extraction implementations.
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`SyncComponentPlugin`].
///
/// # Implementation details
///
/// It adds [`SyncToSubWorld`] as a required component to make the [`SyncWorldPlugin`] aware of the component, and
/// handles cleanup of the component in the sub world when it is removed from an entity.
///
/// [`ExtractBaseComponentPlugin`]: crate::extract_base_component::ExtractBaseComponentPlugin
/// [`SyncWorldPlugin`]: crate::sync_world::SyncWorldPlugin
pub struct SyncComponentPlugin<L, C, F = ()>(PhantomData<(L, C, F)>)
where
    L: AppLabel + Default,
    C: SyncComponent<L, F>,
    F: 'static + Send + Sync;

impl<L: AppLabel + Default, C: SyncComponent<L, F>, F: 'static + Send + Sync> Default
    for SyncComponentPlugin<L, C, F>
{
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Trait that links components from the main world with output components in
/// the sub world. It is used by [`SyncComponentPlugin`].
///
/// This trait is a subtrait of [`ExtractBaseComponent`], which uses it to determine
/// which components to extract.
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`SyncComponentPlugin`].
///
/// [`ExtractBaseComponent`]: crate::extract_base_component::ExtractBaseComponent
pub trait SyncComponent<L: AppLabel + Default, F: 'static + Send + Sync = ()>: Component {
    /// Describes what components should be removed from the sub world if the
    /// implementing component is removed.
    ///
    /// It is also used by the [`ExtractBaseComponent`] trait to determine which
    /// components are generated during extraction.
    ///
    /// [`ExtractBaseComponent`]: crate::extract_base_component::ExtractBaseComponent
    type Out: Bundle<Effect: NoBundleEffect>;
    // TODO: https://github.com/rust-lang/rust/issues/29661
    // type Out: Component = Self;
}

impl<L: AppLabel + Default + Clone, C: SyncComponent<L, F>, F: Send + Sync + 'static> Plugin
    for SyncComponentPlugin<L, C, F>
{
    fn build(&self, app: &mut App) {
        app.register_required_components::<C, SyncToSubWorld<L>>();

        app.add_observer(
            |remove: On<Remove, C>, mut pending: ResMut<PendingSyncEntity>| {
                pending.push(EntityRecord::ComponentRemoved(
                    remove.entity,
                    |mut entity| {
                        entity.remove::<C::Out>();
                    },
                ));
            },
        );
    }
}
