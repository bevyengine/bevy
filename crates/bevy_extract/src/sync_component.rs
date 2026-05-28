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

use bevy_log::warn_once;

/// Plugin that registers a component for automatic sync to the sub world. See [`SyncWorldPlugin`] for more information.
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
/// handles cleanup of the component in the sub world when it is removed from an entity.
///
/// [`ExtractComponentPlugin`]: crate::extract_component::ExtractComponentPlugin
/// [`SyncWorldPlugin`]: crate::sync_world::SyncWorldPlugin
pub struct SyncComponentPlugin<C, L: AppLabel, F = ()>(PhantomData<(C, L, F)>);

// pub type SyncComponentPlugin<C, F = ()> = SyncComponentPlugin<C, RenderApp, F>;

impl<C: SyncComponent<L, F>, L: AppLabel, F> Default for SyncComponentPlugin<C, L, F> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Trait that links components from the main world with output components in
/// the sub world. It is used by [`SyncComponentPlugin`].
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`SyncComponentPlugin`].
///
/// [`ExtractComponent`]: crate::extract_component::ExtractComponent
pub trait SyncComponent<L: AppLabel, F = ()>: Component {
    /// Describes what components should be removed from the sub world if the
    /// implementing component is removed.
    type Target: Bundle<Effect: NoBundleEffect>;
    // TODO: https://github.com/rust-lang/rust/issues/29661
    // type Target: Bundle<Effect: NoBundleEffect> = Self;
}

impl<
        C: SyncComponent<L, F>,
        L: AppLabel + Default + Clone + Copy + Eq,
        F: Send + Sync + 'static,
    > Plugin for SyncComponentPlugin<C, L, F>
{
    fn build(&self, app: &mut App) {
        app.register_required_components::<C, SyncToSubWorld<L>>();

        app.add_observer(
            |remove: On<Remove, C>, maybe_pending: Option<ResMut<PendingSyncEntity<L>>>| {
                let Some(mut pending) = maybe_pending else {
                    warn_once!("A component with sync plugin was removed, but the sub world does not exist, so there is nothing to sync. Skip sync to sub world.");
                    return;
                };
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

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_ecs::component::Component;

    use super::{SyncComponent, SyncComponentPlugin};
    use crate::RenderApp;

    #[derive(Component)]
    struct TestSyncComponent;

    impl SyncComponent<RenderApp> for TestSyncComponent {
        type Target = Self;
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/24927:
    // with `WgpuSettings { backends: None }` there is no render world, and Bevy used to crash on removing any synced component.
    // This test checks that the bug does not happen again.
    #[test]
    fn remove_synced_component_without_render_world() {
        let mut app = App::new();
        app.add_plugins(SyncComponentPlugin::<TestSyncComponent>::default());

        let entity = app.world_mut().spawn(TestSyncComponent).id();
        app.world_mut().despawn(entity);
    }
}
