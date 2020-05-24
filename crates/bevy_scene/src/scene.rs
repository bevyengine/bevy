use crate::ComponentRegistry;
use anyhow::Result;
use bevy_asset::AssetLoader;
use bevy_property::DynamicProperties;
use legion::prelude::{Entity, World};
use serde::{Deserialize, Serialize};
use std::{num::Wrapping, path::Path};
use thiserror::Error;

#[derive(Serialize, Deserialize, Default)]
pub struct Scene {
    pub entities: Vec<SceneEntity>,
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
                                    entities.push(SceneEntity {
                                        entity: entity.index(),
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
                .map(|e| Entity::new(e.entity, Wrapping(1))),
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

#[derive(Serialize, Deserialize)]
pub struct SceneEntity {
    pub entity: u32,
    pub components: Vec<DynamicProperties>,
}

#[derive(Default)]
pub struct SceneLoader;

impl AssetLoader<Scene> for SceneLoader {
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<Scene> {
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes).unwrap();
        let entities = Vec::<SceneEntity>::deserialize(&mut deserializer).unwrap();
        Ok(Scene { entities })
    }
    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["scn"];
        EXTENSIONS
    }
}
