use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    any,
    fmt::{self, Formatter},
};

use bevy_utils::tracing::warn;

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
        deserializer.deserialize_any(EntityVisitor)
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

    /// We accidentally forgot to derive `ReflectSerialize` and `ReflectDeserialize` on `Name` for a while,
    /// which meant that `Name` components were being serialized as `{ "hash": _, "name" _ }` maps.
    ///
    /// We've since fixed this on [#11447](https://github.com/bevyengine/bevy/pull/11447), but in order
    /// to maintain backwards compatibility with data that might have been serialized with the wrong format,
    /// we should keep support for deserializing it for a bit. (A couple of releases should be enough.)
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        #[deprecated(
            since = "0.13.0",
            note = "Support for deserializing `Name` from a `{{ \"hash\": _, \"name\" _ }}` map is kept for backwards compatibility, but will be removed in a future version."
        )]
        fn _i_am_deprecated() {} // Can't use `#[deprecated]` on a trait impl method

        let mut result = Err(Error::missing_field("name"));

        while let Some(key) = map.next_key::<&str>()? {
            if key == "hash" {
                map.next_value::<u64>()?;
            } else if key == "name" {
                result = Ok(Name::new(map.next_value::<&str>()?.to_string()));
            } else {
                return Err(Error::unknown_field(&key, &["name", "hash"]));
            }
        }

        if result.is_ok() {
            // Doing this “manually” here because we don't have access to `bevy_log`'s `warn_once` macro
            static DID_WARN_ABOUT_DEPRECATION: AtomicBool = AtomicBool::new(false);
            if !DID_WARN_ABOUT_DEPRECATION.swap(true, Ordering::Relaxed) {
                warn!(
                    "Support for deserializing `Name` from a `{{ \"hash\": _, \"name\" _ }}` map is kept for backwards compatibility, but will be removed in a future version. Please update your serialized data to use a string instead."
                );
            }
        }

        result
    }
}
