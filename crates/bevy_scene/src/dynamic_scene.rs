use crate::{serde::SceneSerializer, Scene, SceneSpawnError};
use anyhow::Result;
use bevy_ecs::{
    entity::EntityMap,
    reflect::{ReflectComponent, ReflectMapEntities},
    world::World,
};
use bevy_reflect::{Reflect, TypeRegistryArc, TypeUuid};
use serde::Serialize;

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
        for archetype in world.archetypes().iter() {
            let entities_offset = scene.entities.len();
            for entity in archetype.entities() {
                scene.entities.push(Entity {
                    entity: entity.id(),
                    components: Vec::new(),
                });
            }

            for component_id in archetype.components() {
                let reflect_component = world
                    .components()
                    .get_info(component_id)
                    .and_then(|info| type_registry.get(info.type_id().unwrap()))
                    .and_then(|registration| registration.data::<ReflectComponent>());
                if let Some(reflect_component) = reflect_component {
                    for (i, entity) in archetype.entities().iter().enumerate() {
                        if let Some(component) = reflect_component.reflect_component(world, *entity)
                        {
                            scene.entities[entities_offset + i]
                                .components
                                .push(component.clone_value());
                        }
                    }
                }
            }
        }

        scene
    }

    pub fn write_to_world(
        &self,
        world: &mut World,
        entity_map: &mut EntityMap,
    ) -> Result<(), SceneSpawnError> {
        let registry = world.get_resource::<TypeRegistryArc>().unwrap().clone();
        let type_registry = registry.read();
        for scene_entity in self.entities.iter() {
            let entity = *entity_map
                .entry(bevy_ecs::entity::Entity::new(scene_entity.entity))
                .or_insert_with(|| world.spawn().id());
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
                if world
                    .entity(entity)
                    .contains_type_id(registration.type_id())
                {
                    reflect_component.apply_component(world, entity, &**component);
                } else {
                    reflect_component.add_component(world, entity, &**component);
                }
            }
        }

        for registration in type_registry.iter() {
            if let Some(map_entities_reflect) = registration.data::<ReflectMapEntities>() {
                map_entities_reflect
                    .map_entities(world, entity_map)
                    .unwrap();
            }
        }

        Ok(())
    }

    // TODO: move to AssetSaver when it is implemented
    pub fn serialize_ron(&self, registry: &TypeRegistryArc) -> Result<String, ron::Error> {
        serialize_ron(SceneSerializer::new(self, registry))
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
