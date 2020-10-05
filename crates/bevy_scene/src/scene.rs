use crate::serde::SceneSerializer;
use anyhow::Result;
use bevy_ecs::World;
use bevy_property::{DynamicProperties, PropertyTypeRegistry};
use bevy_type_registry::ComponentRegistry;
use serde::Serialize;

#[derive(Debug, Default)]
pub struct Scene {
    pub entities: Vec<Entity>,
}

#[derive(Debug)]
pub struct Entity {
    pub entity: u32,
    pub components: Vec<DynamicProperties>,
}

impl Scene {
    pub fn from_world(world: &World, component_registry: &ComponentRegistry) -> Self {
        let mut scene = Scene::default();
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
                    if let Some(component_registration) = component_registry.get(&type_info.id()) {
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

    // TODO: move to AssetSaver when it is implemented
    pub fn serialize_ron(&self, registry: &PropertyTypeRegistry) -> Result<String, ron::Error> {
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
