use crate::{DynamicScene, Scene};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::{
    entity::{Entity, EntityMap},
    event::ManualEventReader,
    system::Command,
    world::{EntityRef, Mut, World},
};
use bevy_hierarchy::{AddChild, Parent};
use bevy_utils::{tracing::error, HashMap};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug)]
struct InstanceInfo {
    entity_map: EntityMap,
    parent: Option<Entity>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct InstanceId(Uuid);

impl InstanceId {
    fn new() -> Self {
        InstanceId(Uuid::new_v4())
    }
}

#[derive(Hash, Debug, PartialEq, Eq)]
pub enum SceneHandle {
    World(Handle<Scene>),
    Reflected(Handle<DynamicScene>),
}
impl Clone for SceneHandle {
    fn clone(&self) -> Self {
        match self {
            Self::World(handle) => Self::World(handle.clone_weak()),
            Self::Reflected(handle) => Self::Reflected(handle.clone_weak()),
        }
    }
}
impl SceneHandle {
    fn write_to_world(&self, world: &mut World, entity_map: &mut EntityMap) -> SpawnResult<()> {
        let err = || self.non_existing();
        match self {
            Self::Reflected(scene) => {
                world.resource_scope(|world, scenes: Mut<Assets<DynamicScene>>| {
                    let scene = scenes.get(scene).ok_or_else(err)?;
                    scene.write_to_world(world, entity_map)
                })?;
            }
            Self::World(scene) => world.resource_scope(|world, scenes: Mut<Assets<Scene>>| {
                let scene = scenes.get(scene).ok_or_else(err)?;
                scene.write_to_world(world, entity_map)
            })?,
        };
        Ok(())
    }

    fn non_existing(&self) -> SceneSpawnError {
        SceneSpawnError::NonExistentScene {
            handle: self.clone(),
        }
    }
}
impl From<Handle<Scene>> for SceneHandle {
    fn from(handle: Handle<Scene>) -> Self {
        Self::World(handle)
    }
}
impl From<Handle<DynamicScene>> for SceneHandle {
    fn from(handle: Handle<DynamicScene>) -> Self {
        Self::Reflected(handle)
    }
}

#[derive(Debug, Clone)]
struct SpawnCommand {
    scene: SceneHandle,
    instance: InstanceId,
    parent: Option<Entity>,
}

impl SpawnCommand {
    fn new(scene: impl Into<SceneHandle>, instance: InstanceId, parent: Option<Entity>) -> Self {
        Self {
            scene: scene.into(),
            instance,
            parent,
        }
    }
}
#[derive(Default)]
pub struct SceneSpawner {
    instances: HashMap<SceneHandle, Vec<InstanceId>>,
    instances_info: HashMap<InstanceId, InstanceInfo>,
    readers: SceneEventReaders,
    scenes_to_spawn: Vec<SpawnCommand>,
    instances_to_despawn: Vec<InstanceId>,
}

/// Helper struct to wrap `ManualEventReader` for the various scene handle
/// types.
#[derive(Default)]
struct SceneEventReaders {
    dynamic: ManualEventReader<AssetEvent<DynamicScene>>,
    real: ManualEventReader<AssetEvent<Scene>>,
}

#[derive(Error, Debug)]
pub enum SceneSpawnError {
    #[error("scene contains the unregistered component `{type_name}`. consider adding `#[reflect(Component)]` to your type")]
    UnregisteredComponent { type_name: String },
    #[error("scene contains the unregistered type `{type_name}`. consider registering the type using `app.register_type::<T>()`")]
    UnregisteredType { type_name: String },
    #[error("scene does not exist")]
    NonExistentScene { handle: SceneHandle },
}

pub type SpawnResult<T> = Result<T, SceneSpawnError>;
impl SceneSpawner {
    /// Spawn a scene.
    ///
    /// This will only update the world when [`scene_spawner_system`] runs, see
    /// [`SceneSpawner::spawn_sync`] for a method with immediate world update.
    ///
    /// The returned [`InstanceId`] can be used later to refer to the specific
    /// instance of the scene you spawned.
    pub fn spawn(&mut self, scene_handle: Handle<Scene>) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn
            .push(SpawnCommand::new(scene_handle, instance_id, None));
        instance_id
    }

    /// Spawn a dynamic scene. See [`SceneSpawner::spawn`].
    pub fn spawn_dynamic(&mut self, scene_handle: Handle<DynamicScene>) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn
            .push(SpawnCommand::new(scene_handle, instance_id, None));
        instance_id
    }

    /// Spawn a scene as a child of an existing entity.
    ///
    /// The returned [`InstanceId`] can be used later to refer to the specific
    /// instance of the scene you spawned.
    pub fn spawn_as_child(&mut self, scene_handle: Handle<Scene>, parent: Entity) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn
            .push(SpawnCommand::new(scene_handle, instance_id, Some(parent)));
        instance_id
    }

    /// Spawn a dynamic scene as a child of an existing entity. See
    /// [`SceneSpawner::spawn_as_child`].
    pub fn spawn_dynamic_as_child(
        &mut self,
        scene_handle: Handle<DynamicScene>,
        parent: Entity,
    ) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn
            .push(SpawnCommand::new(scene_handle, instance_id, Some(parent)));
        instance_id
    }

    /// Despawn the provided scene. This will remove the scene and
    /// all its related entities from the world.
    ///
    /// This will only update the world when [`scene_spawner_system`] runs, see
    /// [`SceneSpawner::despawn_sync`] for a method with immediate world
    /// update.
    pub fn despawn(&mut self, scene_handle: Handle<Scene>) -> SpawnResult<()> {
        self.despawn_scene(scene_handle.into())
    }

    /// Despawn the provided dynamic scene. See [`SceneSpawner::despawn`].
    pub fn despawn_dynamic(&mut self, scene_handle: Handle<DynamicScene>) -> SpawnResult<()> {
        self.despawn_scene(scene_handle.into())
    }

    fn despawn_scene(&mut self, scene_handle: SceneHandle) -> SpawnResult<()> {
        let err = || scene_handle.non_existing();
        for instance_id in self.instances.get(&scene_handle).ok_or_else(err)? {
            self.instances_to_despawn.push(*instance_id);
        }
        Ok(())
    }

    /// Despawns immediately the provided scene.
    ///
    /// This will remove the scene and all its related entities from the world.
    /// The world will be updated before this method returns. Requires
    /// exclusive world acces through `&mut World`.
    ///
    /// [`SceneSpawner::despawn`] does the same thing, but does not require
    /// exclusive world access, it will update the world when
    /// [`scene_spawner_system`] runs.
    pub fn despawn_sync(
        &mut self,
        world: &mut World,
        scene_handle: Handle<Scene>,
    ) -> SpawnResult<()> {
        self.despawn_scene_sync(world, scene_handle.into())
    }

    /// Despawns immediately the provided dynamic scene. See
    /// [`SceneSpawner::despawn_sync`].
    pub fn despawn_dynamic_sync(
        &mut self,
        world: &mut World,
        scene_handle: Handle<DynamicScene>,
    ) -> SpawnResult<()> {
        self.despawn_scene_sync(world, scene_handle.into())
    }

    fn despawn_scene_sync(
        &mut self,
        world: &mut World,
        scene_handle: SceneHandle,
    ) -> SpawnResult<()> {
        let err = || scene_handle.non_existing();
        for instance_id in self.instances.get(&scene_handle).ok_or_else(err)? {
            let instance_info = self.instances_info.remove(instance_id).ok_or_else(err)?;
            for entity in instance_info.entity_map.values() {
                // Ignore result: if it is already despawned, good!
                let _ = world.despawn(entity);
            }
        }
        self.instances.remove(&scene_handle);
        Ok(())
    }

    pub fn despawn_instance(&mut self, instance_id: InstanceId) {
        self.instances_to_despawn.push(instance_id);
    }

    pub fn despawn_instance_sync(&mut self, world: &mut World, instance_id: &InstanceId) {
        if let Some(instance_info) = self.instances_info.remove(instance_id) {
            for entity in instance_info.entity_map.values() {
                // Ignore result: if it is already despawned, good!
                let _ = world.despawn(entity);
            }
        }
    }

    /// Spawn a scene into the world immediately.
    ///
    /// The world will be updated before this method returns. Requires
    /// exclusive world acces through [`&mut World`].
    pub fn spawn_sync(
        &mut self,
        world: &mut World,
        scene_handle: Handle<Scene>,
    ) -> SpawnResult<InstanceId> {
        let cmd = SpawnCommand::new(scene_handle, InstanceId::new(), None);
        self.spawn_scene_instance(world, cmd)
    }

    /// Spawn a scene into the world immediately. See
    /// [`SceneSpawner::spawn_sync`].
    pub fn spawn_dynamic_sync(
        &mut self,
        world: &mut World,
        scene_handle: Handle<DynamicScene>,
    ) -> SpawnResult<InstanceId> {
        let cmd = SpawnCommand::new(scene_handle, InstanceId::new(), None);
        self.spawn_scene_instance(world, cmd)
    }

    /// Spawn a scene instance, using the provided [`InstanceId`].
    fn spawn_scene_instance(
        &mut self,
        world: &mut World,
        SpawnCommand {
            scene,
            instance,
            parent,
        }: SpawnCommand,
    ) -> SpawnResult<InstanceId> {
        let mut entity_map = EntityMap::default();
        scene.write_to_world(world, &mut entity_map)?;

        let info = InstanceInfo { entity_map, parent };
        self.instances_info.insert(instance, info);
        let spawned = self.instances.entry(scene).or_default();
        spawned.push(instance);
        Ok(instance)
    }

    pub fn update_spawned_scenes(
        &mut self,
        world: &mut World,
        scene_handles: &[SceneHandle],
    ) -> SpawnResult<()> {
        for scene_handle in scene_handles {
            let err = || scene_handle.non_existing();
            for instance_id in self.instances.get(scene_handle).ok_or_else(err)? {
                let instance_info = self.instances_info.get_mut(instance_id).ok_or_else(err)?;
                scene_handle.write_to_world(world, &mut instance_info.entity_map)?;
                if let Some(parent) = instance_info.parent {
                    self.set_scene_parent(world, *instance_id, parent);
                }
            }
        }
        Ok(())
    }

    /// Manually despawn instances marked for elimination.
    pub fn despawn_queued_instances(&mut self, world: &mut World) -> SpawnResult<()> {
        let instances_to_despawn = std::mem::take(&mut self.instances_to_despawn);

        for instance in instances_to_despawn {
            self.despawn_instance_sync(world, &instance);
        }
        Ok(())
    }

    /// Manually spawn scenes marked for creation.
    pub fn spawn_queued_scenes(&mut self, world: &mut World) -> SpawnResult<()> {
        let scenes_to_spawn = std::mem::take(&mut self.scenes_to_spawn);
        for cmd in scenes_to_spawn {
            match self.spawn_scene_instance(world, cmd.clone()) {
                Ok(_) => {
                    if let Some(parent) = cmd.parent {
                        self.set_scene_parent(world, cmd.instance, parent);
                    }
                }
                // The scene to spawn did not exist in the Assets<Scene> (or
                // Assets<DynamicScene>) collection, meaning it still didn't
                // finish loading, so we keep it tucked into the spawn queue to
                // try loading it later, once it fully loaded.
                Err(SceneSpawnError::NonExistentScene { .. }) => {
                    // NOTE: the handle in NonExistentScene is a weak handle, I
                    // found that cloning scene_handle **would break Scene (but
                    // not DynamicScene) loading**
                    self.scenes_to_spawn.push(cmd);
                }
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }

    fn set_scene_parent(&self, world: &mut World, instance_id: InstanceId, parent: Entity) {
        // Only the root of the scene _does not_ have a parent.
        let has_no_parents = |entity: EntityRef| !entity.contains::<Parent>();
        let is_scene_root =
            |entity, world: &World| world.get_entity(entity).map_or(false, has_no_parents);

        if let Some(instance) = self.instances_info.get(&instance_id) {
            for entity in instance.entity_map.values() {
                if is_scene_root(entity, world) {
                    let child = entity;
                    // FIXME: is this a bug in `AddChild`?
                    let (parent, child) = (child, parent);
                    AddChild { parent, child }.write(world);
                }
            }
        }
    }

    /// Check that a scene instance spawned previously is ready to use.
    pub fn instance_is_ready(&self, instance_id: InstanceId) -> bool {
        self.instances_info.contains_key(&instance_id)
    }

    /// Get an iterator over the entities in an instance, once it's spawned.
    pub fn iter_instance_entities(
        &'_ self,
        instance_id: InstanceId,
    ) -> Option<impl Iterator<Item = Entity> + '_> {
        self.instances_info
            .get(&instance_id)
            .map(|instance| instance.entity_map.values())
    }
}

/// Update the world according to queued scene commands.
///
/// This system runs at the very end of the
/// [`CoreStage::PreUpdate`](bevy_app::prelude::CoreStage::PreUpdate) stage.
/// Meaning That scene updates (typically in the case of hot-reloading) will
/// be visible in the `CoreStage::Update` (the default) stage.
pub fn scene_spawner_system(world: &mut World) {
    world.resource_scope(|world, mut scene_spawner: Mut<SceneSpawner>| {
        let scene_spawner = &mut *scene_spawner;
        let mut updated_spawned_scenes = Vec::new();
        for event in scene_spawner.readers.dynamic.iter(world.resource()) {
            if let AssetEvent::Modified { handle } = event {
                let scene_handle = SceneHandle::Reflected(handle.clone_weak());
                if scene_spawner.instances.contains_key(&scene_handle) {
                    updated_spawned_scenes.push(scene_handle);
                }
            }
        }
        for event in scene_spawner.readers.real.iter(world.resource()) {
            if let AssetEvent::Modified { handle } = event {
                let scene_handle = SceneHandle::World(handle.clone_weak());
                if scene_spawner.instances.contains_key(&scene_handle) {
                    updated_spawned_scenes.push(scene_handle);
                }
            }
        }

        scene_spawner.despawn_queued_instances(world).unwrap();
        scene_spawner.spawn_queued_scenes(world).unwrap();
        scene_spawner
            .update_spawned_scenes(world, &updated_spawned_scenes)
            .unwrap();
    });
}
