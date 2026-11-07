//! Tools for waiting for collections of assets to load more easily.
//!
//! These tools could be used to create loading screens, or to ensure your game does not change
//! states until certain assets are loaded. Importantly, **this is not limited** to actually
//! creating a loading screen, and in fact, this module does not provide anything for visually
//! displaying things on the screen. For example, you may choose to have a "narrow gap" that your
//! character needs to squeeze through to enter the next area, and the animation speed may be tied
//! to the progress of loading. While this isn't a traditional loading screen, we still don't want
//! our game to progress any further until those assets are loaded.

use core::any::TypeId;

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashSet},
    lifecycle::HookContext,
    query::{Has, With},
    schedule::{IntoScheduleConfigs, SystemSet},
    system::{Commands, Populated, Query, Res},
    world::{DeferredWorld, FromWorld, World},
};
use bevy_platform::collections::HashSet;
use tracing::warn;

use crate::{
    handle_internal_asset_events, Asset, AssetPath, AssetServer, AssetTrackingSystems, Handle,
    UntypedHandle,
};

/// A plugin for managing the [`AwaitLoads`] component.
pub(crate) struct AwaitLoadsPlugin;

impl Plugin for AwaitLoadsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (poll_pending_loads, poll_pending_dependencies)
                .chain()
                .in_set(AwaitLoadsSystems),
        )
        .configure_sets(
            PreUpdate,
            AwaitLoadsSystems
                .after(AssetTrackingSystems)
                .after(handle_internal_asset_events),
        );
    }
}

#[derive(SystemSet, PartialEq, Eq, Debug, Hash, Clone)]
pub struct AwaitLoadsSystems;

/// A builder for constructing an [`AwaitLoads`] component.
///
/// This acts as a wrapper around [`AssetServer`] that keeps tracks of the assets you've loaded, so
/// you can wait on them later.
pub struct AwaitLoadsBuilder {
    /// The asset server to load assets from.
    asset_server: AssetServer,
    /// The handles that were previously loaded.
    handles: HashSet<UntypedHandle>,
}

impl AwaitLoadsBuilder {
    /// Creates a new builder using the provided [`AssetServer`] to load assets.
    pub fn new(asset_server: AssetServer) -> Self {
        Self {
            asset_server,
            handles: HashSet::default(),
        }
    }

    /// Loads an asset and returns its handle.
    ///
    /// The asset will be tracked internally and will be added as an asset that should be waited for
    /// by the [`AwaitLoads`] produced by [`Self::build`].
    pub fn load<'a, A: Asset>(&mut self, path: impl Into<AssetPath<'a>>) -> Handle<A> {
        let handle = self.asset_server.load::<A>(path);
        self.handles.insert(handle.clone().untyped());
        handle
    }

    /// Loads an asset from its [`TypeId`] and returns its (erased) handle.
    ///
    /// The asset will be tracked internally and will be added as an asset that should be waited for
    /// by the [`AwaitLoads`] produced by [`Self::build`].
    pub fn load_erased<'a>(
        &mut self,
        type_id: TypeId,
        path: impl Into<AssetPath<'a>>,
    ) -> UntypedHandle {
        let handle = self
            .asset_server
            .load_builder()
            .load_erased(type_id, path.into());
        self.handles.insert(handle.clone());
        handle
    }

    /// Constructs an [`AwaitLoads`] component that can be spawned from the assets tracked by this
    /// builder.
    pub fn build(self) -> AwaitLoads {
        AwaitLoads(self.handles)
    }
}

impl FromWorld for AwaitLoadsBuilder {
    fn from_world(world: &mut World) -> Self {
        Self::new(world.resource::<AssetServer>().clone())
    }
}

impl From<AssetServer> for AwaitLoadsBuilder {
    fn from(value: AssetServer) -> Self {
        Self::new(value)
    }
}

// This impl seems redundant, but it has one advantage: it **consumes** the `Res<AssetServer>`,
// making it no longer usable. This allows you to make sure that you don't accidentally use the
// `AssetServer` directly in a system (compared to cloning the `AssetServer`, which could allow you
// to keep using the original `AssetServer`).
impl From<Res<'_, AssetServer>> for AwaitLoadsBuilder {
    fn from(value: Res<AssetServer>) -> Self {
        Self::new(value.clone())
    }
}

/// A component that tracks the loading progress of assets.
///
/// Once all handles have been loaded, the entity with this component will insert the
/// [`LoadFinished`] component. You can use observers to react to the insertion of this component
/// (and therefore, reacting to the load finishing).
#[derive(Component, Default, Clone, Debug)]
#[component(immutable, on_insert=on_insert_await_loads, on_discard=on_discard_await_loads)]
#[require(LoadableDependency)]
pub struct AwaitLoads(
    /// The handles for which this component should wait before inserting the [`LoadFinished`]
    /// component.
    ///
    /// Note: these handles are expected to be loaded through the [`AssetServer`]. Handles created
    /// in other ways, (e.g., [`Assets::reserve_handle`], [`Assets::add`]) will never be considered
    /// loaded.
    ///
    /// [`Assets::reserve_handle`]: crate::Assets::reserve_handle
    /// [`Assets::add`]: crate::Assets::add
    pub HashSet<UntypedHandle>,
);

/// A component that tracks the loading progress of other [`LoadableDependency`] components.
///
/// Once all dependencies have been loaded, the entity with this component will insert the
/// [`LoadFinished`] component (unless [`InsertDependenciesFinished`] is on this entity). You can
/// use observers to react to the insertion of this component (and therefore, reacting to the load
/// finishing).
#[derive(Component, Default, Clone, Debug)]
#[component(immutable, on_insert=on_insert_await_dependencies, on_discard=on_discard_await_dependencies)]
#[require(LoadableDependency)]
pub struct AwaitDependencies(
    /// The dependencies that this component should wait on to be loaded.
    ///
    /// The entities in this set are expected to have the [`LoadableDependency`] component. This is
    /// automatically added by [`AwaitLoads`], and also by [`Self`] (allowing you to await on
    /// another instance of [`Self`]). Once an entity in this set includes the [`LoadFinished`]
    /// component, it is considered loaded and will not be waited for any longer.
    pub EntityHashSet,
);

/// A component to mark that the [`AwaitDependencies`] component should insert
/// [`DependenciesFinished`] instead of [`LoadFinished`].
///
/// This allows you to manually control when the entity is marked loaded (by manually inserting the
/// [`LoadFinished`] component), while still being able to detect that the entity's dependencies
/// have loaded (by checking if the entity has the [`DependenciesFinished`] component).
#[derive(Component, Default)]
pub struct InsertDependenciesFinished;

/// A marker to indicate that an entity is a dependency that may load eventually.
///
/// This entity is considered "loaded" once the [`LoadFinished`] component is inserted onto it.
///
/// This marker is most often used indirectly through [`AwaitLoads`].
#[derive(Component, Default)]
pub struct LoadableDependency;

/// A marker component indicating that a [`LoadableDependency`] (usually added through the
/// [`AwaitLoads`] component) has finished loading.
#[derive(Component, Default)]
pub struct LoadFinished;

/// A marker component indicating that dependencies of an [`AwaitDependencies`] are all loaded.
///
/// This allows you to add behavior that happens after all dependencies have loaded, but before this
/// [`LoadableDependency`] has finished loading. For example, you could wait on all dependencies to
/// load, then start polling for some other condition, after which you manually insert the
/// [`LoadFinished`] component.
///
/// After this component has been inserted, it's up to the user to insert the [`LoadFinished`]
/// component themselves.
#[derive(Component, Default)]
pub struct DependenciesFinished;

/// Hook to set up the [`AwaitLoads`] component to start waiting.
fn on_insert_await_loads(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(await_loads) = world.entity(entity).get::<AwaitLoads>() else {
        return;
    };
    let pending_loads = PendingLoads(await_loads.0.clone());
    world.commands().entity(entity).insert(pending_loads);
}

/// Hook to clear related components to [`AwaitLoads`].
fn on_discard_await_loads(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world
        .commands()
        .entity(entity)
        .try_remove::<(PendingLoads, LoadFinished, LoadableDependency)>();
}

/// Hook to set up the [`AwaitDependencies`] component to start waiting.
fn on_insert_await_dependencies(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(await_dependencies) = world.entity(entity).get::<AwaitDependencies>() else {
        return;
    };
    let pending_dependencies = PendingDependencies(await_dependencies.0.clone());
    world.commands().entity(entity).insert(pending_dependencies);
}

/// Hook to clear related components to [`AwaitDependencies`].
fn on_discard_await_dependencies(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
) {
    world
        .commands()
        .entity(entity)
        .try_remove::<(PendingDependencies, LoadFinished, LoadableDependency)>();
}

/// Component tracking the outstanding asset handles for an [`AwaitLoads`] component.
#[derive(Component)]
struct PendingLoads(
    /// The handles that still haven't loaded.
    HashSet<UntypedHandle>,
);

/// Component tracking the outstanding dependencies for an [`AwaitDependencies`] component.
#[derive(Component)]
struct PendingDependencies(
    /// The dependencies that still haven't loaded.
    EntityHashSet,
);

/// A system for polling whether an [`AwaitLoads`] component is finished loading.
fn poll_pending_loads(
    mut pending: Populated<(Entity, &mut PendingLoads)>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for (entity, mut pending) in pending.iter_mut() {
        pending
            .0
            .retain(|handle| !asset_server.is_loaded_with_dependencies(handle));
        if pending.0.is_empty() {
            commands
                .entity(entity)
                .remove::<PendingLoads>()
                .insert(LoadFinished);
        }
    }
}

/// A system for polling whether an [`AwaitDependencies`] component is finished loading.
fn poll_pending_dependencies(
    mut pending: Populated<(
        Entity,
        &mut PendingDependencies,
        Has<InsertDependenciesFinished>,
    )>,
    is_finished: Query<Has<LoadFinished>, With<LoadableDependency>>,
    mut commands: Commands,
) {
    for (entity, mut pending, insert_dependencies_finished) in pending.iter_mut() {
        pending.0.retain(|dependency| {
            let Ok(is_finished) = is_finished.get(*dependency) else {
                warn!(
                    "AwaitDependencies component is waiting on entity {dependency:?}, but the \
                    entity is not a `LoadableDependency`. Ignoring this dependency."
                );
                return false;
            };
            !is_finished
        });
        // TODO: This check isn't great for large dependency trees. We have to wait a frame between
        // when the dependency finishes and the dependent realizes and becomes loaded. For deep
        // trees, this could incur many frames of delay! We could consider instead computing a
        // toposort of dependencies and then processing dependencies in order, or using observers
        // to listen for when the dependent loads, or just making our own sad version of
        // many-to-many relationships so the child can immediately notify the parent.
        if pending.0.is_empty() {
            let mut entity = commands.entity(entity);
            entity.remove::<PendingDependencies>();
            if insert_dependencies_finished {
                entity.insert(DependenciesFinished);
            } else {
                entity.insert(LoadFinished);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use bevy_app::App;
    use bevy_ecs::{
        entity::{Entity, EntityHashSet},
        world::FromWorld,
    };

    use crate::{
        await_loads::{
            AwaitDependencies, AwaitLoadsBuilder, DependenciesFinished, InsertDependenciesFinished,
            LoadFinished,
        },
        io::memory::Dir,
        tests::{create_app_with_gate, run_app_until, TestAsset, TrivialLoader},
        AssetApp, AssetServer,
    };

    fn is_load_finished(app: &App, entity: Entity) -> bool {
        app.world().entity(entity).contains::<LoadFinished>()
    }

    #[test]
    fn waits_for_assets() {
        let dir = Dir::default();

        let (mut app, gate_opener) = create_app_with_gate(dir.clone());
        app.init_asset::<TestAsset>()
            .register_asset_loader(TrivialLoader);

        let asset_server = app.world().resource::<AssetServer>().clone();

        dir.insert_asset_text(Path::new("a.txt"), "");
        dir.insert_asset_text(Path::new("b.txt"), "");
        dir.insert_asset_text(Path::new("c.txt"), "");

        let mut load_builder = AwaitLoadsBuilder::from_world(app.world_mut());
        let a = load_builder.load::<TestAsset>("a.txt");
        let b = load_builder.load::<TestAsset>("b.txt");
        let c = load_builder.load::<TestAsset>("c.txt");

        let await_loads = app.world_mut().spawn(load_builder.build()).id();

        app.update();

        assert!(!is_load_finished(&app, await_loads));

        gate_opener.open("a.txt");
        run_app_until(&mut app, |_| asset_server.is_loaded(&a).then_some(()));

        // Still not loaded.
        assert!(!is_load_finished(&app, await_loads));

        gate_opener.open("b.txt");
        run_app_until(&mut app, |_| asset_server.is_loaded(&b).then_some(()));

        // Still not loaded.
        assert!(!is_load_finished(&app, await_loads));

        gate_opener.open("c.txt");
        run_app_until(&mut app, |_| asset_server.is_loaded(&c).then_some(()));

        // Finally loaded!!
        assert!(is_load_finished(&app, await_loads));
    }

    #[test]
    fn waits_for_dependencies() {
        let dir = Dir::default();

        let (mut app, gate_opener) = create_app_with_gate(dir.clone());
        app.init_asset::<TestAsset>()
            .register_asset_loader(TrivialLoader);

        let asset_server = app.world().resource::<AssetServer>().clone();

        dir.insert_asset_text(Path::new("a.txt"), "");
        dir.insert_asset_text(Path::new("b.txt"), "");

        let mut child_1 = AwaitLoadsBuilder::new(asset_server.clone());
        let a = child_1.load::<TestAsset>("a.txt");
        let child_1 = app.world_mut().spawn(child_1.build()).id();

        let mut child_2 = AwaitLoadsBuilder::new(asset_server.clone());
        let b = child_2.load::<TestAsset>("b.txt");
        let child_2 = app.world_mut().spawn(child_2.build()).id();

        let parent = app
            .world_mut()
            .spawn(AwaitDependencies(EntityHashSet::from_iter([
                child_1, child_2,
            ])))
            .id();

        app.update();

        assert!(!is_load_finished(&app, child_1));
        assert!(!is_load_finished(&app, child_2));
        assert!(!is_load_finished(&app, parent));

        gate_opener.open("a.txt");
        run_app_until(&mut app, |_| asset_server.is_loaded(&a).then_some(()));

        // Parent is still waiting on child_2.
        assert!(is_load_finished(&app, child_1));
        assert!(!is_load_finished(&app, child_2));
        assert!(!is_load_finished(&app, parent));

        gate_opener.open("b.txt");
        run_app_until(&mut app, |_| asset_server.is_loaded(&b).then_some(()));

        // We may need to run one more iteration in case child_2 resolves after parent (so the
        // parent won't see the child state until the next update). This isn't desirable behavior,
        // it's just a consequence of the current implementation.
        app.update();

        // Finally loaded!!
        assert!(is_load_finished(&app, child_1));
        assert!(is_load_finished(&app, child_2));
        assert!(is_load_finished(&app, parent));
    }

    #[test]
    fn waits_for_dependencies_then_manual() {
        let dir = Dir::default();

        let (mut app, gate_opener) = create_app_with_gate(dir.clone());
        app.init_asset::<TestAsset>()
            .register_asset_loader(TrivialLoader);

        let asset_server = app.world().resource::<AssetServer>().clone();

        dir.insert_asset_text(Path::new("a.txt"), "");

        let mut bottom = AwaitLoadsBuilder::new(asset_server.clone());
        let a = bottom.load::<TestAsset>("a.txt");
        let bottom = app.world_mut().spawn(bottom.build()).id();

        let middle = app
            .world_mut()
            .spawn((
                AwaitDependencies(EntityHashSet::from_iter([bottom])),
                InsertDependenciesFinished,
            ))
            .id();

        let top = app
            .world_mut()
            .spawn(AwaitDependencies(EntityHashSet::from_iter([middle])))
            .id();

        app.update();

        assert!(!is_load_finished(&app, bottom));
        assert!(!is_load_finished(&app, middle));
        assert!(!is_load_finished(&app, top));

        gate_opener.open("a.txt");
        run_app_until(&mut app, |_| asset_server.is_loaded(&a).then_some(()));

        // Additional load to prove this isn't succeeding because of some execution ordering.
        app.update();

        // The bottom has loaded, but the bottom and top still haven't been marked loaded.
        assert!(is_load_finished(&app, bottom));
        assert!(!is_load_finished(&app, middle));
        assert!(!is_load_finished(&app, top));
        // Middle has been notified that its dependencies have finished loading.
        assert!(app
            .world()
            .entity(middle)
            .contains::<DependenciesFinished>());

        app.world_mut().entity_mut(middle).insert(LoadFinished);

        app.update();

        assert!(is_load_finished(&app, bottom));
        assert!(is_load_finished(&app, middle));
        assert!(is_load_finished(&app, top));
    }
}
