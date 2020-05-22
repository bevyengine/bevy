use crate::{ComponentRegistration, ComponentRegistry, Scene};
use legion::prelude::{Entity, World};
use serde::{
    de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor},
    Deserialize,
};
use std::num::Wrapping;

pub struct SceneDeserializer<'a> {
    pub component_registry: &'a ComponentRegistry,
    pub scene: &'a mut Scene,
}

impl<'de> DeserializeSeed<'de> for SceneDeserializer<'de> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(EntitySeqVisiter {
            world: &mut self.scene.world,
            component_registry: &self.component_registry,
        })?;

        Ok(())
    }
}

struct EntitySeqVisiter<'a> {
    pub component_registry: &'a ComponentRegistry,
    pub world: &'a mut World,
}

impl<'a, 'de> Visitor<'de> for EntitySeqVisiter<'a> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("list of entities")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while let Some(()) = seq.next_element_seed(EntityDeserializer {
            world: self.world,
            component_registry: self.component_registry,
        })? {}

        Ok(())
    }
}

struct EntityDeserializer<'a> {
    pub component_registry: &'a ComponentRegistry,
    pub world: &'a mut World,
}


pub const ENTITY_FIELD_ID: &str = "id";
pub const ENTITY_FIELD_COMPONENTS: &str = "components";

impl<'a, 'de> DeserializeSeed<'de> for EntityDeserializer<'a> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "Entity",
            &[ENTITY_FIELD_ID, ENTITY_FIELD_COMPONENTS],
            EntityVisiter {
                world: self.world,
                component_registry: self.component_registry,
            },
        )
    }
}

struct EntityVisiter<'a> {
    pub component_registry: &'a ComponentRegistry,
    pub world: &'a mut World,
}

impl<'a, 'de> Visitor<'de> for EntityVisiter<'a> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("entity struct")
    }

    fn visit_map<V>(self, mut map: V) -> Result<(), V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut entity = None;
        let mut components = false;
        while let Some(key) = map.next_key()? {
            match key {
                EntityField::Id => {
                    if entity.is_some() {
                        return Err(Error::duplicate_field(ENTITY_FIELD_ID));
                    }
                    let id = map.next_value()?;
                    self.world
                        .entity_allocator
                        .push_next_ids((&[Entity::new(id, Wrapping(1))]).iter().map(|e| (*e)));
                    entity = Some(self.world.insert((), vec![()])[0]);
                }
                EntityField::Components => {
                    if components {
                        return Err(Error::duplicate_field(ENTITY_FIELD_COMPONENTS));
                    }

                    let entity = entity.ok_or_else(|| Error::missing_field(ENTITY_FIELD_ID))?;
                    // this is just a placeholder value to protect against duplicates
                    components = true;
                    map.next_value_seed(ComponentSeqDeserializer {
                        entity,
                        world: self.world,
                        component_registry: self.component_registry,
                    })?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntityField {
    Id,
    Components,
}

struct ComponentSeqDeserializer<'a> {
    pub component_registry: &'a ComponentRegistry,
    pub world: &'a mut World,
    pub entity: Entity,
}

impl<'a, 'de> DeserializeSeed<'de> for ComponentSeqDeserializer<'a> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ComponentSeqVisiter {
            entity: self.entity,
            world: self.world,
            component_registry: self.component_registry,
        })
    }
}

struct ComponentSeqVisiter<'a> {
    pub component_registry: &'a ComponentRegistry,
    pub world: &'a mut World,
    pub entity: Entity,
}

impl<'a, 'de> Visitor<'de> for ComponentSeqVisiter<'a> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("list of components")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while let Some(()) = seq.next_element_seed(ComponentDeserializer {
            entity: self.entity,
            world: self.world,
            component_registry: self.component_registry,
        })? {}

        Ok(())
    }
}
struct ComponentDeserializer<'a> {
    pub component_registry: &'a ComponentRegistry,
    pub world: &'a mut World,
    pub entity: Entity,
}

impl<'a, 'de> DeserializeSeed<'de> for ComponentDeserializer<'a> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_struct(
            "Component",
            &[COMPONENT_FIELD_TYPE, COMPONENT_FIELD_DATA],
            ComponentVisiter {
                entity: self.entity,
                world: self.world,
                component_registry: self.component_registry,
            },
        )
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum ComponentField {
    Type,
    Data,
}

pub const COMPONENT_FIELD_TYPE: &str = "type";
pub const COMPONENT_FIELD_DATA: &str = "data";

struct ComponentVisiter<'a> {
    pub component_registry: &'a ComponentRegistry,
    pub world: &'a mut World,
    pub entity: Entity,
}

impl<'a, 'de> Visitor<'de> for ComponentVisiter<'a> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("component")
    }

    fn visit_map<V>(self, mut map: V) -> Result<(), V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut component_type = None;
        let mut component_data = false;
        while let Some(key) = map.next_key()? {
            match key {
                ComponentField::Type => {
                    if component_type.is_some() {
                        return Err(Error::duplicate_field(COMPONENT_FIELD_TYPE));
                    }
                    component_type = Some(map.next_value::<String>()?);
                }
                ComponentField::Data => {
                    if component_data {
                        return Err(Error::duplicate_field(COMPONENT_FIELD_DATA));
                    }

                    let component_type = component_type
                        .as_ref()
                        .ok_or_else(|| Error::missing_field(COMPONENT_FIELD_TYPE))?;
                    let component_registration = self
                        .component_registry
                        .get_with_short_name(component_type)
                        .ok_or_else(|| Error::custom(format!("Component '{}' has not been registered. Consider registering it with AppBuilder::register_component::<{}>()", component_type, component_type)))?;
                    // this is just a placeholder value to protect against duplicates
                    component_data = true;
                    map.next_value_seed(ComponentDataDeserializer {
                        entity: self.entity,
                        world: self.world,
                        component_registration,
                    })?;
                }
            }
        }
        Ok(())
    }
}

struct ComponentDataDeserializer<'a> {
    pub component_registration: &'a ComponentRegistration,
    pub world: &'a mut World,
    pub entity: Entity,
}

impl<'a, 'de> DeserializeSeed<'de> for ComponentDataDeserializer<'a> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if let Err(err) = (self.component_registration.individual_comp_deserialize_fn)(
            &mut erased_serde::Deserializer::erase(deserializer),
            self.world,
            self.entity,
        ) {
            return Err(Error::custom(err.to_string()));
        }

        Ok(())
    }
}
