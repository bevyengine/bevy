use crate::{ComponentRegistry, PropertyTypeRegistryContext};
use anyhow::Result;
use bevy_app::FromResources;
use bevy_asset::AssetLoader;
use bevy_property::{DynamicProperties, PropertyTypeRegistry, DynamicPropertiesDeserializer};
use legion::prelude::{Entity, Resources, World};
use serde::{
    de::{DeserializeSeed, SeqAccess, Visitor, MapAccess, Error},
    Serialize,
    Deserialize
};
use std::{cell::RefCell, num::Wrapping, path::Path, rc::Rc};
use thiserror::Error;

#[derive(Default)]
pub struct Scene {
    pub entities: Vec<SceneEntity>,
}

#[derive(Serialize)]
pub struct SceneEntity {
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

impl Serialize for Scene {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.entities.serialize(serializer)
    }
}

pub struct SceneDeserializer<'a> {
    pub property_type_registry: &'a PropertyTypeRegistry,
    pub current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneDeserializer<'a> {
    type Value = Scene;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut scene = Scene::default();
        scene.entities = deserializer.deserialize_seq(SceneEntitySeqVisiter {
            property_type_registry: self.property_type_registry,
            current_type_name: self.current_type_name,
        })?;

        Ok(scene)
    }
}

struct SceneEntitySeqVisiter<'a> {
    pub property_type_registry: &'a PropertyTypeRegistry,
    pub current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> Visitor<'de> for SceneEntitySeqVisiter<'a> {
    type Value = Vec<SceneEntity>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("list of entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut entities = Vec::new();
        while let Some(entity) = seq.next_element_seed(SceneEntityDeserializer {
            property_type_registry: self.property_type_registry,
            current_type_name: self.current_type_name.clone(),
        })? {
            entities.push(entity);
        }

        Ok(entities)
    }
}

pub struct SceneEntityDeserializer<'a> {
    pub property_type_registry: &'a PropertyTypeRegistry,
    pub current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneEntityDeserializer<'a> {
    type Value = SceneEntity;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct("", &["entity", "components"], SceneEntityVisiter {
            property_type_registry: self.property_type_registry,
            current_type_name: self.current_type_name,
        })
    }
}


#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntityField {
    Entity,
    Components,
}

pub const ENTITY_FIELD_ENTITY: &str = "entity";
pub const ENTITY_FIELD_COMPONENTS: &str = "components";

struct SceneEntityVisiter<'a> {
    pub property_type_registry: &'a PropertyTypeRegistry,
    pub current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> Visitor<'de> for SceneEntityVisiter<'a> {
    type Value = SceneEntity;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("entities")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where A: MapAccess<'de> {
        let mut entity = None;
        let mut components = None;
        while let Some(key) = map.next_key()? {
            match key {
                EntityField::Entity => {
                    if entity.is_some() {
                        return Err(Error::duplicate_field(ENTITY_FIELD_ENTITY));
                    }
                    entity = Some(map.next_value::<u32>()?);
                }
                EntityField::Components => {
                    if components.is_some() {
                        return Err(Error::duplicate_field(ENTITY_FIELD_COMPONENTS));
                    }

                    components = Some(map.next_value_seed(ComponentVecDeserializer {
                        current_type_name: self.current_type_name.clone(), 
                        property_type_registry: self.property_type_registry
                    })?);
                }
            }
        }

        let entity = entity
        .as_ref()
        .ok_or_else(|| Error::missing_field(ENTITY_FIELD_ENTITY))?;

        let components = components
        .take()
        .ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;
        Ok(SceneEntity {
            entity: *entity,
            components,
        }) 
    }
}

pub struct ComponentVecDeserializer<'a> {
    pub property_type_registry: &'a PropertyTypeRegistry,
    pub current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> DeserializeSeed<'de> for ComponentVecDeserializer<'a> {
    type Value = Vec<DynamicProperties>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ComponentSeqVisiter {
            property_type_registry: self.property_type_registry,
            current_type_name: self.current_type_name,
        })
    }
}


struct ComponentSeqVisiter<'a> {
    pub property_type_registry: &'a PropertyTypeRegistry,
    pub current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> Visitor<'de> for ComponentSeqVisiter<'a> {
    type Value = Vec<DynamicProperties>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("list of components")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut dynamic_properties = Vec::new();
        while let Some(entity) = seq.next_element_seed(DynamicPropertiesDeserializer {
            current_type_name: self.current_type_name.clone(), 
            property_type_registry: self.property_type_registry
         })? {
            dynamic_properties.push(entity);
        }

        Ok(dynamic_properties)
    }
}


pub struct SceneLoader {
    property_type_registry: PropertyTypeRegistryContext,
}
impl FromResources for SceneLoader {
    fn from_resources(resources: &Resources) -> Self {
        let property_type_registry = resources.get::<PropertyTypeRegistryContext>().unwrap();
        SceneLoader {
            property_type_registry: property_type_registry.clone(),
        }
    }
}

impl AssetLoader<Scene> for SceneLoader {
    fn from_bytes(&self, _asset_path: &Path, bytes: Vec<u8>) -> Result<Scene> {
        let registry = self.property_type_registry.value.read().unwrap();
        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes).unwrap();
        let current_type_name = Rc::new(RefCell::new(None));
        let scene_deserializer = SceneDeserializer {
            property_type_registry: &registry,
            current_type_name: current_type_name.clone(),
        };
        let mut callback = |ident: &Option<&[u8]>| {
            let mut last_type_name = current_type_name.borrow_mut();
            *last_type_name = ident.map(|i| String::from_utf8(i.to_vec()).unwrap());
        };
        deserializer.set_callback(&mut callback);


        let scene = scene_deserializer.deserialize(&mut deserializer).unwrap();
        Ok(scene)
    }
    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["scn"];
        EXTENSIONS
    }
}
