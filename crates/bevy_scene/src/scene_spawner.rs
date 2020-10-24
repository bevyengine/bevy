use crate::{DynamicScene, Scene};
use bevy_app::prelude::*;
use bevy_asset::{AssetEvent, Assets, Handle};
#[cfg(feature = "dynamic-api")]
use bevy_ecs::ComponentId;
use bevy_ecs::{EntityMap, Resources, World};
use bevy_type_registry::TypeRegistry;
use bevy_utils::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug)]
struct InstanceInfo {
    entity_map: EntityMap,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct InstanceId(Uuid);

impl InstanceId {
    pub fn new() -> Self {
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
    scenes_to_spawn: Vec<Handle<Scene>>,
    scenes_to_despawn: Vec<Handle<DynamicScene>>,
}

#[derive(Error, Debug)]
pub enum SceneSpawnError {
    #[error("Scene contains an unregistered component.")]
    UnregisteredComponent { type_name: String },
    #[error("Scene does not exist. Perhaps it is still loading?")]
    NonExistentScene { handle: Handle<DynamicScene> },
    #[error("Scene does not exist. Perhaps it is still loading?")]
    NonExistentRealScene { handle: Handle<Scene> },
}

impl SceneSpawner {
    pub fn spawn_dynamic(&mut self, scene_handle: Handle<DynamicScene>) {
        self.dynamic_scenes_to_spawn.push(scene_handle);
    }

    pub fn spawn(&mut self, scene_handle: Handle<Scene>) {
        self.scenes_to_spawn.push(scene_handle);
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
        let type_registry = resources.get::<TypeRegistry>().unwrap();
        let component_registry = type_registry.component.read();
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
                let component_registration = component_registry
                    .get_with_name(&component.type_name)
                    .ok_or(SceneSpawnError::UnregisteredComponent {
                        type_name: component.type_name.to_string(),
                    })?;
                if world.has_component_type(entity, component_registration.ty.into()) {
                    if component.type_name != "Camera" {
                        component_registration.apply_property_to_entity(world, entity, component);
                    }
                } else {
                    component_registration
                        .add_property_to_entity(world, resources, entity, component);
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
    ) -> Result<(), SceneSpawnError> {
        let instance_id = InstanceId::new();
        let mut instance_info = InstanceInfo {
            entity_map: EntityMap::default(),
        };
        let type_registry = resources.get::<TypeRegistry>().unwrap();
        let component_registry = type_registry.component.read();
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
                    #[cfg(feature = "dynamic-api")]
                    let id = match type_info.id() {
                        ComponentId::RustTypeId(id) => id,
                        ComponentId::ExternalId(_) => {
                            todo!("Handle external type ids in Bevy scene")
                        }
                    };
                    #[cfg(not(feature = "dynamic-api"))]
                    let id = type_info.id().0;

                    if let Some(component_registration) = component_registry.get(&id) {
                        component_registration.component_copy(
                            &scene.world,
                            world,
                            resources,
                            *scene_entity,
                            entity,
                        );
                    }
                }
            }
        }
        for component_registration in component_registry.iter() {
            component_registration
                .map_entities(world, &instance_info.entity_map)
                .unwrap();
        }
        self.spawned_instances.insert(instance_id, instance_info);
        let spawned = self
            .spawned_scenes
            .entry(scene_handle)
            .or_insert_with(Vec::new);
        spawned.push(instance_id);
        Ok(())
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

        for scene_handle in scenes_to_spawn {
            match self.spawn_sync(world, resources, scene_handle) {
                Ok(_) => {}
                Err(SceneSpawnError::NonExistentRealScene { handle }) => {
                    self.scenes_to_spawn.push(handle)
                }
                Err(err) => return Err(err),
            }
        }

        Ok(())
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
    scene_spawner.spawn_queued_scenes(world, resources).unwrap();
    scene_spawner
        .update_spawned_scenes(world, resources, &updated_spawned_scenes)
        .unwrap();
}
