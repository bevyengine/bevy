use crate::{Entity, Scene};
use anyhow::Result;
use bevy_property::{DynamicProperties, DynamicPropertiesDeserializer, PropertyTypeRegistry};
use serde::{
    de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor},
    Deserialize, Serialize,
};
use std::{cell::RefCell, rc::Rc};

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
    type Value = Vec<Entity>;
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
    type Value = Entity;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "Entity",
            &["id", "components"],
            SceneEntityVisiter {
                property_type_registry: self.property_type_registry,
                current_type_name: self.current_type_name,
            },
        )
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntityField {
    Id,
    Components,
}

pub const ENTITY_FIELD_ID: &str = "id";
pub const ENTITY_FIELD_COMPONENTS: &str = "components";

struct SceneEntityVisiter<'a> {
    pub property_type_registry: &'a PropertyTypeRegistry,
    pub current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> Visitor<'de> for SceneEntityVisiter<'a> {
    type Value = Entity;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("entities")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut id = None;
        let mut components = None;
        while let Some(key) = map.next_key()? {
            match key {
                EntityField::Id => {
                    if id.is_some() {
                        return Err(Error::duplicate_field(ENTITY_FIELD_ID));
                    }
                    id = Some(map.next_value::<u32>()?);
                }
                EntityField::Components => {
                    if components.is_some() {
                        return Err(Error::duplicate_field(ENTITY_FIELD_COMPONENTS));
                    }

                    components = Some(map.next_value_seed(ComponentVecDeserializer {
                        current_type_name: self.current_type_name.clone(),
                        property_type_registry: self.property_type_registry,
                    })?);
                }
            }
        }

        let entity = id
            .as_ref()
            .ok_or_else(|| Error::missing_field(ENTITY_FIELD_ID))?;

        let components = components
            .take()
            .ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;
        Ok(Entity {
            id: *entity,
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
            property_type_registry: self.property_type_registry,
        })? {
            dynamic_properties.push(entity);
        }

        Ok(dynamic_properties)
    }
}
