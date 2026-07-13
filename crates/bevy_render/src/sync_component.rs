use core::marker::PhantomData;

use bevy_app::{App, AppLabel, Plugin};
use bevy_ecs::{
    bundle::{Bundle, NoBundleEffect},
    component::Component,
};

use crate::{
    sync_world::{EntityRecord, PendingSyncEntity, SyncToRenderWorld},
    RenderApp,
};

use bevy_log::warn_once;

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

impl<C: SyncComponent<RenderApp, F>, F> Default for SyncComponentPlugin<C, F> {
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
pub trait SyncComponent<L: AppLabel, F = ()>: Component {
    /// Describes what components should be removed from the render world if the
    /// implementing component is removed.
    type Target: Bundle<Effect: NoBundleEffect>;
    // TODO: https://github.com/rust-lang/rust/issues/29661
    // type Target: Bundle<Effect: NoBundleEffect> = Self;
}

impl<C: SyncComponent<RenderApp, F>, F: Send + Sync + 'static> Plugin
    for SyncComponentPlugin<C, F>
{
    fn build(&self, app: &mut App) {
        app.register_required_components::<C, SyncToRenderWorld>();

        app.world_mut()
            .register_component_hooks::<C>()
            .on_remove(|mut world, context| {
                let Some(mut pending) = world.get_resource_mut::<PendingSyncEntity>() else {
                    warn_once!("A component with render sync plugin was removed, but the render world does not exist (probably `WgpuSettings {{ backends: None }}`), so there is nothing to sync. Skip sync to render world.");
                    return;
                };
                pending.push(EntityRecord::ComponentRemoved(
                    context.entity,
                    |mut entity| {
                        entity.remove::<C::Target>();
                    },
                ));
            });
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
