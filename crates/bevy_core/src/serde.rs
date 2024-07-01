use std::{
    any,
    fmt::{self, Formatter},
};

use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

use super::name::Name;
use super::FrameCount;

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

// Manually implementing serialize/deserialize allows us to use a more compact representation as simple integers
impl Serialize for FrameCount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.0)
    }
}

impl<'de> Deserialize<'de> for FrameCount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_u32(FrameVisitor)
    }
}

struct FrameVisitor;

impl<'de> Visitor<'de> for FrameVisitor {
    type Value = FrameCount;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<FrameCount>())
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(FrameCount(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_serde_name() {
        let name = Name::new("MyComponent");
        assert_tokens(&name, &[Token::String("MyComponent")]);
    }

    #[test]
    fn test_serde_frame_count() {
        let frame_count = FrameCount(100);
        assert_tokens(&frame_count, &[Token::U32(100)]);
    }
}
