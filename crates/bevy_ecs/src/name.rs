//! Provides the [`Name`] [`Component`], used for identifying an [`Entity`].

use crate::{component::Component, entity::Entity, query::QueryData};

use alloc::{
    borrow::{Cow, ToOwned},
    string::String,
};
use bevy_platform::hash::Hashed;
use core::{
    hash::{Hash, Hasher},
    ops::Deref,
};

#[cfg(feature = "serialize")]
use {
    alloc::string::ToString,
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

/// Component used to identify an entity. Stores a hash for faster comparisons.
///
/// The hash is eagerly re-computed upon each update to the name.
///
/// [`Name`] should not be treated as a globally unique identifier for entities,
/// as multiple entities can have the same name.  [`Entity`] should be
/// used instead as the default unique identifier.
#[derive(Component, Clone)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, Debug, Clone, Hash, PartialEq)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Deserialize, Serialize)
)]
pub struct Name(pub HashedStr);

impl Default for Name {
    fn default() -> Self {
        Name::new("")
    }
}

/// A wrapper over Hashed. This exists to make Name("value".into()) possible, which plays nicely with contexts like the `bsn!` macro.
#[derive(Reflect, Clone)]
pub struct HashedStr(Hashed<Cow<'static, str>>);

impl From<&'static str> for HashedStr {
    fn from(value: &'static str) -> Self {
        Self(Hashed::new(Cow::Borrowed(value)))
    }
}

impl From<String> for HashedStr {
    fn from(value: String) -> Self {
        Self(Hashed::new(Cow::Owned(value)))
    }
}

impl Name {
    /// Creates a new [`Name`] from any string-like type.
    ///
    /// The internal hash will be computed immediately.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self(HashedStr(Hashed::new(name.into())))
    }

    /// Sets the entity's name.
    ///
    /// The internal hash will be re-computed.
    #[inline(always)]
    pub fn set(&mut self, name: impl Into<Cow<'static, str>>) {
        *self = Name::new(name);
    }

    /// Updates the name of the entity in place.
    ///
    /// This will allocate a new string if the name was previously
    /// created from a borrow.
    #[inline(always)]
    pub fn mutate<F: FnOnce(&mut String)>(&mut self, _f: F) {
        todo!("Expose this functionality in Hashed")
    }

    /// Gets the name of the entity as a `&str`.
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.0 .0
    }
}

impl core::fmt::Display for Name {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Display::fmt(&*self.0 .0, f)
    }
}

impl core::fmt::Debug for Name {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.0 .0, f)
    }
}

/// Convenient query for giving a human friendly name to an entity.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)] pub struct Score(f32);
/// fn increment_score(mut scores: Query<(NameOrEntity, &mut Score)>) {
///     for (name, mut score) in &mut scores {
///         score.0 += 1.0;
///         if score.0.is_nan() {
///             log::error!("Score for {name} is invalid");
///         }
///     }
/// }
/// # bevy_ecs::system::assert_is_system(increment_score);
/// ```
///
/// # Implementation
///
/// The `Display` impl for `NameOrEntity` returns the `Name` where there is one
/// or {index}v{generation} for entities without one.
#[derive(QueryData)]
#[query_data(derive(Debug))]
pub struct NameOrEntity {
    /// A [`Name`] that the entity might have that is displayed if available.
    pub name: Option<&'static Name>,
    /// The unique identifier of the entity as a fallback.
    pub entity: Entity,
}

impl<'w, 's> core::fmt::Display for NameOrEntityItem<'w, 's> {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.name {
            Some(name) => core::fmt::Display::fmt(name, f),
            None => core::fmt::Display::fmt(&self.entity, f),
        }
    }
}

// Conversions from strings

impl From<&str> for Name {
    #[inline(always)]
    fn from(name: &str) -> Self {
        Name::new(name.to_owned())
    }
}

impl From<String> for Name {
    #[inline(always)]
    fn from(name: String) -> Self {
        Name::new(name)
    }
}

// Conversions to strings

impl AsRef<str> for Name {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        &self.0 .0
    }
}

impl From<&Name> for String {
    #[inline(always)]
    fn from(val: &Name) -> String {
        val.as_str().to_owned()
    }
}

impl From<Name> for String {
    #[inline(always)]
    fn from(val: Name) -> String {
        val.as_str().to_owned()
    }
}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.0 .0, state);
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        if self.0 .0.hash() != other.0 .0.hash() {
            // Makes the common case of two strings not been equal very fast
            return false;
        }

        self.0 .0.eq(&other.0 .0)
    }
}

impl Eq for Name {}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0 .0.cmp(&other.0 .0)
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[cfg(feature = "serialize")]
impl Serialize for Name {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(feature = "serialize")]
impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(NameVisitor)
    }
}

#[cfg(feature = "serialize")]
struct NameVisitor;

#[cfg(feature = "serialize")]
impl<'de> Visitor<'de> for NameVisitor {
    type Value = Name;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str(core::any::type_name::<Name>())
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Name::new(v.to_string()))
    }

    fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
        Ok(Name::new(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;
    use alloc::string::ToString;

    #[test]
    fn test_display_of_debug_name() {
        let mut world = World::new();
        let e1 = world.spawn_empty().id();
        let name = Name::new("MyName");
        let e2 = world.spawn(name.clone()).id();
        let mut query = world.query::<NameOrEntity>();
        let d1 = query.get(&world, e1).unwrap();
        // NameOrEntity Display for entities without a Name should be {index}v{generation}
        assert_eq!(d1.to_string(), "0v0");
        let d2 = query.get(&world, e2).unwrap();
        // NameOrEntity Display for entities with a Name should be the Name
        assert_eq!(d2.to_string(), "MyName");
    }
}

#[cfg(all(test, feature = "serialize"))]
mod serde_tests {
    use super::Name;

    use serde_test::{assert_tokens, Token};

    #[test]
    fn test_serde_name() {
        let name = Name::new("MyComponent");
        assert_tokens(&name, &[Token::String("MyComponent")]);
    }
}
