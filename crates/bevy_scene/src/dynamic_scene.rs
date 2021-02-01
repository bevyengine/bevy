use crate::{serde::SceneSerializer, Scene};
use anyhow::Result;
use bevy_ecs::{EntityMap, Resources, World};
use bevy_reflect::{Reflect, ReflectComponent, ReflectMapEntities, TypeRegistryArc, TypeUuid};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DynamicSceneToWorldError {
    #[error("scene contains an unregistered component")]
    UnregisteredComponent { type_name: String },
}

#[derive(Default, TypeUuid)]
#[uuid = "749479b1-fb8c-4ff8-a775-623aa76014f5"]
pub struct DynamicScene {
    pub entities: Vec<Entity>,
}

pub struct Entity {
    pub entity: u32,
    pub components: Vec<Box<dyn Reflect>>,
}

impl DynamicScene {
    pub fn from_scene(scene: &Scene, type_registry: &TypeRegistryArc) -> Self {
        Self::from_world(&scene.world, type_registry)
    }

    pub fn from_world(world: &World, type_registry: &TypeRegistryArc) -> Self {
        let mut scene = DynamicScene::default();
        let type_registry = type_registry.read();
        for archetype in world.archetypes() {
            let mut entities = Vec::new();
            for (index, entity) in archetype.iter_entities().enumerate() {
                if index == entities.len() {
                    entities.push(Entity {
                        entity: entity.id(),
                        components: Vec::new(),
                    })
                }
                for type_info in archetype.types() {
                    if let Some(registration) = type_registry.get(type_info.id()) {
                        if let Some(reflect_component) = registration.data::<ReflectComponent>() {
                            // SAFE: the index comes directly from a currently live component
                            unsafe {
                                let component =
                                    reflect_component.reflect_component(&archetype, index);
                                entities[index].components.push(component.clone_value());
                            }
                        }
                    }
                }
            }

            scene.entities.extend(entities.drain(..));
        }

        scene
    }

    pub fn write_to_world(
        &self,
        world: &mut World,
        resources: &Resources,
    ) -> Result<(), DynamicSceneToWorldError> {
        let type_registry = resources.get::<TypeRegistryArc>().unwrap();
        let type_registry = type_registry.read();
        let mut entity_map = EntityMap::default();
        for scene_entity in self.entities.iter() {
            let new_entity = world.reserve_entity();
            entity_map.insert(bevy_ecs::Entity::new(scene_entity.entity), new_entity);
            for component in scene_entity.components.iter() {
                let registration = type_registry
                    .get_with_name(component.type_name())
                    .ok_or_else(|| DynamicSceneToWorldError::UnregisteredComponent {
                        type_name: component.type_name().to_string(),
                    })?;
                let reflect_component =
                    registration.data::<ReflectComponent>().ok_or_else(|| {
                        DynamicSceneToWorldError::UnregisteredComponent {
                            type_name: component.type_name().to_string(),
                        }
                    })?;
                if world.has_component_type(new_entity, registration.type_id()) {
                    reflect_component.apply_component(world, new_entity, &**component);
                } else {
                    reflect_component.add_component(world, resources, new_entity, &**component);
                }
            }
        }

        for registration in type_registry.iter() {
            if let Some(map_entities_reflect) = registration.data::<ReflectMapEntities>() {
                map_entities_reflect
                    .map_entities(world, &entity_map)
                    .unwrap();
            }
        }

        Ok(())
    }

    // TODO: move to AssetSaver when it is implemented
    pub fn serialize_ron(&self, registry: &TypeRegistryArc) -> Result<String, ron::Error> {
        serialize_ron(SceneSerializer::new(self, registry))
    }

    pub fn get_scene(&self, resources: &Resources) -> Result<Scene, DynamicSceneToWorldError> {
        let mut world = World::default();
        self.write_to_world(&mut world, resources)?;
        Ok(Scene::new(world))
    }
}

pub fn serialize_ron<S>(serialize: S) -> Result<String, ron::Error>
where
    S: Serialize,
{
    let pretty_config = ron::ser::PrettyConfig::default()
        .with_decimal_floats(true)
        .with_indentor("  ".to_string())
        .with_new_line("\n".to_string());
    let mut buf = Vec::new();
    let mut ron_serializer = ron::ser::Serializer::new(&mut buf, Some(pretty_config), false)?;
    serialize.serialize(&mut ron_serializer)?;
    Ok(String::from_utf8(buf).unwrap())
}
