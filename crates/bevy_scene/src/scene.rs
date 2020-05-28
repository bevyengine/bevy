use anyhow::Result;
use bevy_type_registry::ComponentRegistry;
use bevy_property::{PropertyTypeRegistry, DynamicProperties};
use legion::prelude::{Resources, World};
use serde::Serialize;
use std::num::Wrapping;
use thiserror::Error;
use crate::serde::SceneSerializer;

#[derive(Default)]
pub struct Scene {
    pub entities: Vec<Entity>,
}

pub struct Entity {
    pub entity: u32,
    pub components: Vec<DynamicProperties>,
}

#[derive(Error, Debug)]
pub enum SceneAddError {
    #[error("Scene contains an unregistered component.")]
    UnregisteredComponent { type_name: String },
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

                                let properties = component_registration.get_component_properties(
                                    &component_resource_set,
                                    index,
                                );

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

    pub fn add_to_world(
        &self,
        world: &mut World,
        resources: &Resources,
        component_registry: &ComponentRegistry,
    ) -> Result<(), SceneAddError> {
        world.entity_allocator.push_next_ids(
            self.entities
                .iter()
                .map(|e| legion::prelude::Entity::new(e.entity, Wrapping(1))),
        );
        for scene_entity in self.entities.iter() {
            // TODO: use EntityEntry when legion refactor is finished
            let entity = world.insert((), vec![()])[0];
            for component in scene_entity.components.iter() {
                let component_registration = component_registry
                    .get_with_name(&component.type_name)
                    .ok_or_else(|| SceneAddError::UnregisteredComponent {
                        type_name: component.type_name.to_string(),
                    })?;
                component_registration.add_component_to_entity(world, resources, entity, component);
            }
        }

        Ok(())
    }

    // TODO: move to AssetSaver when it is implemented
    pub fn serialize_ron(&self, registry: &PropertyTypeRegistry) -> Result<String, ron::Error> {
        let pretty_config = ron::ser::PrettyConfig::default()
            .with_decimal_floats(true)
            .with_indentor("  ".to_string())
            .with_new_line("\n".to_string());
        let mut buf = Vec::new();
        let mut serializer = ron::ser::Serializer::new(&mut buf, Some(pretty_config), false)?;
        let scene_serializer = SceneSerializer::new(self, registry);
        scene_serializer.serialize(&mut serializer)?;
        Ok(String::from_utf8(buf).unwrap())
    }
}
