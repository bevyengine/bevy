use crate::ComponentRegistry;
use anyhow::Result;
use bevy_property::DynamicProperties;
use legion::prelude::World;
use serde::Serialize;
use std::num::Wrapping;
use thiserror::Error;

#[derive(Default)]
pub struct Scene {
    pub entities: Vec<Entity>,
}

#[derive(Serialize)]
pub struct Entity {
    pub id: u32,
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
                                        id: entity.index(),
                                        components: Vec::new(),
                                    })
                                }

                                let properties = (component_registration.component_properties_fn)(
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
        component_registry: &ComponentRegistry,
    ) -> Result<(), SceneAddError> {
        world.entity_allocator.push_next_ids(
            self.entities
                .iter()
                .map(|e| legion::prelude::Entity::new(e.id, Wrapping(1))),
        );
        for scene_entity in self.entities.iter() {
            // TODO: use EntityEntry when legion refactor is finished
            let entity = world.insert((), vec![()])[0];
            for component in scene_entity.components.iter() {
                let component_registration = component_registry
                    .get_with_full_name(&component.type_name)
                    .ok_or_else(|| SceneAddError::UnregisteredComponent {
                        type_name: component.type_name.to_string(),
                    })?;
                (component_registration.component_add_fn)(world, entity, component);
            }
        }

        Ok(())
    }
}
