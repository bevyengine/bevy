//! Tools for waiting for collections of assets to load more easily.
//!
//! These tools could be used to create loading screens, or to ensure your game does not change
//! states until certain assets are loaded. Importantly, **this is not limited** to actually
//! creating a loading screen, and in fact, this module does not provide anything for visually
//! displaying things on the screen. For example, you may choose to have a "narrow gap" that your
//! character needs to squeeze through to enter the next area, and the animation speed may be tied
//! to the progress of loading. While this isn't a traditional loading screen, we still don't want
//! our game to progress any further until those assets are loaded.
//!
//! # Architecture
//!
//! Entities that are considered "loadable" are those with the [`LoadableDependency`] and an entity
//! is considered loaded if it has the [`LoadFinished`] component. In fact, the only notification
//! given to users that an entity is loaded is by the insertion of the [`LoadFinished`] component.
//! You can use observers to react to [`LoadFinished`] insertion.
//!
//! Users will primarily interact with higher-level components like [`AwaitLoads`] and
//! [`AwaitDependencies`] (as opposed to interacting with [`LoadableDependency`]). These components
//! include [`LoadableDependency`] automatically as a required component.
//!
//! [`AwaitLoads`] allows you to wait on a set of asset handles. [`AwaitDependencies`] allows you to
//! wait on a set of [`LoadableDependency`] entities (which includes both [`AwaitLoads`] and
//! [`AwaitDependencies`] entities).
//!
//! You may also create your own higher-level components by manually inserting
//! [`LoadableDependency`] and [`LoadFinished`] as appropriate.
//!
//! # Patterns
//!
//! The architecture is very flexible, allowing for all sorts of ways to use the API. Provided are
//! some ideas on how to use this API in an ergonomic way for some use cases:
//!
//! ## Load and spawn a level
//!
//! The most common situation is loading all assets for a level, and then spawning that level. There
//! are many ways to achieve this. One that seems to work well is to create an
//! [`AwaitLoadsBuilder`] and load all the handles you need, then in the same system, spawn the
//! built [`AwaitLoads`] with an observer moving those handles into itself.
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # use bevy_asset::{prelude::*, await_loads::*};
//! # #[derive(Component)]
//! # struct WorldAssetRoot(Handle<()>);
//! # #[derive(Component)]
//! # struct Transform;
//! # impl Transform { fn from_xyz(x: f32, y: f32, z: f32) -> Self { Self } }
//! fn my_startup_system(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     let mut load_builder = AwaitLoadsBuilder::from(asset_server);
//!
//!     let house_world_asset = load_builder.load("house.gltf#Scene0");
//!     let tree_world_asset = load_builder.load("tree.gltf#Scene0");
//!     let player_world_asset = load_builder.load("player.gltf#Scene0");
//!
//!     commands.spawn(load_builder.build()).observe(
//!         move |event: On<Insert<LoadFinished>>, mut commands: Commands| {
//!             commands.spawn((
//!                 WorldAssetRoot(house_world_asset.clone()),
//!                 Transform::from_xyz(0.0, 0.0, 10.0),
//!             ));
//!
//!             commands.spawn((
//!                 WorldAssetRoot(tree_world_asset.clone()),
//!                 Transform::from_xyz(3.0, 0.0, 3.0),
//!             ));
//!
//!             commands.spawn(WorldAssetRoot(player_world_asset.clone()));
//!
//!             // This isn't necessary, but despawning prevents these loader entities from piling
//!             // up if you use this pattern many times.
//!             commands.entity(event.entity).despawn();
//!         }
//!     );
//! }
//! ```
//!
//! This allows type-inference to determine the correct type for the handles, and allows you to
//! spawn fairly normally (at the cost of some indentation).
//!
//! ## Global loader and wait for all assets
//!
//! When starting a game, you may not want to have such fine-grained loading and spawning; you may
//! instead prefer to load assets from many systems, and then wait for all of them to load. This can
//! be achieved by sharing an [`AwaitLoadsBuilder`]. Here is an example of such an approach:
//!
//! ```rust
//! # use bevy_ecs::prelude::*;
//! # use bevy_app::prelude::*;
//! # use bevy_asset::{prelude::*, await_loads::*};
//! #[derive(Resource)]
//! struct InitialLoadAssets(AwaitLoadsBuilder);
//!
//! impl FromWorld for InitialLoadAssets {
//!     fn from_world(world: &mut World) -> Self {
//!         Self(AwaitLoadsBuilder::new(world.resource::<AssetServer>().clone()))
//!     }
//! }
//!
//! fn system_1(mut initial_loads: Res<InitialLoadAssets>) { /*Do something here*/ }
//! fn system_2(mut initial_loads: Res<InitialLoadAssets>) { /*Do something here*/ }
//!
//! fn spawn_await_loads(world: &mut World) {
//!     let await_loads = world.remove_resource::<InitialLoadAssets>().unwrap().0.build();
//!     world.spawn(await_loads).observe(|_: On<Insert<LoadFinished>>, mut commands: Commands| {
//!         // We can signal this by setting a state...
//!         //   commands.set_state(MyState::Loaded);
//!         // Or maybe running a schedule...
//!         //   commands.run_schedule(GameLoaded);
//!         // Or whatever else!
//!     });
//! }
//!
//! fn my_plugin(app: &mut App) {
//!     app
//!         .init_resource::<InitialLoadAssets>()
//!         .add_systems(Startup, (system_1, system_2))
//!         // After all the handles have been registered, spawn the `AwaitLoads` so we wait for
//!         // those assets.
//!         .add_systems(PostStartup, spawn_await_loads);
//! }
//! ```
//!
//! This creates an `InitialLoadAssets` resource that we then share across all our systems. Later,
//! we consume it from the world to create our actual loader. This gives an API similar to the
//! regular [`AssetServer`] API.
//!
//! One disadvantage of this approach is that you now need to somehow get the same asset handles
//! from `system_1` and `system_2` into the code that runs the observer (or you need to reload the
//! exact same asset path and type in the observer code).
//!
//! ## Awaiting several dependencies
//!
//! In some cases, you may want to split assets up into chunks, and then wait for all those chunks
//! to be loaded. For this, you can use [`AwaitDependencies`] like so:
//!
//! ```rust
//! # use bevy_ecs::{prelude::*, entity::EntityHashSet};
//! # use bevy_app::prelude::*;
//! # use bevy_asset::{prelude::*, await_loads::*};
//! # type Mesh = ();
//! # type Shader = ();
//! #[derive(Component)]
//! struct PlayerAssets {
//!     collider: Handle<Mesh>,
//!     body: Handle<Mesh>,
//!     sword: Handle<Mesh>,
//! }
//!
//! #[derive(Component)]
//! struct Shaders {
//!     pbr: Handle<Shader>,
//!     tonemapping: Handle<Shader>,
//! }
//!
//! fn load_player_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     let mut load_builder = AwaitLoadsBuilder::from(asset_server);
//!
//!     let player_assets = PlayerAssets {
//!         collider: load_builder.load("collider.obj"),
//!         body: load_builder.load("body.obj"),
//!         sword: load_builder.load("sword.obj"),
//!     };
//!
//!     // We can store the PlayerAssets right on the loader, making it easy to find the
//!     // `AwaitLoads`, and easy to use the assets!
//!     commands.spawn((player_assets, load_builder.build()));
//! }
//!
//! fn load_shaders(mut commands: Commands, asset_server: Res<AssetServer>) {
//!     let mut load_builder = AwaitLoadsBuilder::from(asset_server);
//!
//!     let shaders = Shaders {
//!         pbr: load_builder.load("pbr.wesl"),
//!         tonemapping: load_builder.load("tony_mc_mapface.wesl"),
//!     };
//!
//!     commands.spawn((shaders, load_builder.build()));
//! }
//!
//! fn spawn_loader(
//!     mut commands: Commands,
//!     player_assets: Single<Entity, With<PlayerAssets>>,
//!     shaders: Single<Entity, With<Shaders>>
//! ) {
//!     commands.spawn(AwaitDependencies(
//!         EntityHashSet::from([
//!             player_assets.into_inner(),
//!             shaders.into_inner(),
//!         ]),
//!     )).observe(|_: On<Insert<LoadFinished>>| {
//!         // Do something here!!
//!     });
//! }
//!
//! fn plugin(app: &mut App) {
//!     app.add_systems(Startup, (load_player_assets, load_shaders, spawn_loader).chain());
//! }
//! ```
//!
//! Here each of our little "asset collections" are independent, and are loaded independently. We
//! can also look them up independently using for example `Single<&PlayerAssets>`. However, using
//! [`AwaitDependencies`], we can still wait on all these assets together, and most importantly, we
//! can share these dependencies. So for example, we can have one [`AwaitDependencies`] for the
//! level load, which waits on the player assets and the level geometry, and another for the
//! character customization screen, which waits on the player assets and the customization assets.
use core::any::TypeId;

use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityHashSet},
    lifecycle::HookContext,
    query::{Has, With, Without},
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
            (
                poll_pending_loads,
                poll_pending_dependencies,
                count_ready_frames,
            )
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

/// A component that waits for [`ReadyFlag`] to be true for some number of frames after all
/// dependencies have loaded.
///
/// For some assets, being loaded doesn't mean that the asset is ready to use. There may be some
/// other processing the asset needs to go through before it's ready to be used. This component
/// allows you to 1) wait for the dependencies to be loaded (e.g., an asset load), then 2) wait for
/// the [`ReadyFlag`] to be `true` for some number of frames. If the [`ReadyFlag`] is set to
/// `false`, the frame counter resets -- this allows for cases where there is some delay between
/// the dependencies being loaded and the signal that this entity **shouldn't** be ready.
///
/// Once all dependencies are loaded, **and** the [`ReadyFlag`] is set to true for the requested
/// number of frames (consecutively), this entity is considered loaded. If the [`ReadyFlag`] then
/// becomes false, this entity will **not** "unload" (the loaded state becomes "locked in").
#[derive(Component, Clone, Debug)]
#[component(immutable, on_remove=on_remove_await_ready_frames)]
#[require(
    LoadableDependency,
    AwaitDependencies,
    InsertDependenciesFinished,
    CurrentReadyFrameCount
)]
pub struct AwaitReadyFrames(pub u32);

/// Component indicating that the entity is ready, for the purposes of [`AwaitReadyFrames`].
///
/// It is up to the user to set this to `true` to indicate that their entity is ready.
#[derive(Component, Default)]
#[component(on_remove=on_remove_ready_flag)]
pub struct ReadyFlag(pub bool);

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

/// Hook to clear related components to [`AwaitReadyFrames`].
fn on_remove_await_ready_frames(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    world
        .commands()
        .entity(entity)
        .try_remove::<CurrentReadyFrameCount>();
}

/// Hook to clear the current frame count if you remove the [`ReadyFlag`] component.
fn on_remove_ready_flag(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Ok(mut entity) = world.get_entity_mut(entity) else {
        return;
    };
    let Some(mut current_frame_count) = entity.get_mut::<CurrentReadyFrameCount>() else {
        return;
    };
    current_frame_count.0 = 0;
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

/// Component counting the number of frames that [`ReadyFlag`] was true (consecutively).
#[derive(Component, Default)]
struct CurrentReadyFrameCount(u32);

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

/// System for updating the [`CurrentReadyFrameCount`] based on the [`ReadyFlag`].
// NOTE: Technically, if someone removes the `DependenciesFinished` component, the ready frame count
// gets "stuck" as opposed to resetting. This is not worth fixing though.
fn count_ready_frames(
    mut counting: Populated<
        (
            Entity,
            &AwaitReadyFrames,
            Option<&ReadyFlag>,
            &mut CurrentReadyFrameCount,
        ),
        (With<DependenciesFinished>, Without<LoadFinished>),
    >,
    mut commands: Commands,
) {
    for (entity, await_frames, ready_flag, mut frame_count) in counting.iter_mut() {
        // If the `ReadyFlag` component is missing, we consider it `false`.
        let ready_flag = ready_flag.map_or_default(|flag| flag.0);
        if !ready_flag {
            // We're not ready. Clear the current frame count.
            frame_count.0 = 0;
            continue;
        }

        frame_count.0 += 1;
        if frame_count.0 >= await_frames.0 {
            // This isn't strictly necessary, but it means if someone removes the `LoadFinished`
            // component manually, the frame count is already reset (rather than instantly becoming
            // loaded again the next frame).
            frame_count.0 = 0;
            commands.entity(entity).insert(LoadFinished);
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
            AwaitDependencies, AwaitLoadsBuilder, AwaitReadyFrames, DependenciesFinished,
            InsertDependenciesFinished, LoadFinished, LoadableDependency, ReadyFlag,
        },
        io::memory::Dir,
        tests::{create_app, create_app_with_gate, run_app_until, TestAsset, TrivialLoader},
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

    #[test]
    fn counts_ready_frames_after_dependencies_loaded() {
        let mut app = create_app().0;

        // This entity is a manual dependency. We can insert the LoadFinished whenever we want.
        let child = app.world_mut().spawn(LoadableDependency).id();
        let parent = app
            .world_mut()
            .spawn((
                AwaitDependencies(EntityHashSet::from([child])),
                AwaitReadyFrames(3),
            ))
            .id();

        app.update();
        app.update();
        app.update();
        app.update();
        app.update();

        // We've waited more than 3 ready frames, but we're still not loaded because our dependency
        // is not loaded.
        assert!(!is_load_finished(&app, parent));

        app.world_mut().entity_mut(child).insert(LoadFinished);
        app.update();

        // The child is loaded now, so the parent should be marked that its dependencies finished.
        assert!(app
            .world()
            .entity(parent)
            .contains::<DependenciesFinished>());
        assert!(!is_load_finished(&app, parent));

        app.update();
        app.update();
        app.update();
        app.update();
        app.update();

        // The parent still isn't loaded because we haven't marked the entity as ready.
        assert!(!is_load_finished(&app, parent));

        app.world_mut().entity_mut(parent).insert(ReadyFlag(true));

        app.update();
        app.update();

        // Reset the ready flag. This should reset the current ready frame count.
        app.world_mut()
            .entity_mut(parent)
            .get_mut::<ReadyFlag>()
            .unwrap()
            .0 = false;

        app.update();
        app.update();
        app.update();
        app.update();
        app.update();

        assert!(!is_load_finished(&app, parent));

        app.world_mut()
            .entity_mut(parent)
            .get_mut::<ReadyFlag>()
            .unwrap()
            .0 = true;

        app.update();
        assert!(!is_load_finished(&app, parent));
        app.update();
        assert!(!is_load_finished(&app, parent));
        app.update();

        // We're finally loaded!
        assert!(is_load_finished(&app, parent));
    }
}
