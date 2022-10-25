use crate::{DynamicEntity, DynamicScene};
use anyhow::Result;
use bevy_reflect::{
    serde::{ReflectSerializer, UntypedReflectDeserializer},
    Reflect, TypeRegistry, TypeRegistryArc,
};
use serde::{
    de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor},
    ser::{SerializeSeq, SerializeStruct},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt::Formatter;

pub const SCENE_STRUCT: &str = "Scene";
pub const SCENE_ENTITIES: &str = "entities";

pub const ENTITY_STRUCT: &str = "Entity";
pub const ENTITY_FIELD_ENTITY: &str = "entity";
pub const ENTITY_FIELD_COMPONENTS: &str = "components";

pub struct SceneSerializer<'a> {
    pub scene: &'a DynamicScene,
    pub registry: &'a TypeRegistryArc,
}

impl<'a> SceneSerializer<'a> {
    pub fn new(scene: &'a DynamicScene, registry: &'a TypeRegistryArc) -> Self {
        SceneSerializer { scene, registry }
    }
}

impl<'a> Serialize for SceneSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct(SCENE_STRUCT, 1)?;
        state.serialize_field(
            SCENE_ENTITIES,
            &EntitiesSerializer {
                entities: &self.scene.entities,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct EntitiesSerializer<'a> {
    pub entities: &'a [DynamicEntity],
    pub registry: &'a TypeRegistryArc,
}

impl<'a> Serialize for EntitiesSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.entities.len()))?;
        for entity in self.entities {
            state.serialize_element(&EntitySerializer {
                entity,
                registry: self.registry,
            })?;
        }
        state.end()
    }
}

pub struct EntitySerializer<'a> {
    pub entity: &'a DynamicEntity,
    pub registry: &'a TypeRegistryArc,
}

impl<'a> Serialize for EntitySerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct(ENTITY_STRUCT, 2)?;
        state.serialize_field(ENTITY_FIELD_ENTITY, &self.entity.entity)?;
        state.serialize_field(
            ENTITY_FIELD_COMPONENTS,
            &ComponentsSerializer {
                components: &self.entity.components,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct ComponentsSerializer<'a> {
    pub components: &'a [Box<dyn Reflect>],
    pub registry: &'a TypeRegistryArc,
}

impl<'a> Serialize for ComponentsSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.components.len()))?;
        for component in self.components {
            state.serialize_element(&ReflectSerializer::new(
                &**component,
                &*self.registry.read(),
            ))?;
        }
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum SceneField {
    Entities,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntityField {
    Entity,
    Components,
}

pub struct SceneDeserializer<'a> {
    pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneDeserializer<'a> {
    type Value = DynamicScene;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            SCENE_STRUCT,
            &[SCENE_ENTITIES],
            SceneVisitor {
                type_registry: self.type_registry,
            },
        )
    }
}

struct SceneVisitor<'a> {
    pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for SceneVisitor<'a> {
    type Value = DynamicScene;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("scene struct")
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entities = None;
        while let Some(key) = map.next_key()? {
            match key {
                SceneField::Entities => {
                    if entities.is_some() {
                        return Err(Error::duplicate_field(SCENE_ENTITIES));
                    }
                    entities = Some(map.next_value_seed(SceneEntitySeqDeserializer {
                        type_registry: self.type_registry,
                    })?);
                }
            }
        }

        let entities = entities.ok_or_else(|| Error::missing_field(SCENE_ENTITIES))?;

        Ok(DynamicScene { entities })
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let entities = seq
            .next_element_seed(SceneEntitySeqDeserializer {
                type_registry: self.type_registry,
            })?
            .ok_or_else(|| Error::missing_field(SCENE_ENTITIES))?;

        Ok(DynamicScene { entities })
    }
}

pub struct SceneEntitySeqDeserializer<'a> {
    pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneEntitySeqDeserializer<'a> {
    type Value = Vec<DynamicEntity>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(SceneEntitySeqVisitor {
            type_registry: self.type_registry,
        })
    }
}

struct SceneEntitySeqVisitor<'a> {
    pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for SceneEntitySeqVisitor<'a> {
    type Value = Vec<DynamicEntity>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("list of entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut entities = Vec::new();
        while let Some(entity) = seq.next_element_seed(SceneEntityDeserializer {
            type_registry: self.type_registry,
        })? {
            entities.push(entity);
        }

        Ok(entities)
    }
}

pub struct SceneEntityDeserializer<'a> {
    pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for SceneEntityDeserializer<'a> {
    type Value = DynamicEntity;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            ENTITY_STRUCT,
            &[ENTITY_FIELD_ENTITY, ENTITY_FIELD_COMPONENTS],
            SceneEntityVisitor {
                registry: self.type_registry,
            },
        )
    }
}

struct SceneEntityVisitor<'a> {
    pub registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for SceneEntityVisitor<'a> {
    type Value = DynamicEntity;

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
                EntityField::Entity => {
                    if id.is_some() {
                        return Err(Error::duplicate_field(ENTITY_FIELD_ENTITY));
                    }
                    id = Some(map.next_value::<u32>()?);
                }
                EntityField::Components => {
                    if components.is_some() {
                        return Err(Error::duplicate_field(ENTITY_FIELD_COMPONENTS));
                    }

                    components = Some(map.next_value_seed(ComponentVecDeserializer {
                        registry: self.registry,
                    })?);
                }
            }
        }

        let entity = id
            .as_ref()
            .ok_or_else(|| Error::missing_field(ENTITY_FIELD_ENTITY))?;

        let components = components
            .take()
            .ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;
        Ok(DynamicEntity {
            entity: *entity,
            components,
        })
    }
}

pub struct ComponentVecDeserializer<'a> {
    pub registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for ComponentVecDeserializer<'a> {
    type Value = Vec<Box<dyn Reflect>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ComponentSeqVisitor {
            registry: self.registry,
        })
    }
}

struct ComponentSeqVisitor<'a> {
    pub registry: &'a TypeRegistry,
}

impl<'a, 'de> Visitor<'de> for ComponentSeqVisitor<'a> {
    type Value = Vec<Box<dyn Reflect>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("list of components")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut dynamic_properties = Vec::new();
        while let Some(entity) =
            seq.next_element_seed(UntypedReflectDeserializer::new(self.registry))?
        {
            dynamic_properties.push(entity);
        }

        Ok(dynamic_properties)
    }
}
