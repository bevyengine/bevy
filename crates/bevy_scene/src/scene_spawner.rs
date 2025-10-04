use crate::{DynamicScene, Scene};
use bevy_asset::{AssetEvent, AssetId, Assets, Handle};
use bevy_ecs::{
    entity::{Entity, EntityHashMap},
    event::EntityEvent,
    hierarchy::ChildOf,
    message::{MessageCursor, Messages},
    reflect::AppTypeRegistry,
    resource::Resource,
    world::{Mut, World},
};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::Reflect;
use bevy_utils::prelude::DebugName;
use thiserror::Error;
use uuid::Uuid;

use crate::{DynamicSceneRoot, SceneRoot};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    change_detection::ResMut,
    prelude::{Changed, Component, Without},
    system::{Commands, Query},
};

/// Triggered on a scene's parent entity when [`SceneInstance`](`crate::SceneInstance`) becomes ready to use.
///
/// See also [`On`], [`SceneSpawner::instance_is_ready`].
///
/// [`On`]: bevy_ecs::observer::On
#[derive(Clone, Copy, Debug, Eq, PartialEq, EntityEvent, Reflect)]
#[reflect(Debug, PartialEq, Clone)]
pub struct SceneInstanceReady {
    /// The entity whose scene instance is ready.
    pub entity: Entity,
    /// Instance which has been spawned.
    pub instance_id: InstanceId,
}

/// Information about a scene instance.
#[derive(Debug)]
struct InstanceInfo {
    /// Mapping of entities from the scene world to the instance world.
    entity_map: EntityHashMap<Entity>,
    /// The parent to attach this instance to.
    parent: Option<Entity>,
}

/// Unique id identifying a scene instance.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Reflect)]
#[reflect(Debug, PartialEq, Hash, Clone)]
pub struct InstanceId(Uuid);

impl InstanceId {
    fn new() -> Self {
        InstanceId(Uuid::new_v4())
    }
}

/// Handles spawning and despawning scenes in the world, either synchronously or batched through the [`scene_spawner_system`].
///
/// Synchronous methods: (Scene operations will take effect immediately)
/// - [`spawn_sync`](Self::spawn_sync)
/// - [`spawn_dynamic_sync`](Self::spawn_dynamic_sync)
/// - [`despawn_sync`](Self::despawn_sync)
/// - [`despawn_dynamic_sync`](Self::despawn_dynamic_sync)
/// - [`despawn_instance_sync`](Self::despawn_instance_sync)
/// - [`update_spawned_scenes`](Self::update_spawned_scenes)
/// - [`update_spawned_dynamic_scenes`](Self::update_spawned_dynamic_scenes)
/// - [`spawn_queued_scenes`](Self::spawn_queued_scenes)
/// - [`despawn_queued_scenes`](Self::despawn_queued_scenes)
/// - [`despawn_queued_instances`](Self::despawn_queued_instances)
///
/// Deferred methods: (Scene operations will be processed when the [`scene_spawner_system`] is run)
/// - [`spawn_dynamic`](Self::spawn_dynamic)
/// - [`spawn_dynamic_as_child`](Self::spawn_dynamic_as_child)
/// - [`spawn`](Self::spawn)
/// - [`spawn_as_child`](Self::spawn_as_child)
/// - [`despawn`](Self::despawn)
/// - [`despawn_dynamic`](Self::despawn_dynamic)
/// - [`despawn_instance`](Self::despawn_instance)
#[derive(Default, Resource)]
pub struct SceneSpawner {
    pub(crate) spawned_scenes: HashMap<AssetId<Scene>, HashSet<InstanceId>>,
    pub(crate) spawned_dynamic_scenes: HashMap<AssetId<DynamicScene>, HashSet<InstanceId>>,
    spawned_instances: HashMap<InstanceId, InstanceInfo>,
    scene_asset_event_reader: MessageCursor<AssetEvent<Scene>>,
    // TODO: temp fix for https://github.com/bevyengine/bevy/issues/12756 effect on scenes
    // To handle scene hot reloading, they are unloaded/reloaded on asset modifications.
    // When loading several subassets of a scene as is common with gltf, they each trigger a complete asset load,
    // and each will trigger either a created or modified event for the parent asset. This causes the scene to be
    // unloaded, losing its initial setup, and reloaded without it.
    // Debouncing scene asset events let us ignore events that happen less than SCENE_ASSET_AGE_THRESHOLD frames
    // apart and not reload the scene in those cases as it's unlikely to be an actual asset change.
    debounced_scene_asset_events: HashMap<AssetId<Scene>, u32>,
    dynamic_scene_asset_event_reader: MessageCursor<AssetEvent<DynamicScene>>,
    // TODO: temp fix for https://github.com/bevyengine/bevy/issues/12756 effect on scenes
    // See debounced_scene_asset_events
    debounced_dynamic_scene_asset_events: HashMap<AssetId<DynamicScene>, u32>,
    scenes_to_spawn: Vec<(Handle<Scene>, InstanceId, Option<Entity>)>,
    dynamic_scenes_to_spawn: Vec<(Handle<DynamicScene>, InstanceId, Option<Entity>)>,
    scenes_to_despawn: Vec<AssetId<Scene>>,
    dynamic_scenes_to_despawn: Vec<AssetId<DynamicScene>>,
    instances_to_despawn: Vec<InstanceId>,
    instances_ready: Vec<(InstanceId, Option<Entity>)>,
}

/// Errors that can occur when spawning a scene.
#[derive(Error, Debug)]
pub enum SceneSpawnError {
    /// Scene contains an unregistered component type.
    #[error("scene contains the unregistered component `{type_path}`. consider adding `#[reflect(Component)]` to your type")]
    UnregisteredComponent {
        /// Type of the unregistered component.
        type_path: String,
    },
    /// Scene contains an unregistered resource type.
    #[error("scene contains the unregistered resource `{type_path}`. consider adding `#[reflect(Resource)]` to your type")]
    UnregisteredResource {
        /// Type of the unregistered resource.
        type_path: String,
    },
    /// Scene contains an unregistered type.
    #[error(
        "scene contains the unregistered type `{std_type_name}`. \
        consider reflecting it with `#[derive(Reflect)]` \
        and registering the type using `app.register_type::<T>()`"
    )]
    UnregisteredType {
        /// The [type name](std::any::type_name) for the unregistered type.
        std_type_name: DebugName,
    },
    /// Scene contains an unregistered type which has a `TypePath`.
    #[error(
        "scene contains the reflected type `{type_path}` but it was not found in the type registry. \
        consider registering the type using `app.register_type::<T>()``"
    )]
    UnregisteredButReflectedType {
        /// The unregistered type.
        type_path: String,
    },
    /// Scene contains a proxy without a represented type.
    #[error("scene contains dynamic type `{type_path}` without a represented type. consider changing this using `set_represented_type`.")]
    NoRepresentedType {
        /// The dynamic instance type.
        type_path: String,
    },
    /// Dynamic scene with the given id does not exist.
    #[error("scene does not exist")]
    NonExistentScene {
        /// Id of the non-existent dynamic scene.
        id: AssetId<DynamicScene>,
    },
    /// Scene with the given id does not exist.
    #[error("scene does not exist")]
    NonExistentRealScene {
        /// Id of the non-existent scene.
        id: AssetId<Scene>,
    },
}

impl SceneSpawner {
    /// Schedule the spawn of a new instance of the provided dynamic scene.
    pub fn spawn_dynamic(&mut self, id: impl Into<Handle<DynamicScene>>) -> InstanceId {
        let instance_id = InstanceId::new();
        self.dynamic_scenes_to_spawn
            .push((id.into(), instance_id, None));
        instance_id
    }

    /// Schedule the spawn of a new instance of the provided dynamic scene as a child of `parent`.
    pub fn spawn_dynamic_as_child(
        &mut self,
        id: impl Into<Handle<DynamicScene>>,
        parent: Entity,
    ) -> InstanceId {
        let instance_id = InstanceId::new();
        self.dynamic_scenes_to_spawn
            .push((id.into(), instance_id, Some(parent)));
        instance_id
    }

    /// Schedule the spawn of a new instance of the provided scene.
    pub fn spawn(&mut self, id: impl Into<Handle<Scene>>) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn.push((id.into(), instance_id, None));
        instance_id
    }

    /// Schedule the spawn of a new instance of the provided scene as a child of `parent`.
    pub fn spawn_as_child(&mut self, id: impl Into<Handle<Scene>>, parent: Entity) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn
            .push((id.into(), instance_id, Some(parent)));
        instance_id
    }

    /// Schedule the despawn of all instances of the provided scene.
    pub fn despawn(&mut self, id: impl Into<AssetId<Scene>>) {
        self.scenes_to_despawn.push(id.into());
    }

    /// Schedule the despawn of all instances of the provided dynamic scene.
    pub fn despawn_dynamic(&mut self, id: impl Into<AssetId<DynamicScene>>) {
        self.dynamic_scenes_to_despawn.push(id.into());
    }

    /// Schedule the despawn of a scene instance, removing all its entities from the world.
    ///
    /// Note: this will despawn _all_ entities associated with this instance, including those
    /// that have been removed from the scene hierarchy. To despawn _only_ entities still in the hierarchy,
    /// despawn the relevant root entity directly.
    pub fn despawn_instance(&mut self, instance_id: InstanceId) {
        self.instances_to_despawn.push(instance_id);
    }

    /// This will remove all records of this instance, without despawning any entities.
    pub fn unregister_instance(&mut self, instance_id: InstanceId) {
        self.spawned_instances.remove(&instance_id);
    }

    /// Immediately despawns all instances of a scene.
    pub fn despawn_sync(
        &mut self,
        world: &mut World,
        id: impl Into<AssetId<Scene>>,
    ) -> Result<(), SceneSpawnError> {
        if let Some(instance_ids) = self.spawned_scenes.remove(&id.into()) {
            for instance_id in instance_ids {
                self.despawn_instance_sync(world, &instance_id);
            }
        }
        Ok(())
    }

    /// Immediately despawns all instances of a dynamic scene.
    pub fn despawn_dynamic_sync(
        &mut self,
        world: &mut World,
        id: impl Into<AssetId<DynamicScene>>,
    ) -> Result<(), SceneSpawnError> {
        if let Some(instance_ids) = self.spawned_dynamic_scenes.remove(&id.into()) {
            for instance_id in instance_ids {
                self.despawn_instance_sync(world, &instance_id);
            }
        }
        Ok(())
    }

    /// Immediately despawns a scene instance, removing all its entities from the world.
    pub fn despawn_instance_sync(&mut self, world: &mut World, instance_id: &InstanceId) {
        if let Some(mut instance) = self.spawned_instances.remove(instance_id) {
            Self::despawn_instance_internal(world, &mut instance);
        }
    }

    fn despawn_instance_internal(world: &mut World, instance: &mut InstanceInfo) {
        for &entity in instance.entity_map.values() {
            if let Ok(entity_mut) = world.get_entity_mut(entity) {
                entity_mut.despawn();
            };
        }
        // Just make sure if we reuse `InstanceInfo` for something, we don't reuse the despawned entities.
        instance.entity_map.clear();
    }

    /// Immediately spawns a new instance of the provided dynamic scene.
    pub fn spawn_dynamic_sync(
        &mut self,
        world: &mut World,
        id: impl Into<AssetId<DynamicScene>>,
    ) -> Result<InstanceId, SceneSpawnError> {
        let mut entity_map = EntityHashMap::default();
        let id = id.into();
        Self::spawn_dynamic_internal(world, id, &mut entity_map)?;
        let instance_id = InstanceId::new();
        self.spawned_instances.insert(
            instance_id,
            InstanceInfo {
                entity_map,
                parent: None,
            },
        );
        let spawned = self.spawned_dynamic_scenes.entry(id).or_default();
        spawned.insert(instance_id);
        // We trigger `SceneInstanceReady` events after processing all scenes
        // SceneSpawner may not be available in the observer.
        self.instances_ready.push((instance_id, None));
        Ok(instance_id)
    }

    fn spawn_dynamic_internal(
        world: &mut World,
        id: AssetId<DynamicScene>,
        entity_map: &mut EntityHashMap<Entity>,
    ) -> Result<(), SceneSpawnError> {
        world.resource_scope(|world, scenes: Mut<Assets<DynamicScene>>| {
            let scene = scenes
                .get(id)
                .ok_or(SceneSpawnError::NonExistentScene { id })?;

            scene.write_to_world(world, entity_map)
        })
    }

    /// Immediately spawns a new instance of the provided scene.
    pub fn spawn_sync(
        &mut self,
        world: &mut World,
        id: impl Into<AssetId<Scene>>,
    ) -> Result<InstanceId, SceneSpawnError> {
        let mut entity_map = EntityHashMap::default();
        let id = id.into();
        Self::spawn_sync_internal(world, id, &mut entity_map)?;
        let instance_id = InstanceId::new();
        self.spawned_instances.insert(
            instance_id,
            InstanceInfo {
                entity_map,
                parent: None,
            },
        );
        let spawned = self.spawned_scenes.entry(id).or_default();
        spawned.insert(instance_id);
        // We trigger `SceneInstanceReady` events after processing all scenes
        // SceneSpawner may not be available in the observer.
        self.instances_ready.push((instance_id, None));
        Ok(instance_id)
    }

    fn spawn_sync_internal(
        world: &mut World,
        id: AssetId<Scene>,
        entity_map: &mut EntityHashMap<Entity>,
    ) -> Result<(), SceneSpawnError> {
        world.resource_scope(|world, scenes: Mut<Assets<Scene>>| {
            let scene = scenes
                .get(id)
                .ok_or(SceneSpawnError::NonExistentRealScene { id })?;

            scene.write_to_world_with(
                world,
                entity_map,
                &world.resource::<AppTypeRegistry>().clone(),
            )
        })
    }

    /// Iterate through all instances of the provided scenes and update those immediately.
    ///
    /// Useful for updating already spawned scene instances after their corresponding scene has been
    /// modified.
    pub fn update_spawned_scenes(
        &mut self,
        world: &mut World,
        scene_ids: &[AssetId<Scene>],
    ) -> Result<(), SceneSpawnError> {
        for id in scene_ids {
            if let Some(spawned_instances) = self.spawned_scenes.get(id) {
                for instance_id in spawned_instances {
                    if let Some(instance_info) = self.spawned_instances.get_mut(instance_id) {
                        // Despawn the scene before respawning it. This is a very heavy operation,
                        // but otherwise, entities may be left behind, or be left in an otherwise
                        // invalid state (e.g., invalid relationships).
                        Self::despawn_instance_internal(world, instance_info);
                        Self::spawn_sync_internal(world, *id, &mut instance_info.entity_map)?;
                        Self::set_scene_instance_parent_sync(world, instance_info);
                        // We trigger `SceneInstanceReady` events after processing all scenes
                        // SceneSpawner may not be available in the observer.
                        self.instances_ready
                            .push((*instance_id, instance_info.parent));
                    }
                }
            }
        }
        Ok(())
    }

    /// Iterate through all instances of the provided dynamic scenes and update those immediately.
    ///
    /// Useful for updating already spawned scene instances after their corresponding dynamic scene
    /// has been modified.
    pub fn update_spawned_dynamic_scenes(
        &mut self,
        world: &mut World,
        scene_ids: &[AssetId<DynamicScene>],
    ) -> Result<(), SceneSpawnError> {
        for id in scene_ids {
            if let Some(spawned_instances) = self.spawned_dynamic_scenes.get(id) {
                for instance_id in spawned_instances {
                    if let Some(instance_info) = self.spawned_instances.get_mut(instance_id) {
                        // Despawn the scene before respawning it. This is a very heavy operation,
                        // but otherwise, entities may be left behind, or be left in an otherwise
                        // invalid state (e.g., invalid relationships).
                        Self::despawn_instance_internal(world, instance_info);
                        Self::spawn_dynamic_internal(world, *id, &mut instance_info.entity_map)?;
                        Self::set_scene_instance_parent_sync(world, instance_info);
                        // We trigger `SceneInstanceReady` events after processing all scenes
                        // SceneSpawner may not be available in the observer.
                        self.instances_ready
                            .push((*instance_id, instance_info.parent));
                    }
                }
            }
        }
        Ok(())
    }

    /// Immediately despawns all scenes scheduled for despawn by despawning their instances.
    pub fn despawn_queued_scenes(&mut self, world: &mut World) -> Result<(), SceneSpawnError> {
        let scenes_to_despawn = core::mem::take(&mut self.scenes_to_despawn);
        for scene_handle in scenes_to_despawn {
            self.despawn_sync(world, scene_handle)?;
        }
        let scenes_to_despawn = core::mem::take(&mut self.dynamic_scenes_to_despawn);
        for scene_handle in scenes_to_despawn {
            self.despawn_dynamic_sync(world, scene_handle)?;
        }
        Ok(())
    }

    /// Immediately despawns all scene instances scheduled for despawn.
    pub fn despawn_queued_instances(&mut self, world: &mut World) {
        let instances_to_despawn = core::mem::take(&mut self.instances_to_despawn);

        for instance_id in instances_to_despawn {
            self.despawn_instance_sync(world, &instance_id);
        }
    }

    /// Immediately spawns all scenes scheduled for spawn.
    pub fn spawn_queued_scenes(&mut self, world: &mut World) -> Result<(), SceneSpawnError> {
        let scenes_to_spawn = core::mem::take(&mut self.dynamic_scenes_to_spawn);

        for (handle, instance_id, parent) in scenes_to_spawn {
            let mut entity_map = EntityHashMap::default();

            match Self::spawn_dynamic_internal(world, handle.id(), &mut entity_map) {
                Ok(_) => {
                    let instance_info = InstanceInfo { entity_map, parent };
                    Self::set_scene_instance_parent_sync(world, &instance_info);

                    self.spawned_instances.insert(instance_id, instance_info);
                    let spawned = self.spawned_dynamic_scenes.entry(handle.id()).or_default();
                    spawned.insert(instance_id);
                    // We trigger `SceneInstanceReady` events after processing all scenes
                    // SceneSpawner may not be available in the observer.
                    self.instances_ready.push((instance_id, parent));
                }
                Err(SceneSpawnError::NonExistentScene { .. }) => {
                    self.dynamic_scenes_to_spawn
                        .push((handle, instance_id, parent));
                }
                Err(err) => return Err(err),
            }
        }

        let scenes_to_spawn = core::mem::take(&mut self.scenes_to_spawn);

        for (scene_handle, instance_id, parent) in scenes_to_spawn {
            let mut entity_map = EntityHashMap::default();

            match Self::spawn_sync_internal(world, scene_handle.id(), &mut entity_map) {
                Ok(_) => {
                    let instance_info = InstanceInfo { entity_map, parent };
                    Self::set_scene_instance_parent_sync(world, &instance_info);

                    self.spawned_instances.insert(instance_id, instance_info);
                    let spawned = self.spawned_scenes.entry(scene_handle.id()).or_default();
                    spawned.insert(instance_id);

                    // We trigger `SceneInstanceReady` events after processing all scenes
                    // SceneSpawner may not be available in the observer.
                    self.instances_ready.push((instance_id, parent));
                }
                Err(SceneSpawnError::NonExistentRealScene { .. }) => {
                    self.scenes_to_spawn
                        .push((scene_handle, instance_id, parent));
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    fn set_scene_instance_parent_sync(world: &mut World, instance: &InstanceInfo) {
        let Some(parent) = instance.parent else {
            return;
        };
        for &entity in instance.entity_map.values() {
            // Add the `ChildOf` component to the scene root, and update the `Children` component of
            // the scene parent
            if !world
                .get_entity(entity)
                .ok()
                // This will filter only the scene root entity, as all other from the
                // scene have a parent
                // Entities that wouldn't exist anymore are also skipped
                // this case shouldn't happen anyway
                .is_none_or(|entity| entity.contains::<ChildOf>())
            {
                world.entity_mut(parent).add_child(entity);
            }
        }
    }

    fn trigger_scene_ready_events(&mut self, world: &mut World) {
        for (instance_id, parent) in self.instances_ready.drain(..) {
            if let Some(parent) = parent {
                // Defer via commands otherwise SceneSpawner is not available in the observer.
                world.commands().trigger(SceneInstanceReady {
                    instance_id,
                    entity: parent,
                });
            } else {
                // Defer via commands otherwise SceneSpawner is not available in the observer.
                // TODO: triggering this for PLACEHOLDER is suboptimal, but this scene system is on
                // its way out, so lets avoid breaking people by making a second event.
                world.commands().trigger(SceneInstanceReady {
                    instance_id,
                    entity: Entity::PLACEHOLDER,
                });
            }
        }
    }

    /// Check that a scene instance spawned previously is ready to use
    pub fn instance_is_ready(&self, instance_id: InstanceId) -> bool {
        self.spawned_instances.contains_key(&instance_id)
    }

    /// Get an iterator over the entities in an instance, once it's spawned.
    ///
    /// Before the scene is spawned, the iterator will be empty. Use [`Self::instance_is_ready`]
    /// to check if the instance is ready.
    pub fn iter_instance_entities(
        &'_ self,
        instance_id: InstanceId,
    ) -> impl Iterator<Item = Entity> + '_ {
        self.spawned_instances
            .get(&instance_id)
            .map(|instance| instance.entity_map.values())
            .into_iter()
            .flatten()
            .copied()
    }
}

/// System that handles scheduled scene instance spawning and despawning through a [`SceneSpawner`].
pub fn scene_spawner_system(world: &mut World) {
    world.resource_scope(|world, mut scene_spawner: Mut<SceneSpawner>| {
        // remove any loading instances where parent is deleted
        let is_parent_alive = |parent: &Option<Entity>| {
            parent
                .map(|parent| world.get_entity(parent).is_ok())
                .unwrap_or(true) // If we don't have a parent, then consider the parent alive.
        };
        scene_spawner
            .dynamic_scenes_to_spawn
            .retain(|(_, _, parent)| is_parent_alive(parent));
        scene_spawner
            .scenes_to_spawn
            .retain(|(_, _, parent)| is_parent_alive(parent));

        let scene_asset_events = world.resource::<Messages<AssetEvent<Scene>>>();
        let dynamic_scene_asset_events = world.resource::<Messages<AssetEvent<DynamicScene>>>();
        let scene_spawner = &mut *scene_spawner;

        let mut updated_spawned_scenes = Vec::new();
        for event in scene_spawner
            .scene_asset_event_reader
            .read(scene_asset_events)
        {
            match event {
                AssetEvent::Added { id } => {
                    scene_spawner.debounced_scene_asset_events.insert(*id, 0);
                }
                AssetEvent::Modified { id } => {
                    if scene_spawner
                        .debounced_scene_asset_events
                        .insert(*id, 0)
                        .is_none()
                        && scene_spawner.spawned_scenes.contains_key(id)
                    {
                        updated_spawned_scenes.push(*id);
                    }
                }
                _ => {}
            }
        }
        let mut updated_spawned_dynamic_scenes = Vec::new();
        for event in scene_spawner
            .dynamic_scene_asset_event_reader
            .read(dynamic_scene_asset_events)
        {
            match event {
                AssetEvent::Added { id } => {
                    scene_spawner
                        .debounced_dynamic_scene_asset_events
                        .insert(*id, 0);
                }
                AssetEvent::Modified { id } => {
                    if scene_spawner
                        .debounced_dynamic_scene_asset_events
                        .insert(*id, 0)
                        .is_none()
                        && scene_spawner.spawned_dynamic_scenes.contains_key(id)
                    {
                        updated_spawned_dynamic_scenes.push(*id);
                    }
                }
                _ => {}
            }
        }

        scene_spawner.despawn_queued_scenes(world).unwrap();
        scene_spawner.despawn_queued_instances(world);
        scene_spawner
            .spawn_queued_scenes(world)
            .unwrap_or_else(|err| panic!("{}", err));
        scene_spawner
            .update_spawned_scenes(world, &updated_spawned_scenes)
            .unwrap();
        scene_spawner
            .update_spawned_dynamic_scenes(world, &updated_spawned_dynamic_scenes)
            .unwrap();
        scene_spawner.trigger_scene_ready_events(world);

        const SCENE_ASSET_AGE_THRESHOLD: u32 = 2;
        for asset_id in scene_spawner.debounced_scene_asset_events.clone().keys() {
            let age = scene_spawner
                .debounced_scene_asset_events
                .get(asset_id)
                .unwrap();
            if *age > SCENE_ASSET_AGE_THRESHOLD {
                scene_spawner.debounced_scene_asset_events.remove(asset_id);
            } else {
                scene_spawner
                    .debounced_scene_asset_events
                    .insert(*asset_id, *age + 1);
            }
        }
        for asset_id in scene_spawner
            .debounced_dynamic_scene_asset_events
            .clone()
            .keys()
        {
            let age = scene_spawner
                .debounced_dynamic_scene_asset_events
                .get(asset_id)
                .unwrap();
            if *age > SCENE_ASSET_AGE_THRESHOLD {
                scene_spawner
                    .debounced_dynamic_scene_asset_events
                    .remove(asset_id);
            } else {
                scene_spawner
                    .debounced_dynamic_scene_asset_events
                    .insert(*asset_id, *age + 1);
            }
        }
    });
}

/// [`InstanceId`] of a spawned scene. It can be used with the [`SceneSpawner`] to
/// interact with the spawned scene.
#[derive(Component, Deref, DerefMut)]
pub struct SceneInstance(pub(crate) InstanceId);

/// System that will spawn scenes from the [`SceneRoot`] and [`DynamicSceneRoot`] components.
pub fn scene_spawner(
    mut commands: Commands,
    mut scene_to_spawn: Query<
        (Entity, &SceneRoot, Option<&mut SceneInstance>),
        (Changed<SceneRoot>, Without<DynamicSceneRoot>),
    >,
    mut dynamic_scene_to_spawn: Query<
        (Entity, &DynamicSceneRoot, Option<&mut SceneInstance>),
        (Changed<DynamicSceneRoot>, Without<SceneRoot>),
    >,
    mut scene_spawner: ResMut<SceneSpawner>,
) {
    for (entity, scene, instance) in &mut scene_to_spawn {
        let new_instance = scene_spawner.spawn_as_child(scene.0.clone(), entity);
        if let Some(mut old_instance) = instance {
            scene_spawner.despawn_instance(**old_instance);
            *old_instance = SceneInstance(new_instance);
        } else {
            commands.entity(entity).insert(SceneInstance(new_instance));
        }
    }
    for (entity, dynamic_scene, instance) in &mut dynamic_scene_to_spawn {
        let new_instance = scene_spawner.spawn_dynamic_as_child(dynamic_scene.0.clone(), entity);
        if let Some(mut old_instance) = instance {
            scene_spawner.despawn_instance(**old_instance);
            *old_instance = SceneInstance(new_instance);
        } else {
            commands.entity(entity).insert(SceneInstance(new_instance));
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_asset::{AssetPlugin, AssetServer, Handle};
    use bevy_ecs::{
        component::Component,
        hierarchy::Children,
        observer::On,
        prelude::ReflectComponent,
        query::With,
        system::{Commands, Query, Res, ResMut, RunSystemOnce},
    };
    use bevy_reflect::Reflect;

    use crate::{DynamicSceneBuilder, DynamicSceneRoot, ScenePlugin};

    use super::*;
    use crate::{DynamicScene, SceneSpawner};
    use bevy_app::ScheduleRunnerPlugin;
    use bevy_asset::Assets;
    use bevy_ecs::{
        entity::Entity,
        prelude::{AppTypeRegistry, World},
    };

    #[derive(Component, Reflect, Default)]
    #[reflect(Component)]
    struct ComponentA {
        pub x: f32,
        pub y: f32,
    }

    #[test]
    fn spawn_and_delete() {
        let mut app = App::new();

        app.add_plugins(ScheduleRunnerPlugin::default())
            .add_plugins(AssetPlugin::default())
            .add_plugins(ScenePlugin);
        app.update();

        let mut scene_world = World::new();

        // create a new DynamicScene manually
        let type_registry = app.world().resource::<AppTypeRegistry>().clone();
        scene_world.insert_resource(type_registry);
        scene_world.spawn(ComponentA { x: 3.0, y: 4.0 });
        let scene = DynamicScene::from_world(&scene_world);
        let scene_handle = app
            .world_mut()
            .resource_mut::<Assets<DynamicScene>>()
            .add(scene);

        // spawn the scene as a child of `entity` using `DynamicSceneRoot`
        let entity = app
            .world_mut()
            .spawn(DynamicSceneRoot(scene_handle.clone()))
            .id();

        // run the app's schedule once, so that the scene gets spawned
        app.update();

        // make sure that the scene was added as a child of the root entity
        let (scene_entity, scene_component_a) = app
            .world_mut()
            .query::<(Entity, &ComponentA)>()
            .single(app.world())
            .unwrap();
        assert_eq!(scene_component_a.x, 3.0);
        assert_eq!(scene_component_a.y, 4.0);
        assert_eq!(
            app.world().entity(entity).get::<Children>().unwrap().len(),
            1
        );

        // let's try to delete the scene
        let mut scene_spawner = app.world_mut().resource_mut::<SceneSpawner>();
        scene_spawner.despawn_dynamic(&scene_handle);

        // run the scene spawner system to despawn the scene
        app.update();

        // the scene entity does not exist anymore
        assert!(app.world().get_entity(scene_entity).is_err());

        // the root entity does not have any children anymore
        assert!(app.world().entity(entity).get::<Children>().is_none());
    }

    #[derive(Reflect, Component, Debug, PartialEq, Eq, Clone, Copy, Default)]
    #[reflect(Component)]
    struct A(usize);

    #[test]
    fn clone_dynamic_entities() {
        let mut world = World::default();

        // setup
        let atr = AppTypeRegistry::default();
        atr.write().register::<A>();
        world.insert_resource(atr);
        world.insert_resource(Assets::<DynamicScene>::default());

        // start test
        world.spawn(A(42));

        assert_eq!(world.query::<&A>().iter(&world).len(), 1);

        // clone only existing entity
        let mut scene_spawner = SceneSpawner::default();
        let entity = world
            .query_filtered::<Entity, With<A>>()
            .single(&world)
            .unwrap();
        let scene = DynamicSceneBuilder::from_world(&world)
            .extract_entity(entity)
            .build();

        let scene_id = world.resource_mut::<Assets<DynamicScene>>().add(scene);
        let instance_id = scene_spawner
            .spawn_dynamic_sync(&mut world, &scene_id)
            .unwrap();

        // verify we spawned exactly one new entity with our expected component
        assert_eq!(world.query::<&A>().iter(&world).len(), 2);

        // verify that we can get this newly-spawned entity by the instance ID
        let new_entity = scene_spawner
            .iter_instance_entities(instance_id)
            .next()
            .unwrap();

        // verify this is not the original entity
        assert_ne!(entity, new_entity);

        // verify this new entity contains the same data as the original entity
        let [old_a, new_a] = world
            .query::<&A>()
            .get_many(&world, [entity, new_entity])
            .unwrap();
        assert_eq!(old_a, new_a);
    }

    #[derive(Component, Reflect, Default)]
    #[reflect(Component)]
    struct ComponentF;

    #[derive(Resource, Default)]
    struct TriggerCount(u32);

    fn setup() -> App {
        let mut app = App::new();
        app.add_plugins((AssetPlugin::default(), ScenePlugin));
        app.init_resource::<TriggerCount>();

        app.register_type::<ComponentF>();
        app.world_mut().spawn(ComponentF);
        app.world_mut().spawn(ComponentF);

        app
    }

    fn build_scene(app: &mut App) -> Handle<Scene> {
        app.world_mut()
            .run_system_once(
                |world: &World,
                 type_registry: Res<'_, AppTypeRegistry>,
                 asset_server: Res<'_, AssetServer>| {
                    asset_server.add(
                        Scene::from_dynamic_scene(&DynamicScene::from_world(world), &type_registry)
                            .unwrap(),
                    )
                },
            )
            .expect("Failed to run scene builder system.")
    }

    fn build_dynamic_scene(app: &mut App) -> Handle<DynamicScene> {
        app.world_mut()
            .run_system_once(|world: &World, asset_server: Res<'_, AssetServer>| {
                asset_server.add(DynamicScene::from_world(world))
            })
            .expect("Failed to run dynamic scene builder system.")
    }

    fn observe_trigger(app: &mut App, scene_id: InstanceId, scene_entity: Option<Entity>) {
        // Add observer
        app.world_mut().add_observer(
            move |event: On<SceneInstanceReady>,
                  scene_spawner: Res<SceneSpawner>,
                  mut trigger_count: ResMut<TriggerCount>| {
                assert_eq!(
                    event.event().instance_id,
                    scene_id,
                    "`SceneInstanceReady` contains the wrong `InstanceId`"
                );
                assert_eq!(
                    event.event_target(),
                    scene_entity.unwrap_or(Entity::PLACEHOLDER),
                    "`SceneInstanceReady` triggered on the wrong parent entity"
                );
                assert!(
                    scene_spawner.instance_is_ready(event.event().instance_id),
                    "`InstanceId` is not ready"
                );
                trigger_count.0 += 1;
            },
        );

        // Check observer is triggered once.
        app.update();
        app.world_mut()
            .run_system_once(|trigger_count: Res<TriggerCount>| {
                assert_eq!(
                    trigger_count.0, 1,
                    "wrong number of `SceneInstanceReady` triggers"
                );
            })
            .unwrap();
    }

    #[test]
    fn observe_scene() {
        let mut app = setup();

        // Build scene.
        let scene = build_scene(&mut app);

        // Spawn scene.
        let scene_id = app
            .world_mut()
            .run_system_once(move |mut scene_spawner: ResMut<'_, SceneSpawner>| {
                scene_spawner.spawn(scene.clone())
            })
            .unwrap();

        // Check trigger.
        observe_trigger(&mut app, scene_id, None);
    }

    #[test]
    fn observe_dynamic_scene() {
        let mut app = setup();

        // Build scene.
        let scene = build_dynamic_scene(&mut app);

        // Spawn scene.
        let scene_id = app
            .world_mut()
            .run_system_once(move |mut scene_spawner: ResMut<'_, SceneSpawner>| {
                scene_spawner.spawn_dynamic(scene.clone())
            })
            .unwrap();

        // Check trigger.
        observe_trigger(&mut app, scene_id, None);
    }

    #[test]
    fn observe_scene_as_child() {
        let mut app = setup();

        // Build scene.
        let scene = build_scene(&mut app);

        // Spawn scene as child.
        let (scene_id, scene_entity) = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands<'_, '_>,
                      mut scene_spawner: ResMut<'_, SceneSpawner>| {
                    let entity = commands.spawn_empty().id();
                    let id = scene_spawner.spawn_as_child(scene.clone(), entity);
                    (id, entity)
                },
            )
            .unwrap();

        // Check trigger.
        observe_trigger(&mut app, scene_id, Some(scene_entity));
    }

    #[test]
    fn observe_dynamic_scene_as_child() {
        let mut app = setup();

        // Build scene.
        let scene = build_dynamic_scene(&mut app);

        // Spawn scene as child.
        let (scene_id, scene_entity) = app
            .world_mut()
            .run_system_once(
                move |mut commands: Commands<'_, '_>,
                      mut scene_spawner: ResMut<'_, SceneSpawner>| {
                    let entity = commands.spawn_empty().id();
                    let id = scene_spawner.spawn_dynamic_as_child(scene.clone(), entity);
                    (id, entity)
                },
            )
            .unwrap();

        // Check trigger.
        observe_trigger(&mut app, scene_id, Some(scene_entity));
    }

    #[test]
    fn despawn_scene() {
        let mut app = App::new();
        app.add_plugins((AssetPlugin::default(), ScenePlugin));
        app.register_type::<ComponentF>();

        let asset_server = app.world().resource::<AssetServer>();

        // Build scene.
        let scene = asset_server.add(DynamicScene::default());
        let count = 10;

        // Checks the number of scene instances stored in `SceneSpawner`.
        let check = |world: &mut World, expected_count: usize| {
            let scene_spawner = world.resource::<SceneSpawner>();
            assert_eq!(
                scene_spawner.spawned_dynamic_scenes[&scene.id()].len(),
                expected_count
            );
            assert_eq!(scene_spawner.spawned_instances.len(), expected_count);
        };

        // Spawn scene.
        for _ in 0..count {
            app.world_mut()
                .spawn((ComponentF, DynamicSceneRoot(scene.clone())));
        }

        app.update();
        check(app.world_mut(), count);

        // Despawn scene.
        app.world_mut()
            .run_system_once(
                |mut commands: Commands, query: Query<Entity, With<ComponentF>>| {
                    for entity in query.iter() {
                        commands.entity(entity).despawn();
                    }
                },
            )
            .unwrap();

        app.update();
        check(app.world_mut(), 0);
    }

    #[test]
    fn scene_child_order_preserved_when_archetype_order_mismatched() {
        let mut app = App::new();

        app.add_plugins(ScheduleRunnerPlugin::default())
            .add_plugins(AssetPlugin::default())
            .add_plugins(ScenePlugin)
            .register_type::<ComponentA>()
            .register_type::<ComponentF>();
        app.update();

        let mut scene_world = World::new();
        let root = scene_world.spawn_empty().id();
        let temporary_root = scene_world.spawn_empty().id();
        // Spawn entities with different parent first before parenting them to the actual root, allowing us
        // to decouple child order from archetype-creation-order
        let child1 = scene_world
            .spawn((ChildOf(temporary_root), ComponentA { x: 1.0, y: 1.0 }))
            .id();
        let child2 = scene_world
            .spawn((ChildOf(temporary_root), ComponentA { x: 2.0, y: 2.0 }))
            .id();
        // the "first" child is intentionally spawned with a different component to force it into a "newer" archetype,
        // meaning it will be iterated later in the spawn code.
        let child0 = scene_world
            .spawn((ChildOf(temporary_root), ComponentF))
            .id();

        scene_world
            .entity_mut(root)
            .add_children(&[child0, child1, child2]);

        let scene = Scene::new(scene_world);
        let scene_handle = app.world_mut().resource_mut::<Assets<Scene>>().add(scene);

        let spawned = app.world_mut().spawn(SceneRoot(scene_handle.clone())).id();

        app.update();
        let world = app.world_mut();

        let spawned_root = world.entity(spawned).get::<Children>().unwrap()[1];
        let children = world.entity(spawned_root).get::<Children>().unwrap();
        assert_eq!(children.len(), 3);
        assert!(world.entity(children[0]).get::<ComponentF>().is_some());
        assert_eq!(
            world.entity(children[1]).get::<ComponentA>().unwrap().x,
            1.0
        );
        assert_eq!(
            world.entity(children[2]).get::<ComponentA>().unwrap().x,
            2.0
        );
    }
}
