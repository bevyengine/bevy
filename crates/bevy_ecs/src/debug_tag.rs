//! XXX TODO: Document.

use crate::component::Component;
use alloc::borrow::Cow;

#[cfg(feature = "serialize")]
use {
    alloc::string::{String, ToString},
    serde::{
        de::{Error, Visitor},
        Deserialize, Deserializer, Serialize, Serializer,
    },
};

#[cfg(feature = "bevy_reflect")]
use {
    crate::reflect::ReflectComponent,
    bevy_reflect::{std_traits::ReflectDefault, Reflect},
};

#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// XXX TODO: Document
#[derive(Component, Clone)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, Debug, Clone)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Deserialize, Serialize)
)]
pub struct DebugTag {
    #[cfg(feature = "debug_tag")]
    tag: Cow<'static, str>,
}

impl DebugTag {
    /// XXX TODO: Document
    #[cfg_attr(
        not(feature = "debug_tag"),
        expect(
            unused_variables,
            reason = "The value will be ignored if the `debug_tag` feature is not enabled"
        )
    )]
    pub fn new(tag: impl Into<Cow<'static, str>>) -> Self {
        #[cfg(feature = "debug_tag")]
        let out = Self { tag: tag.into() };

        #[cfg(not(feature = "debug_tag"))]
        let out = Self {};

        out
    }
}

/// XXX TODO: Document
#[macro_export]
macro_rules! debug_tag {
    ($arg:expr) => {
        if cfg!(feature = "debug_tag") {
            DebugTag::new($arg)
        } else {
            DebugTag::default()
        }
    };
}

impl Default for DebugTag {
    fn default() -> Self {
        #[cfg(feature = "debug_tag")]
        let out = Self::new("");

        #[cfg(not(feature = "debug_tag"))]
        let out = Self {};

        out
    }
}

#[cfg(feature = "serialize")]
impl Serialize for DebugTag {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[cfg(feature = "debug_tag")]
        let out = serializer.serialize_str(&self.tag);

        // XXX TODO: Think this through. Any potential for issues if it's serialized
        // when disabled but then deserialized when enabled? Depends on use cases.
        #[cfg(not(feature = "debug_tag"))]
        let out = serializer.serialize_str(DEBUG_TAG_DISABLED);

        out
    }
}

#[cfg(feature = "serialize")]
impl<'de> Deserialize<'de> for DebugTag {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(DebugTagVisitor)
    }
}

#[cfg(feature = "serialize")]
struct DebugTagVisitor;

#[cfg(feature = "serialize")]
impl<'de> Visitor<'de> for DebugTagVisitor {
    type Value = DebugTag;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str(core::any::type_name::<DebugTag>())
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(DebugTag::new(v.to_string()))
    }

    fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(DebugTag::new(v))
    }
}

#[cfg(not(feature = "debug_tag"))]
const DEBUG_TAG_DISABLED: &str = "[REDACTED]";

impl core::fmt::Debug for DebugTag {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        #[cfg(feature = "debug_tag")]
        f.write_str(self.tag.as_ref())?;

        #[cfg(not(feature = "debug_tag"))]
        f.write_str(DEBUG_TAG_DISABLED)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::format;

    #[test]
    fn test_debug_format() {
        #[cfg(feature = "debug_tag")]
        let expected = "hello";

        #[cfg(not(feature = "debug_tag"))]
        let expected = DEBUG_TAG_DISABLED;

        let tag = DebugTag::new(expected);

        assert_eq!(format!("{tag:?}"), expected);
    }
}

#[cfg(all(test, feature = "serialize"))]
mod serde_tests {
    use super::*;
    use serde_test::{assert_ser_tokens, Token};

    #[test]
    fn test_serde() {
        #[cfg(feature = "debug_tag")]
        let expected = "hello";

        #[cfg(not(feature = "debug_tag"))]
        let expected = DEBUG_TAG_DISABLED;

        let tag = DebugTag::new(expected);
        let tokens = &[Token::String(expected)];

        // TODO: Also test deserialization? We can't use `serde_test::assert_de_tokens`
        // as it requires the value to be `PartialEq`.
        assert_ser_tokens(&tag, tokens);
    }
}
