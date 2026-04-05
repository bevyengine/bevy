use crate::{Scene, SceneList, SceneListPatch, ScenePatch, ScenePatchInstance, SpawnSceneError};
use alloc::sync::Arc;
use bevy_asset::{AssetEvent, AssetServer, Assets, Handle};
use bevy_ecs::{message::MessageCursor, prelude::*, relationship::Relationship};
use bevy_platform::collections::HashMap;
use tracing::error;

/// Adds scene spawning functionality to [`World`].
pub trait WorldSceneExt {
    /// Spawns the given [`Scene`] immediately. This will resolve the Scene (using [`Scene::resolve`]). If that fails (for example, if there are dependencies that have not been
    /// loaded yet), it will return a [`SpawnSceneError`]. If resolving the [`Scene`] is successful, the scene will be spawned.
    ///
    /// If resolving and spawning is successful, it will return a new [`EntityWorldMut`] containing the full contents of the spawned scene.
    ///
    /// See [`Scene`] for the features of the scene system (and how to use it).
    ///
    /// If your scene has a dependency that might not be loaded yet (for example, it inherits from a `.bsn` asset file), consider using [`World::queue_spawn_scene`].
    ///
    /// ```
    /// # use bevy_app::App;
    /// # use bevy_scene::{prelude::*, ScenePlugin};
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::AssetPlugin;
    /// # use bevy_app::TaskPoolPlugin;
    /// # let mut app = App::new();
    /// # app.add_plugins((
    /// #     TaskPoolPlugin::default(),
    /// #     AssetPlugin::default(),
    /// #     ScenePlugin::default(),
    /// # ));
    /// # let world = app.world_mut();
    /// #[derive(Component, Default, Clone)]
    /// struct Score(usize);
    ///
    /// #[derive(Component, Default, Clone)]
    /// struct Sword;
    ///
    /// #[derive(Component, Default, Clone)]
    /// struct Shield;
    ///
    /// world.spawn_scene(bsn! {
    ///     #Player
    ///     Score(0)
    ///     Children [
    ///         Sword,
    ///         Shield,
    ///     ]
    /// }).unwrap();
    /// ```
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> Result<EntityWorldMut<'_>, SpawnSceneError>;

    /// Queues the `scene` to be spawned. This will evaluate the `scene`'s dependencies (via [`Scene::register_dependencies`]) and queue it to be resolved and spawned
    /// after all of the dependencies have been loaded. If a [`SpawnSceneError`] occurs, it will be logged as an error.
    ///
    /// If the dependencies are already loaded (or there are no dependencies), then the scene will be spawned this frame.
    ///
    /// See [`Scene`] for the features of the scene system (and how to use it).
    ///
    /// ```
    /// # use bevy_app::App;
    /// # use bevy_scene::{prelude::*, ScenePlugin};
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::AssetPlugin;
    /// # use bevy_app::TaskPoolPlugin;
    /// # let mut app = App::new();
    /// # app.add_plugins((
    /// #     TaskPoolPlugin::default(),
    /// #     AssetPlugin::default(),
    /// #     ScenePlugin::default(),
    /// # ));
    /// # let world = app.world_mut();
    /// #[derive(Component, Default, Clone)]
    /// struct Score(usize);
    ///
    /// #[derive(Component, Default, Clone)]
    /// struct Sword;
    ///
    /// #[derive(Component, Default, Clone)]
    /// struct Shield;
    ///
    /// // This scene inherits from the "player.bsn" asset. It will be spawned on the frame that "player.bsn"
    /// // is fully loaded.
    /// world.queue_spawn_scene(bsn! {
    ///     :"player.bsn"
    ///     #Player
    ///     Score(0)
    ///     Children [
    ///         Sword,
    ///         Shield,
    ///     ]
    /// });
    /// ```
    fn queue_spawn_scene<S: Scene>(&mut self, scene: S) -> EntityWorldMut<'_>;

    /// Spawns the given [`SceneList`] immediately. This will resolve the scene list (using [`SceneList::resolve_list`]). If that fails (for example, if there are dependencies that have not been
    /// loaded yet), it will return a [`SpawnSceneError`]. If resolving the [`SceneList`] is successful, the scene list will be spawned.
    ///
    /// If resolving and spawning is successful, it will return a [`Vec<Entity>`] containing each entity described in the [`SceneList`].
    ///
    /// See [`Scene`] for the features of the scene system (and how to use it).
    ///
    /// If your scene list has a dependency that might not be loaded yet (for example, it inherits from a `.bsn` asset file), consider using [`World::queue_spawn_scene_list`].
    ///
    /// ```
    /// # use bevy_app::App;
    /// # use bevy_scene::{prelude::*, ScenePlugin};
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::AssetPlugin;
    /// # use bevy_app::TaskPoolPlugin;
    /// # let mut app = App::new();
    /// # app.add_plugins((
    /// #     TaskPoolPlugin::default(),
    /// #     AssetPlugin::default(),
    /// #     ScenePlugin::default(),
    /// # ));
    /// # let world = app.world_mut();
    /// #[derive(Component, FromTemplate)]
    /// enum Team {
    ///     #[default]
    ///     Red,
    ///     Blue,
    /// }
    ///
    /// world.spawn_scene_list(bsn_list! {
    ///     (
    ///         #Player1
    ///         Team::Red
    ///     ),
    ///     (
    ///         #Player2
    ///         Team::Blue
    ///     )
    /// }).unwrap();
    /// ```
    // PERF: ideally this is an iterator
    fn spawn_scene_list<L: SceneList>(&mut self, scenes: L)
        -> Result<Vec<Entity>, SpawnSceneError>;

    /// Queues the `scene_list` to be spawned. This will evaluate the `scene_list`'s dependencies (via [`Scene::register_dependencies`]) and queue it to be resolved
    /// and spawned after all of the dependencies have been loaded. If a [`SpawnSceneError`] occurs, it will be logged as an error.
    ///
    /// If the dependencies are already loaded (or there are no dependencies), then the scene list will be spawned this frame.
    /// ```
    /// # use bevy_app::App;
    /// # use bevy_scene::{prelude::*, ScenePlugin};
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::AssetPlugin;
    /// # use bevy_app::TaskPoolPlugin;
    /// # let mut app = App::new();
    /// # app.add_plugins((
    /// #     TaskPoolPlugin::default(),
    /// #     AssetPlugin::default(),
    /// #     ScenePlugin::default(),
    /// # ));
    /// # let world = app.world_mut();
    /// #[derive(Component, FromTemplate)]
    /// enum Team {
    ///     #[default]
    ///     Red,
    ///     Blue,
    /// }
    /// // This scene list inherits from the "player.bsn" asset. It will be spawned on the frame that "player.bsn"
    /// // is loaded.
    /// world.queue_spawn_scene_list(bsn_list! [
    ///     (
    ///         :"player.bsn"
    ///         #Player1
    ///         Team::Red
    ///     ),
    ///     (
    ///         :"player.bsn"
    ///         #Player2
    ///         Team::Blue
    ///     )
    /// ]);
    /// ```
    fn queue_spawn_scene_list<L: SceneList>(&mut self, scenes: L);
}

impl WorldSceneExt for World {
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> Result<EntityWorldMut<'_>, SpawnSceneError> {
        let assets = self.resource::<AssetServer>();
        let mut patch = ScenePatch::load(assets, scene);
        patch.resolve(assets, self.resource::<Assets<ScenePatch>>())?;
        patch.spawn(self)
    }

    fn queue_spawn_scene<S: Scene>(&mut self, scene: S) -> EntityWorldMut<'_> {
        let assets = self.resource::<AssetServer>();
        let patch = ScenePatch::load(assets, scene);
        let handle = assets.add(patch);
        self.spawn(ScenePatchInstance(handle))
    }

    fn spawn_scene_list<L: SceneList>(
        &mut self,
        scenes: L,
    ) -> Result<Vec<Entity>, SpawnSceneError> {
        let assets = self.resource::<AssetServer>();
        let mut patch = SceneListPatch::load(assets, scenes);
        patch.resolve(assets, self.resource::<Assets<ScenePatch>>())?;
        patch.spawn(self)
    }

    fn queue_spawn_scene_list<L: SceneList>(&mut self, scenes: L) {
        let assets = self.resource::<AssetServer>();
        let patch = SceneListPatch::load(assets, scenes);
        let handle = assets.add(patch);
        self.resource_mut::<QueuedScenes>()
            .scene_list_spawns
            .push(handle);
    }
}

/// Adds scene spawning functionality to [`Commands`].
pub trait CommandsSceneExt {
    /// Spawns the given [`Scene`] as soon as [`Commands`] are applied. This will resolve the Scene (using [`Scene::resolve`]). If that fails (for example, if there are dependencies that have not been
    /// loaded yet), it will log a [`SpawnSceneError`] as an error. If resolving the [`Scene`] is successful, the scene will be spawned.
    ///
    /// This is essentially a [`Command`] that runs [`World::spawn_scene`].
    ///
    /// See [`Scene`] for the features of the scene system (and how to use it).
    ///
    /// If your scene has a dependency that might not be loaded yet (for example, it inherits from a `.bsn` asset file), consider using [`Commands::queue_spawn_scene`].
    ///
    /// ```
    /// # use bevy_scene::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let mut commands = world.commands();
    /// #[derive(Component, Default, Clone)]
    /// struct Score(usize);
    ///
    /// #[derive(Component, Default, Clone)]
    /// struct Sword;
    ///
    /// #[derive(Component, Default, Clone)]
    /// struct Shield;
    ///
    /// commands.spawn_scene(bsn! {
    ///     #Player
    ///     Score(0)
    ///     Children [
    ///         Sword,
    ///         Shield,
    ///     ]
    /// });
    /// ```
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> EntityCommands<'_>;

    /// Queues the `scene` to be spawned. This will evaluate the `scene`'s dependencies (via [`Scene::register_dependencies`]) and queue it to be resolved and spawned
    /// after all of the dependencies have been loaded. If a [`SpawnSceneError`] occurs, it will be logged as an error.
    ///
    /// If the dependencies are already loaded (or there are no dependencies), then the scene will be spawned this frame.
    ///
    /// See [`Scene`] for the features of the scene system (and how to use it).
    ///
    /// ```
    /// # use bevy_scene::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let mut commands = world.commands();
    /// #[derive(Component, Default, Clone)]
    /// struct Score(usize);
    ///
    /// #[derive(Component, Default, Clone)]
    /// struct Sword;
    ///
    /// #[derive(Component, Default, Clone)]
    /// struct Shield;
    ///
    /// // This scene inherits from the "player.bsn" asset. It will be spawned on the frame that "player.bsn"
    /// // is fully loaded.
    /// commands.queue_spawn_scene(bsn! {
    ///     :"player.bsn"
    ///     #Player
    ///     Score(0)
    ///     Children [
    ///         Sword,
    ///         Shield,
    ///     ]
    /// });
    /// ```
    fn queue_spawn_scene<S: Scene>(&mut self, scene: S) -> EntityCommands<'_>;

    /// Spawns the given [`SceneList`] as soon as [`Commands`] are applied. This will resolve the scene list (using [`SceneList::resolve_list`]). If that fails (for example, if there are dependencies that have not been
    /// loaded yet), it will log a [`SpawnSceneError`] as an error. If resolving the [`Scene`] is successful, the scene list will be spawned.
    ///
    /// This is essentially a [`Command`] that performs [`World::spawn_scene_list`].
    ///
    /// See [`Scene`] for the features of the scene system (and how to use it).
    ///
    /// If your scene list has a dependency that might not be loaded yet (for example, it inherits from a `.bsn` asset file), consider using [`Commands::queue_spawn_scene_list`].
    ///
    /// ```
    /// # use bevy_scene::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let mut commands = world.commands();
    /// #[derive(Component, FromTemplate)]
    /// enum Team {
    ///     #[default]
    ///     Red,
    ///     Blue,
    /// }
    ///
    /// commands.spawn_scene_list(bsn_list! {
    ///     (
    ///         :"player.bsn"
    ///         #Player1
    ///         Team::Red
    ///     ),
    ///     (
    ///         :"player.bsn"
    ///         #Player2
    ///         Team::Blue
    ///     )
    /// });
    /// ```
    fn spawn_scene_list<L: SceneList>(&mut self, scenes: L);

    /// Queues the `scene_list` to be spawned. This will evaluate the `scene_list`'s dependencies (via [`Scene::register_dependencies`]) and queue it to be resolved
    /// and spawned after all of the dependencies have been loaded. If a [`SpawnSceneError`] occurs, it will be logged as an error.
    ///
    /// If the dependencies are already loaded (or there are no dependencies), then the scene will be spawned this frame.
    ///
    /// ```
    /// # use bevy_scene::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let mut commands = world.commands();
    /// #[derive(Component, FromTemplate)]
    /// enum Team {
    ///     #[default]
    ///     Red,
    ///     Blue,
    /// }
    ///
    /// // This scene list inherits from the "player.bsn" asset. It will be spawned on the frame that "player.bsn"
    /// // is loaded.
    /// commands.queue_spawn_scene_list(bsn_list! [
    ///     (
    ///         :"player.bsn"
    ///         #Player1
    ///         Team::Red
    ///     ),
    ///     (
    ///         :"player.bsn"
    ///         #Player2
    ///         Team::Blue
    ///     )
    /// ]);
    /// ```
    fn queue_spawn_scene_list<L: SceneList>(&mut self, scenes: L);
}

impl<'w, 's> CommandsSceneExt for Commands<'w, 's> {
    fn spawn_scene<S: Scene>(&mut self, scene: S) -> EntityCommands<'_> {
        let mut entity_commands = self.spawn_empty();
        let id = entity_commands.id();
        entity_commands.commands().queue(move |world: &mut World| {
            if let Ok(mut entity) = world.get_entity_mut(id)
                && let Err(err) = entity.apply_scene(scene)
            {
                error!("{err}");
            }
        });
        entity_commands
    }

    fn queue_spawn_scene<S: Scene>(&mut self, scene: S) -> EntityCommands<'_> {
        let mut entity_commands = self.spawn_empty();
        let id = entity_commands.id();
        entity_commands.commands().queue(move |world: &mut World| {
            if let Ok(mut entity) = world.get_entity_mut(id) {
                entity.queue_apply_scene(scene);
            }
        });
        entity_commands
    }

    fn spawn_scene_list<L: SceneList>(&mut self, scenes: L) {
        self.queue(move |world: &mut World| {
            if let Err(err) = world.spawn_scene_list(scenes) {
                error!("{err}");
            }
        });
    }

    fn queue_spawn_scene_list<L: SceneList>(&mut self, scenes: L) {
        self.queue(move |world: &mut World| {
            world.queue_spawn_scene_list(scenes);
        });
    }
}

/// Adds scene functionality to [`EntityWorldMut`].
pub trait EntityWorldMutSceneExt {
    /// Spawns a [`SceneList`], where each entity is related to the current entity using [`RelationshipTarget::Relationship`].
    ///
    /// This will evaluate the `scene_list`'s dependencies (via [`SceneList::register_dependencies`]) and queue it to be resolved
    /// and spawned after all of the dependencies have been loaded. If a [`SpawnSceneError`] occurs, it will be logged as an error.
    ///
    /// If the dependencies are already loaded (or there are no dependencies), then the scene list will be spawned this frame.
    ///
    /// ```
    /// # use bevy_app::App;
    /// # use bevy_scene::{prelude::*, ScenePlugin};
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::AssetPlugin;
    /// # use bevy_app::TaskPoolPlugin;
    /// # let mut app = App::new();
    /// # app.add_plugins((
    /// #     TaskPoolPlugin::default(),
    /// #     AssetPlugin::default(),
    /// #     ScenePlugin::default(),
    /// # ));
    /// # let world = app.world_mut();
    /// #[derive(Component, FromTemplate)]
    /// enum Team {
    ///     #[default]
    ///     Red,
    ///     Blue,
    /// }
    ///
    /// world.spawn_empty().queue_spawn_related_scenes::<Children>(bsn_list! {
    ///     (
    ///         #Player1
    ///         Team::Red
    ///     ),
    ///     (
    ///         #Player2
    ///         Team::Blue
    ///     )
    /// });
    /// ```
    fn queue_spawn_related_scenes<T: RelationshipTarget>(self, scenes: impl SceneList) -> Self;

    /// Applies the given [`Scene`] to the current entity immediately. This will resolve the Scene (using [`Scene::resolve`]). If that fails (for example, if there are dependencies that have not been
    /// loaded yet), it will return a [`SpawnSceneError`]. If resolving the [`Scene`] is successful, the scene will be spawned.
    ///
    /// If resolving and spawning is successful, the entity will contain the full contents of the spawned scene.
    ///
    /// This will write directly on top of any existing components on the entity. [`Scene`] is generally used as a spawning mechanism, so for most things, prefer using [`World::spawn_scene`].
    ///
    /// See [`Scene`] for the features of the scene system (and how to use it).
    ///
    /// If your scene has a dependency that might not be loaded yet (for example, it inherits from a `.bsn` asset file), consider using [`World::queue_spawn_scene`].
    fn apply_scene<S: Scene>(&mut self, scene: S) -> Result<(), SpawnSceneError>;

    /// Queues the `scene` to be applied. This will evaluate the `scene`'s dependencies (via [`Scene::register_dependencies`]) and queue it to be resolved and spawned
    /// after all of the dependencies have been loaded. If a [`SpawnSceneError`] occurs, it will be logged as an error.
    ///
    /// If the dependencies are already loaded (or there are no dependencies), then the scene will be spawned this frame.
    /// This will write directly on top of any existing components on the entity. [`Scene`] is generally used as a spawning mechanism, so for most things, prefer using [`World::queue_spawn_scene`].
    ///
    /// See [`Scene`] for the features of the scene system (and how to use it).
    fn queue_apply_scene<S: Scene>(&mut self, scene: S);
}

impl EntityWorldMutSceneExt for EntityWorldMut<'_> {
    fn queue_spawn_related_scenes<T: RelationshipTarget>(mut self, scenes: impl SceneList) -> Self {
        let assets = self.resource::<AssetServer>();
        let patch = SceneListPatch::load(assets, scenes);
        let handle = assets.add(patch);
        let entity = self.id();
        self.resource_mut::<QueuedScenes>()
            .related_scene_list_spawns
            .push((
                RelatedSceneListSpawn {
                    entity,
                    insert: |entity, target| {
                        entity.insert(
                            <<T as RelationshipTarget>::Relationship as Relationship>::from(target),
                        );
                    },
                },
                handle,
            ));
        self
    }

    fn apply_scene<S: Scene>(&mut self, scene: S) -> Result<(), SpawnSceneError> {
        let assets = self.resource::<AssetServer>();
        let mut patch = ScenePatch::load(assets, scene);
        patch.resolve(assets, self.resource::<Assets<ScenePatch>>())?;
        patch.apply(self)
    }

    fn queue_apply_scene<S: Scene>(&mut self, scene: S) {
        let assets = self.resource::<AssetServer>();
        let patch = ScenePatch::load(assets, scene);
        let handle = assets.add(patch);
        self.insert(ScenePatchInstance(handle));
    }
}

/// Adds scene functionality to [`EntityWorldMut`].
pub trait EntityCommandsSceneExt {
    /// Spawns a [`SceneList`], where each entity is related to the current entity using [`RelationshipTarget::Relationship`].
    ///
    /// This will evaluate the `scene_list`'s dependencies (via [`SceneList::register_dependencies`]) and queue it to be resolved
    /// and spawned after all of the dependencies have been loaded. If a [`SpawnSceneError`] occurs, it will be logged as an error.
    ///
    /// If the dependencies are already loaded (or there are no dependencies), then the scene list will be spawned this frame.
    ///
    /// ```
    /// # use bevy_app::App;
    /// # use bevy_scene::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_asset::AssetPlugin;
    /// # use bevy_app::TaskPoolPlugin;
    /// # let mut app = App::new();
    /// # let mut commands = app.world_mut().commands();
    /// #[derive(Component, FromTemplate)]
    /// enum Team {
    ///     #[default]
    ///     Red,
    ///     Blue,
    /// }
    ///
    /// commands.spawn_empty().queue_spawn_related_scenes::<Children>(bsn_list! {
    ///     (
    ///         #Player1
    ///         Team::Red
    ///     ),
    ///     (
    ///         #Player2
    ///         Team::Blue
    ///     )
    /// });
    /// ```
    fn queue_spawn_related_scenes<T: RelationshipTarget>(
        &mut self,
        scenes: impl SceneList,
    ) -> &mut Self;

    /// Applies the given [`Scene`] to the current entity as soon as [`Commands`] are applied. This will resolve the Scene (using [`Scene::resolve`]). If that fails (for example, if there are dependencies that have not been
    /// loaded yet), it will log a [`SpawnSceneError`] as an error. If resolving the [`Scene`] is successful, the scene will be spawned.
    ///
    /// If resolving and spawning is successful, the entity will contain the full contents of the spawned scene.
    ///
    /// This will write directly on top of any existing components on the entity. [`Scene`] is generally used as a spawning mechanism, so for most things, prefer using [`Commands::spawn_scene`].
    ///
    /// See [`Scene`] for the features of the scene system (and how to use it).
    ///
    /// If your scene has a dependency that might not be loaded yet (for example, it inherits from a `.bsn` asset file), consider using [`Commands::spawn_scene`].
    fn apply_scene<S: Scene>(&mut self, scene: S) -> &mut Self;

    /// Queues the `scene` to be applied. This will evaluate the `scene`'s dependencies (via [`Scene::register_dependencies`]) and queue it to be resolved and spawned
    /// after all of the dependencies have been loaded. If a [`SpawnSceneError`] occurs, it will be logged as an error.
    ///
    /// If the dependencies are already loaded (or there are no dependencies), then the scene will be spawned this frame.
    /// This will write directly on top of any existing components on the entity. [`Scene`] is generally used as a spawning mechanism, so for most things, prefer using [`Commands::queue_spawn_scene`].
    ///
    /// See [`Scene`] for the features of the scene system (and how to use it).
    fn queue_apply_scene<S: Scene>(&mut self, scene: S) -> &mut Self;
}

impl EntityCommandsSceneExt for EntityCommands<'_> {
    fn queue_spawn_related_scenes<T: RelationshipTarget>(
        &mut self,
        scenes: impl SceneList,
    ) -> &mut Self {
        self.queue(move |entity: EntityWorldMut| {
            entity.queue_spawn_related_scenes::<T>(scenes);
        });
        self
    }

    fn apply_scene<S: Scene>(&mut self, scene: S) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut| entity.apply_scene(scene));
        self
    }

    fn queue_apply_scene<S: Scene>(&mut self, scene: S) -> &mut Self {
        self.queue(move |mut entity: EntityWorldMut| entity.queue_apply_scene(scene));
        self
    }
}

/// A [`System`] that resolves [`ScenePatch`] and [`SceneListPatch`] assets whose dependencies have been fully loaded.
pub fn resolve_scene_patches(
    mut events: MessageReader<AssetEvent<ScenePatch>>,
    mut list_events: MessageReader<AssetEvent<SceneListPatch>>,
    assets: Res<AssetServer>,
    mut patches: ResMut<Assets<ScenePatch>>,
    mut list_patches: ResMut<Assets<SceneListPatch>>,
    mut queued: ResMut<QueuedScenes>,
) {
    for event in events.read() {
        match *event {
            AssetEvent::LoadedWithDependencies { id } => {
                if let Some(patch) = patches.get(id) {
                    match patch.resolve_internal(&assets, &patches) {
                        Ok(resolved) => {
                            let mut patch = patches.get_mut(id).unwrap();
                            patch.resolved = Some(Arc::new(resolved));
                        }
                        Err(err) => error!("Failed to resolve scene {id}: {err}"),
                    }
                }
            }
            AssetEvent::Removed { id } => {
                if let Some(waiting) = queued.waiting_scene_entities.remove(&id)
                    && !waiting.is_empty()
                {
                    error!(
                        "Failed to spawn entities waiting for scene {id:?} because it was removed: {waiting:?}"
                    );
                }
            }
            _ => {}
        }
    }
    for event in list_events.read() {
        match *event {
            AssetEvent::LoadedWithDependencies { id } => {
                if let Some(mut list_patch) = list_patches.get_mut(id) {
                    match list_patch.resolve_internal(&assets, &patches) {
                        Ok(resolved) => {
                            list_patch.resolved = Some(resolved);
                        }
                        Err(err) => error!("Failed to resolve scene list {id}: {err}"),
                    }
                }
            }
            AssetEvent::Removed { id } => {
                if let Some(waiting) = queued.waiting_scene_list_spawns.remove(&id)
                    && waiting > 0
                {
                    error!(
                        "Failed to spawn scene list {id:?} {waiting} times because it was removed."
                    );
                }

                if let Some(waiting) = queued.waiting_related_list_entities.remove(&id)
                    && !waiting.is_empty()
                {
                    let waiting_entities = waiting.iter().map(|r| r.entity).collect::<Vec<_>>();
                    error!(
                        "Failed to spawn related entities for scene list {id:?} because it was removed: {waiting_entities:?}"
                    );
                }
            }
            _ => {}
        }
    }
}

/// A [`Resource`] that tracks entities / scenes that have been queued to spawn.
#[derive(Resource, Default)]
pub struct QueuedScenes {
    new_scene_entities: Vec<Entity>,
    related_scene_list_spawns: Vec<(RelatedSceneListSpawn, Handle<SceneListPatch>)>,
    scene_list_spawns: Vec<Handle<SceneListPatch>>,
    waiting_scene_entities: HashMap<Handle<ScenePatch>, Vec<Entity>>,
    waiting_related_list_entities: HashMap<Handle<SceneListPatch>, Vec<RelatedSceneListSpawn>>,
    waiting_scene_list_spawns: HashMap<Handle<SceneListPatch>, usize>,
}

pub(crate) struct RelatedSceneListSpawn {
    entity: Entity,
    insert: fn(&mut EntityWorldMut, target: Entity),
}

/// An [`Observer`] system that queues newly added [`ScenePatchInstance`] entities.
pub fn on_add_scene_patch_instance(
    add: On<Add, ScenePatchInstance>,
    mut queued_scenes: ResMut<QueuedScenes>,
) {
    queued_scenes.new_scene_entities.push(add.entity);
}

/// A system that spawns queued scenes when they are loaded.
pub fn spawn_queued(
    world: &mut World,
    scene_patch_instances: &mut QueryState<&ScenePatchInstance>,
    mut reader: Local<MessageCursor<AssetEvent<ScenePatch>>>,
    mut list_reader: Local<MessageCursor<AssetEvent<SceneListPatch>>>,
) {
    world.resource_scope(|world, mut list_patches: Mut<Assets<SceneListPatch>>| {
        world.resource_scope(|world, mut queued: Mut<QueuedScenes>| {
            loop {
                if queued.is_empty() {
                    break;
                }
                queued.spawn_queued(world, scene_patch_instances, &list_patches);
            }

            world.resource_scope(|world, events: Mut<Messages<AssetEvent<ScenePatch>>>| {
                for event in reader.read(&events) {
                    let patches = world.resource::<Assets<ScenePatch>>();
                    if let AssetEvent::LoadedWithDependencies { id } = event
                        && let Some(resolved) = patches.get(*id).and_then(|p| p.resolved.clone())
                        && let Some(entities) = queued.waiting_scene_entities.remove(id)
                    {
                        for entity in entities {
                            if let Ok(mut entity_mut) = world.get_entity_mut(entity)
                                && let Err(err) = resolved.apply(&mut entity_mut)
                            {
                                error!(
                                    "Failed to apply scene (id: {}) to entity {entity}: {}",
                                    id, err
                                );
                            }
                        }
                    }
                }
            });
            world.resource_scope(
                |world, list_events: Mut<Messages<AssetEvent<SceneListPatch>>>| {
                    for event in list_reader.read(&list_events) {
                        if let AssetEvent::LoadedWithDependencies { id } = event
                            && let Some(list_patch) = list_patches.get_mut(*id)
                        {
                            if let Some(scene_list_spawns) =
                                queued.waiting_related_list_entities.remove(id)
                            {
                                for scene_list_spawn in scene_list_spawns {
                                    let result = list_patch.spawn_with(world, |entity| {
                                        (scene_list_spawn.insert)(entity, scene_list_spawn.entity);
                                    });

                                    if let Err(err) = result {
                                        error!("Failed to spawn scene list (id: {}): {}", id, err);
                                    }
                                }
                            }

                            if let Some(waiting_list_spawns) =
                                queued.waiting_scene_list_spawns.remove(id)
                            {
                                for _ in 0..waiting_list_spawns {
                                    let result = list_patch.spawn(world);
                                    if let Err(err) = result {
                                        error!("Failed to spawn scene list (id: {}): {}", id, err);
                                    }
                                }
                            }
                        }
                    }
                },
            );
        });
    });
}

impl QueuedScenes {
    fn is_empty(&self) -> bool {
        self.new_scene_entities.is_empty()
            && self.related_scene_list_spawns.is_empty()
            && self.scene_list_spawns.is_empty()
    }

    fn spawn_queued(
        &mut self,
        world: &mut World,
        scene_patch_instances: &mut QueryState<&ScenePatchInstance>,
        list_patches: &Assets<SceneListPatch>,
    ) {
        for entity in core::mem::take(&mut self.new_scene_entities) {
            let Ok(handle) = scene_patch_instances.get(world, entity).map(|h| &h.0) else {
                continue;
            };
            let patches = world.resource::<Assets<ScenePatch>>();
            if let Some(resolved) = patches.get(handle).and_then(|p| p.resolved.clone()) {
                let mut entity_mut = world.get_entity_mut(entity).unwrap();
                if let Err(err) = resolved.apply(&mut entity_mut) {
                    let scene_patch_instance = scene_patch_instances.get(world, entity).unwrap();
                    let handle = &scene_patch_instance.0;
                    let id = handle.id();
                    let path = handle.path();
                    error!(
                        "Failed to apply scene (id: {id}, path: {path:?}) to \
                                    entity {entity}: {err}",
                    );
                }
            } else {
                let entities = self
                    .waiting_scene_entities
                    .entry(handle.clone())
                    .or_default();
                entities.push(entity);
            }
        }

        for (scene_list_spawn, handle) in core::mem::take(&mut self.related_scene_list_spawns) {
            if let Some(list_patch) = list_patches.get(&handle) {
                let result = list_patch.spawn_with(world, |entity| {
                    (scene_list_spawn.insert)(entity, scene_list_spawn.entity);
                });

                if let Err(err) = result {
                    error!(
                        "Failed to spawn scene list (id: {}, path: {:?}): {}",
                        handle.id(),
                        handle.path(),
                        err
                    );
                }
            } else {
                let entities = self
                    .waiting_related_list_entities
                    .entry(handle)
                    .or_default();
                entities.push(scene_list_spawn);
            }
        }

        for handle in core::mem::take(&mut self.scene_list_spawns) {
            if let Some(list_patch) = list_patches.get(&handle) {
                let result = list_patch.spawn(world);
                if let Err(err) = result {
                    error!(
                        "Failed to spawn scene list (id: {}, path: {:?}): {}",
                        handle.id(),
                        handle.path(),
                        err
                    );
                }
            } else {
                let count = self.waiting_scene_list_spawns.entry(handle).or_default();
                *count += 1;
            }
        }
    }
}
