use crate::Scene;
use bevy_app::{EventReader, Events, FromResources, GetEventReader};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_type_registry::TypeRegistry;
use legion::prelude::{Entity, Resources, World};
use std::{
    collections::{HashMap, HashSet},
    num::Wrapping,
};
use thiserror::Error;
pub struct SceneSpawner {
    loaded_scene_entities: HashMap<Handle<Scene>, Vec<Entity>>,
    scene_asset_event_reader: EventReader<AssetEvent<Scene>>,
    scenes_to_spawn: Vec<Handle<Scene>>,
    scenes_to_load: Vec<Handle<Scene>>,
}

impl FromResources for SceneSpawner {
    fn from_resources(resources: &Resources) -> Self {
        SceneSpawner {
            scene_asset_event_reader: resources.get_event_reader::<AssetEvent<Scene>>(),
            loaded_scene_entities: Default::default(),
            scenes_to_spawn: Default::default(),
            scenes_to_load: Default::default(),
        }
    }
}

#[derive(Error, Debug)]
pub enum SceneSpawnError {
    #[error("Scene contains an unregistered component.")]
    UnregisteredComponent { type_name: String },
    #[error("Scene does not exist. Perhaps it is still loading?")]
    NonExistentScene { handle: Handle<Scene> },
}

impl SceneSpawner {
    pub fn spawn(&mut self, scene: Handle<Scene>) {
        self.scenes_to_spawn.push(scene);
    }

    pub fn load(&mut self, scene: Handle<Scene>) {
        self.scenes_to_load.push(scene);
    }

    pub fn load_sync(
        &mut self,
        world: &mut World,
        resources: &Resources,
        scene_handle: Handle<Scene>,
    ) -> Result<(), SceneSpawnError> {
        let type_registry = resources.get::<TypeRegistry>().unwrap();
        let component_registry = type_registry.component.read().unwrap();
        let scenes = resources.get::<Assets<Scene>>().unwrap();
        let scene = scenes
            .get(&scene_handle)
            .ok_or_else(|| SceneSpawnError::NonExistentScene {
                handle: scene_handle,
            })?;

        // TODO: this vec might not be needed
        let mut entity_ids = Vec::with_capacity(scene.entities.len());
        for scene_entity in scene.entities.iter() {
            // TODO: use EntityEntry when legion refactor is finished
            let mut entity = Entity::new(scene_entity.entity, Wrapping(1));
            if world.get_entity_location(entity).is_none() {
                world.entity_allocator.push_next_ids((&[entity]).iter().cloned());
                entity = world.insert((), vec![()])[0];
            }
            entity_ids.push(entity);
            for component in scene_entity.components.iter() {
                let component_registration = component_registry
                    .get_with_name(&component.type_name)
                    .ok_or_else(|| SceneSpawnError::UnregisteredComponent {
                        type_name: component.type_name.to_string(),
                    })?;
                component_registration.add_component_to_entity(world, resources, entity, component);
            }
        }

        self.loaded_scene_entities.insert(scene_handle, entity_ids);
        Ok(())
    }

    pub fn load_queued_scenes(&mut self, world: &mut World, resources: &Resources) {
        let scenes_to_load = self.scenes_to_load.drain(..).collect::<Vec<_>>();
        let mut non_existent_scenes = Vec::new();
        for scene_handle in scenes_to_load {
            match self.load_sync(world, resources, scene_handle) {
                Ok(_) => {}
                Err(SceneSpawnError::NonExistentScene { .. }) => {
                    non_existent_scenes.push(scene_handle)
                }
                Err(err) => panic!("{:?}", err),
            }
        }

        self.scenes_to_load = non_existent_scenes;
    }
}

pub fn scene_spawner_system(world: &mut World, resources: &mut Resources) {
    let mut scene_spawner = resources.get_mut::<SceneSpawner>().unwrap();
    let scene_asset_events = resources.get::<Events<AssetEvent<Scene>>>().unwrap();

    for event in scene_spawner
        .scene_asset_event_reader
        .iter(&scene_asset_events)
    {
        if let AssetEvent::Modified { handle } = event {
            if scene_spawner.loaded_scene_entities.contains_key(handle) {
                scene_spawner.load(*handle);
            }
        }
    }

    scene_spawner.load_queued_scenes(world, resources);
}
