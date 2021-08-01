use crate::{DynamicScene, Scene};
use bevy_app::{Events, ManualEventReader};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::{
    entity::{Entity, EntityMap},
    reflect::{ReflectComponent, ReflectMapEntities},
    world::{Mut, World},
};
use bevy_reflect::TypeRegistryArc;
use bevy_transform::prelude::Parent;
use bevy_utils::{tracing::error, HashMap};
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
    scene_asset_event_reader: ManualEventReader<AssetEvent<DynamicScene>>,
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
                if let Some(instance) = self.spawned_instances.get(instance_id) {
                    for entity in instance.entity_map.values() {
                        let _ = world.despawn(entity); // Ignore the result, despawn only cares if
                                                       // it exists.
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
        scene_handle: &Handle<DynamicScene>,
    ) -> Result<(), SceneSpawnError> {
        let mut entity_map = EntityMap::default();
        Self::spawn_dynamic_internal(world, scene_handle, &mut entity_map)?;
        let instance_id = InstanceId::new();
        self.spawned_instances
            .insert(instance_id, InstanceInfo { entity_map });
        let spawned = self
            .spawned_dynamic_scenes
            .entry(scene_handle.clone())
            .or_insert_with(Vec::new);
        spawned.push(instance_id);
        Ok(())
    }

    fn spawn_dynamic_internal(
        world: &mut World,
        scene_handle: &Handle<DynamicScene>,
        entity_map: &mut EntityMap,
    ) -> Result<(), SceneSpawnError> {
        world.resource_scope(|world, scenes: Mut<Assets<DynamicScene>>| {
            let scene =
                scenes
                    .get(scene_handle)
                    .ok_or_else(|| SceneSpawnError::NonExistentScene {
                        handle: scene_handle.clone_weak(),
                    })?;
            scene.write_to_world(world, entity_map)
        })
    }

    pub fn spawn_sync(
        &mut self,
        world: &mut World,
        scene_handle: Handle<Scene>,
    ) -> Result<InstanceId, SceneSpawnError> {
        self.spawn_sync_internal(world, scene_handle, InstanceId::new())
    }

    fn spawn_sync_internal(
        &mut self,
        world: &mut World,
        scene_handle: Handle<Scene>,
        instance_id: InstanceId,
    ) -> Result<InstanceId, SceneSpawnError> {
        let mut instance_info = InstanceInfo {
            entity_map: EntityMap::default(),
        };
        let type_registry = world.get_resource::<TypeRegistryArc>().unwrap().clone();
        let type_registry = type_registry.read();
        world.resource_scope(|world, scenes: Mut<Assets<Scene>>| {
            let scene =
                scenes
                    .get(&scene_handle)
                    .ok_or_else(|| SceneSpawnError::NonExistentRealScene {
                        handle: scene_handle.clone(),
                    })?;

            for archetype in scene.world.archetypes().iter() {
                for scene_entity in archetype.entities() {
                    let entity = *instance_info
                        .entity_map
                        .entry(*scene_entity)
                        .or_insert_with(|| world.spawn().id());
                    for component_id in archetype.components() {
                        let component_info = scene
                            .world
                            .components()
                            .get_info(component_id)
                            .expect("component_ids in archetypes should have ComponentInfo");

                        let reflect_component = type_registry
                            .get(component_info.type_id().unwrap())
                            .ok_or_else(|| SceneSpawnError::UnregisteredType {
                                type_name: component_info.name().to_string(),
                            })
                            .and_then(|registration| {
                                registration.data::<ReflectComponent>().ok_or_else(|| {
                                    SceneSpawnError::UnregisteredComponent {
                                        type_name: component_info.name().to_string(),
                                    }
                                })
                            })?;
                        reflect_component.copy_component(
                            &scene.world,
                            world,
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
        })
    }

    pub fn update_spawned_scenes(
        &mut self,
        world: &mut World,
        scene_handles: &[Handle<DynamicScene>],
    ) -> Result<(), SceneSpawnError> {
        for scene_handle in scene_handles {
            if let Some(spawned_instances) = self.spawned_dynamic_scenes.get(scene_handle) {
                for instance_id in spawned_instances.iter() {
                    if let Some(instance_info) = self.spawned_instances.get_mut(instance_id) {
                        Self::spawn_dynamic_internal(
                            world,
                            scene_handle,
                            &mut instance_info.entity_map,
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

    pub fn spawn_queued_scenes(&mut self, world: &mut World) -> Result<(), SceneSpawnError> {
        let scenes_to_spawn = std::mem::take(&mut self.dynamic_scenes_to_spawn);

        for scene_handle in scenes_to_spawn {
            match self.spawn_dynamic_sync(world, &scene_handle) {
                Ok(_) => {}
                Err(SceneSpawnError::NonExistentScene { .. }) => {
                    self.dynamic_scenes_to_spawn.push(scene_handle)
                }
                Err(err) => return Err(err),
            }
        }

        let scenes_to_spawn = std::mem::take(&mut self.scenes_to_spawn);

        for (scene_handle, instance_id) in scenes_to_spawn {
            match self.spawn_sync_internal(world, scene_handle, instance_id) {
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
                    if let Some(mut entity_mut) = world.get_entity_mut(entity) {
                        if !entity_mut.contains::<Parent>() {
                            entity_mut.insert(Parent(parent));
                        }
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

pub fn scene_spawner_system(world: &mut World) {
    world.resource_scope(|world, mut scene_spawner: Mut<SceneSpawner>| {
        let scene_asset_events = world
            .get_resource::<Events<AssetEvent<DynamicScene>>>()
            .unwrap();

        let mut updated_spawned_scenes = Vec::new();
        for event in scene_spawner
            .scene_asset_event_reader
            .iter(scene_asset_events)
        {
            if let AssetEvent::Modified { handle } = event {
                if scene_spawner.spawned_dynamic_scenes.contains_key(handle) {
                    updated_spawned_scenes.push(handle.clone_weak());
                }
            }
        }

        scene_spawner.despawn_queued_scenes(world).unwrap();
        scene_spawner
            .spawn_queued_scenes(world)
            .unwrap_or_else(|err| panic!("{}", err));
        scene_spawner
            .update_spawned_scenes(world, &updated_spawned_scenes)
            .unwrap();
        scene_spawner.set_scene_instance_parent_sync(world);
    });
}
