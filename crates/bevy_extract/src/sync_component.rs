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
/// It adds [`SyncToSubWorld`] as a required component to make the [`SyncWorldPlugin`] aware of the component, and
/// handles cleanup of the component in the render world when it is removed from an entity.
///
/// [`ExtractComponentPlugin`]: crate::extract_component::ExtractComponentPlugin
/// [`SyncWorldPlugin`]: crate::sync_world::SyncWorldPlugin
pub struct SyncComponentPlugin<L: AppLabel, C, F = ()>(PhantomData<(L, C, F)>);

impl<L: AppLabel, C: SyncComponent<L, F>, F> Default for SyncComponentPlugin<L, C, F> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Trait that links components from the main world with output components in
/// the render world. It is used by [`SyncComponentPlugin`].
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`SyncComponentPlugin`].
///
/// [`ExtractComponent`]: crate::extract_component::ExtractComponent
pub trait SyncComponent<L: AppLabel, F = ()>: Component {
    /// Describes what components should be removed from the render world if the
    /// implementing component is removed.
    type Target: Bundle<Effect: NoBundleEffect>;
    // TODO: https://github.com/rust-lang/rust/issues/29661
    // type Target: Bundle<Effect: NoBundleEffect> = Self;
}

impl<
        L: AppLabel + Default + Clone + Copy + Eq,
        C: SyncComponent<L, F>,
        F: Send + Sync + 'static,
    > Plugin for SyncComponentPlugin<L, C, F>
{
    fn build(&self, app: &mut App) {
        app.register_required_components::<C, SyncToSubWorld<L>>();

        app.add_observer(
            |remove: On<Remove, C>, mut pending: ResMut<PendingSyncEntity<L>>| {
                pending.push(EntityRecord::<L>::ComponentRemoved(
                    remove.entity,
                    |mut entity| {
                        entity.remove::<C::Target>();
                    },
                ));
            },
        );
    }
}
