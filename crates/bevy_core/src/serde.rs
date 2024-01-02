use std::{
    any,
    fmt::{self, Formatter},
};

use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

use super::name::Name;

impl Serialize for Name {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(EntityVisitor)
    }
}

struct EntityVisitor;

impl<'de> Visitor<'de> for EntityVisitor {
    type Value = Name;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Name>())
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Name::new(v.to_string()))
    }

    fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(Name::new(v))
    }
}
