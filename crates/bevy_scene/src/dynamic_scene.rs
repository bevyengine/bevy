use crate::{serde::SceneSerializer, Scene};
use anyhow::Result;
use bevy_ecs::{EntityMap, Resources, World, ComponentId};
use bevy_property::{DynamicProperties, PropertyTypeRegistry};
use bevy_type_registry::{ComponentRegistry, TypeRegistry, TypeUuid};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DynamicSceneToWorldError {
    #[error("Scene contains an unregistered component.")]
    UnregisteredComponent { type_name: String },
}

#[derive(Default, TypeUuid)]
#[uuid = "749479b1-fb8c-4ff8-a775-623aa76014f5"]
pub struct DynamicScene {
    pub entities: Vec<Entity>,
}

pub struct Entity {
    pub entity: u32,
    pub components: Vec<DynamicProperties>,
}

impl DynamicScene {
    pub fn from_scene(scene: &Scene, component_registry: &ComponentRegistry) -> Self {
        Self::from_world(&scene.world, component_registry)
    }

    pub fn from_world(world: &World, component_registry: &ComponentRegistry) -> Self {
        let mut scene = DynamicScene::default();
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
                    if let Some(component_registration) =
                        component_registry.get(match &type_info.id() {
                            ComponentId::RustTypeId(id) => id,
                            ComponentId::ExternalId(_) => {
                                todo!("Handle external type ids in Bevy scene")
                            }
                        })
                    {
                        let properties =
                            component_registration.get_component_properties(&archetype, index);

                        entities[index].components.push(properties.to_dynamic());
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
        let type_registry = resources.get::<TypeRegistry>().unwrap();
        let component_registry = type_registry.component.read();
        let mut entity_map = EntityMap::default();
        for scene_entity in self.entities.iter() {
            let new_entity = world.reserve_entity();
            entity_map.insert(bevy_ecs::Entity::new(scene_entity.entity), new_entity);
            for component in scene_entity.components.iter() {
                let component_registration = component_registry
                    .get_with_name(&component.type_name)
                    .ok_or_else(|| DynamicSceneToWorldError::UnregisteredComponent {
                        type_name: component.type_name.to_string(),
                    })?;
                if world.has_component_type(new_entity, component_registration.ty.into()) {
                    component_registration.apply_property_to_entity(world, new_entity, component);
                } else {
                    component_registration
                        .add_property_to_entity(world, resources, new_entity, component);
                }
            }
        }

        for component_registration in component_registry.iter() {
            component_registration
                .map_entities(world, &entity_map)
                .unwrap();
        }

        Ok(())
    }

    // TODO: move to AssetSaver when it is implemented
    pub fn serialize_ron(&self, registry: &PropertyTypeRegistry) -> Result<String, ron::Error> {
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
