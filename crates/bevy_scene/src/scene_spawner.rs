use crate::{DynamicScene, Scene};
use bevy_app::prelude::*;
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::{Entity, EntityMap, Resources, World};
use bevy_reflect::{ReflectComponent, ReflectMapEntities, TypeRegistryArc};
use bevy_transform::prelude::Parent;
use bevy_utils::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug)]
struct InstanceInfo {
    entity_map: EntityMap,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct InstanceId(Uuid);

impl InstanceId {
    fn new() -> Self {
        InstanceId(Uuid::new_v4())
    }
}

#[derive(Default)]
pub struct SceneSpawner {
    spawned_scenes: HashMap<Handle<Scene>, Vec<InstanceId>>,
    spawned_dynamic_scenes: HashMap<Handle<DynamicScene>, Vec<InstanceId>>,
    spawned_instances: HashMap<InstanceId, InstanceInfo>,
    scene_asset_event_reader: EventReader<AssetEvent<DynamicScene>>,
    dynamic_scenes_to_spawn: Vec<Handle<DynamicScene>>,
    scenes_to_spawn: Vec<(Handle<Scene>, InstanceId)>,
    scenes_to_despawn: Vec<Handle<DynamicScene>>,
    scenes_with_parent: Vec<(InstanceId, Entity)>,
}

#[derive(Error, Debug)]
pub enum SceneSpawnError {
    #[error("scene contains the unregistered component `{type_name}`. consider adding `#[reflect(Component)]` to your type")]
    UnregisteredComponent { type_name: String },
    #[error("scene contains the unregistered type `{type_name}`. consider registering the type using `app.register_type::<T>()`")]
    UnregisteredType { type_name: String },
    #[error("scene does not exist")]
    NonExistentScene { handle: Handle<DynamicScene> },
    #[error("scene does not exist")]
    NonExistentRealScene { handle: Handle<Scene> },
}

impl SceneSpawner {
    pub fn spawn_dynamic(&mut self, scene_handle: Handle<DynamicScene>) {
        self.dynamic_scenes_to_spawn.push(scene_handle);
    }

    pub fn spawn(&mut self, scene_handle: Handle<Scene>) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn.push((scene_handle, instance_id));
        instance_id
    }

    pub fn spawn_as_child(&mut self, scene_handle: Handle<Scene>, parent: Entity) -> InstanceId {
        let instance_id = InstanceId::new();
        self.scenes_to_spawn.push((scene_handle, instance_id));
        self.scenes_with_parent.push((instance_id, parent));
        instance_id
    }

    pub fn despawn(&mut self, scene_handle: Handle<DynamicScene>) {
        self.scenes_to_despawn.push(scene_handle);
    }

    pub fn despawn_sync(
        &mut self,
        world: &mut World,
        scene_handle: Handle<DynamicScene>,
    ) -> Result<(), SceneSpawnError> {
        if let Some(instance_ids) = self.spawned_dynamic_scenes.get(&scene_handle) {
            for instance_id in instance_ids {
                if let Some(instance) = self.spawned_instances.get(&instance_id) {
                    for entity in instance.entity_map.values() {
                        let _ = world.despawn(entity); // Ignore the result, despawn only cares if it exists.
                    }
                }
            }

            self.spawned_dynamic_scenes.remove(&scene_handle);
        }
        Ok(())
    }

    pub fn spawn_dynamic_sync(
        &mut self,
        world: &mut World,
        resources: &Resources,
        scene_handle: &Handle<DynamicScene>,
    ) -> Result<(), SceneSpawnError> {
        let instance_id = InstanceId::new();
        let mut instance_info = InstanceInfo {
            entity_map: EntityMap::default(),
        };
        Self::spawn_dynamic_internal(world, resources, scene_handle, &mut instance_info)?;
        self.spawned_instances.insert(instance_id, instance_info);
        let spawned = self
            .spawned_dynamic_scenes
            .entry(scene_handle.clone())
            .or_insert_with(Vec::new);
        spawned.push(instance_id);
        Ok(())
    }

    fn spawn_dynamic_internal(
        world: &mut World,
        resources: &Resources,
        scene_handle: &Handle<DynamicScene>,
        instance_info: &mut InstanceInfo,
    ) -> Result<(), SceneSpawnError> {
        let type_registry = resources.get::<TypeRegistryArc>().unwrap();
        let type_registry = type_registry.read();
        let scenes = resources.get::<Assets<DynamicScene>>().unwrap();
        let scene = scenes
            .get(scene_handle)
            .ok_or_else(|| SceneSpawnError::NonExistentScene {
                handle: scene_handle.clone_weak(),
            })?;

        for scene_entity in scene.entities.iter() {
            let entity = *instance_info
                .entity_map
                // TODO: use Entity type directly in scenes to properly encode generation / avoid the need to patch things up?
                .entry(bevy_ecs::Entity::new(scene_entity.entity))
                .or_insert_with(|| world.reserve_entity());
            for component in scene_entity.components.iter() {
                let registration = type_registry
                    .get_with_name(component.type_name())
                    .ok_or_else(|| SceneSpawnError::UnregisteredType {
                        type_name: component.type_name().to_string(),
                    })?;
                let reflect_component =
                    registration.data::<ReflectComponent>().ok_or_else(|| {
                        SceneSpawnError::UnregisteredComponent {
                            type_name: component.type_name().to_string(),
                        }
                    })?;
                if world.has_component_type(entity, registration.type_id()) {
                    if registration.short_name() != "Camera" {
                        reflect_component.apply_component(world, entity, &**component);
                    }
                } else {
                    reflect_component.add_component(world, resources, entity, &**component);
                }
            }
        }
        Ok(())
    }

    pub fn spawn_sync(
        &mut self,
        world: &mut World,
        resources: &Resources,
        scene_handle: Handle<Scene>,
    ) -> Result<InstanceId, SceneSpawnError> {
        self.spawn_sync_internal(world, resources, scene_handle, InstanceId::new())
    }

    fn spawn_sync_internal(
        &mut self,
        world: &mut World,
        resources: &Resources,
        scene_handle: Handle<Scene>,
        instance_id: InstanceId,
    ) -> Result<InstanceId, SceneSpawnError> {
        let mut instance_info = InstanceInfo {
            entity_map: EntityMap::default(),
        };
        let type_registry = resources.get::<TypeRegistryArc>().unwrap();
        let type_registry = type_registry.read();
        let scenes = resources.get::<Assets<Scene>>().unwrap();
        let scene =
            scenes
                .get(&scene_handle)
                .ok_or_else(|| SceneSpawnError::NonExistentRealScene {
                    handle: scene_handle.clone(),
                })?;

        for archetype in scene.world.archetypes() {
            for scene_entity in archetype.iter_entities() {
                let entity = *instance_info
                    .entity_map
                    .entry(*scene_entity)
                    .or_insert_with(|| world.reserve_entity());
                for type_info in archetype.types() {
                    let registration = type_registry.get(type_info.id()).ok_or_else(|| {
                        SceneSpawnError::UnregisteredType {
                            type_name: type_info.type_name().to_string(),
                        }
                    })?;
                    let reflect_component =
                        registration.data::<ReflectComponent>().ok_or_else(|| {
                            SceneSpawnError::UnregisteredComponent {
                                type_name: registration.name().to_string(),
                            }
                        })?;
                    reflect_component.copy_component(
                        &scene.world,
                        world,
                        resources,
                        *scene_entity,
                        entity,
                    );
                }
            }
        }
        for registration in type_registry.iter() {
            if let Some(map_entities_reflect) = registration.data::<ReflectMapEntities>() {
                map_entities_reflect
                    .map_entities(world, &instance_info.entity_map)
                    .unwrap();
            }
        }
        self.spawned_instances.insert(instance_id, instance_info);
        let spawned = self
            .spawned_scenes
            .entry(scene_handle)
            .or_insert_with(Vec::new);
        spawned.push(instance_id);
        Ok(instance_id)
    }

    pub fn update_spawned_scenes(
        &mut self,
        world: &mut World,
        resources: &Resources,
        scene_handles: &[Handle<DynamicScene>],
    ) -> Result<(), SceneSpawnError> {
        for scene_handle in scene_handles {
            if let Some(spawned_instances) = self.spawned_dynamic_scenes.get(scene_handle) {
                for instance_id in spawned_instances.iter() {
                    if let Some(instance_info) = self.spawned_instances.get_mut(instance_id) {
                        Self::spawn_dynamic_internal(
                            world,
                            resources,
                            scene_handle,
                            instance_info,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn despawn_queued_scenes(&mut self, world: &mut World) -> Result<(), SceneSpawnError> {
        let scenes_to_despawn = std::mem::take(&mut self.scenes_to_despawn);

        for scene_handle in scenes_to_despawn {
            self.despawn_sync(world, scene_handle)?;
        }
        Ok(())
    }

    pub fn spawn_queued_scenes(
        &mut self,
        world: &mut World,
        resources: &Resources,
    ) -> Result<(), SceneSpawnError> {
        let scenes_to_spawn = std::mem::take(&mut self.dynamic_scenes_to_spawn);

        for scene_handle in scenes_to_spawn {
            match self.spawn_dynamic_sync(world, resources, &scene_handle) {
                Ok(_) => {}
                Err(SceneSpawnError::NonExistentScene { .. }) => {
                    self.dynamic_scenes_to_spawn.push(scene_handle)
                }
                Err(err) => return Err(err),
            }
        }

        let scenes_to_spawn = std::mem::take(&mut self.scenes_to_spawn);

        for (scene_handle, instance_id) in scenes_to_spawn {
            match self.spawn_sync_internal(world, resources, scene_handle, instance_id) {
                Ok(_) => {}
                Err(SceneSpawnError::NonExistentRealScene { handle }) => {
                    self.scenes_to_spawn.push((handle, instance_id))
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
    }

    pub(crate) fn set_scene_instance_parent_sync(&mut self, world: &mut World) {
        let scenes_with_parent = std::mem::take(&mut self.scenes_with_parent);

        for (instance_id, parent) in scenes_with_parent {
            if let Some(instance) = self.spawned_instances.get(&instance_id) {
                for entity in instance.entity_map.values() {
                    if let Err(bevy_ecs::ComponentError::MissingComponent(_)) =
                        world.get::<Parent>(entity)
                    {
                        let _ = world.insert_one(entity, Parent(parent));
                    }
                }
            } else {
                self.scenes_with_parent.push((instance_id, parent));
            }
        }
    }

    /// Check that an scene instance spawned previously is ready to use
    pub fn instance_is_ready(&self, instance_id: InstanceId) -> bool {
        self.spawned_instances.contains_key(&instance_id)
    }

    /// Get an iterator over the entities in an instance, once it's spawned
    pub fn iter_instance_entities(
        &'_ self,
        instance_id: InstanceId,
    ) -> Option<impl Iterator<Item = Entity> + '_> {
        self.spawned_instances
            .get(&instance_id)
            .map(|instance| instance.entity_map.values())
    }
}

pub fn scene_spawner_system(world: &mut World, resources: &mut Resources) {
    let mut scene_spawner = resources.get_mut::<SceneSpawner>().unwrap();
    let scene_asset_events = resources.get::<Events<AssetEvent<DynamicScene>>>().unwrap();

    let mut updated_spawned_scenes = Vec::new();
    for event in scene_spawner
        .scene_asset_event_reader
        .iter(&scene_asset_events)
    {
        if let AssetEvent::Modified { handle } = event {
            if scene_spawner.spawned_dynamic_scenes.contains_key(handle) {
                updated_spawned_scenes.push(handle.clone_weak());
            }
        }
    }

    scene_spawner.despawn_queued_scenes(world).unwrap();
    scene_spawner
        .spawn_queued_scenes(world, resources)
        .unwrap_or_else(|err| panic!("{}", err));
    scene_spawner
        .update_spawned_scenes(world, resources, &updated_spawned_scenes)
        .unwrap();
    scene_spawner.set_scene_instance_parent_sync(world);
}
