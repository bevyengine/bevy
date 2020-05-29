use crate::serde::SceneSerializer;
use anyhow::Result;
use bevy_property::{DynamicProperties, PropertyTypeRegistry};
use bevy_type_registry::ComponentRegistry;
use legion::{entity::EntityIndex, prelude::World};
use serde::Serialize;

#[derive(Default)]
pub struct Scene {
    pub entities: Vec<Entity>,
}

pub struct Entity {
    pub entity: EntityIndex,
    pub components: Vec<DynamicProperties>,
}

impl Scene {
    pub fn from_world(world: &World, component_registry: &ComponentRegistry) -> Self {
        let mut scene = Scene::default();
        for archetype in world.storage().archetypes() {
            for chunkset in archetype.chunksets() {
                for component_storage in chunkset.occupied() {
                    let mut entities = Vec::new();
                    for (component_type_id, _component_meta) in archetype.description().components()
                    {
                        if let Some(component_registration) =
                            component_registry.get(component_type_id)
                        {
                            let component_resource_set =
                                component_storage.components(*component_type_id).unwrap();
                            for (index, entity) in component_storage.entities().iter().enumerate() {
                                if index == entities.len() {
                                    entities.push(Entity {
                                        entity: entity.index(),
                                        components: Vec::new(),
                                    })
                                }

                                let properties = component_registration
                                    .get_component_properties(&component_resource_set, index);

                                entities[index].components.push(properties.to_dynamic());
                            }
                        }
                    }

                    scene.entities.extend(entities.drain(..));
                }
            }
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
