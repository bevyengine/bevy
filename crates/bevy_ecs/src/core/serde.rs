// modified by Bevy contributors

use crate::Entity;
use serde::{de::Visitor, Deserialize, Serialize, Serializer};

impl Serialize for Entity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.id())
    }
}

impl<'de> Deserialize<'de> for Entity {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(deserializer.deserialize_u32(EntityVisitor)?)
    }
}

struct EntityVisitor;

impl<'de> Visitor<'de> for EntityVisitor {
    type Value = Entity;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expected Entity")
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Entity::new(v))
    }
}
