//! Provides the [`Name`] [`Component`], used for identifying an [`Entity`].

use crate::{self as bevy_ecs, component::Component, entity::Entity, query::QueryData};

use alloc::{
    borrow::{Cow, ToOwned},
    string::String,
};

#[cfg(feature = "bevy_reflect")]
use {
    crate::reflect::ReflectComponent,
    bevy_reflect::{std_traits::ReflectDefault, Reflect},
};

#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Component used to identify an entity.
///
/// [`Name`] should not be treated as a globally unique identifier for entities,
/// as multiple entities can have the same name. [`Entity`] should be
/// used instead as the default unique identifier.
#[derive(Component, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, Debug)
)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Deserialize, Serialize)
)]
pub struct Name(Cow<'static, str>);

impl Default for Name {
    fn default() -> Self {
        Name(Cow::Borrowed(""))
    }
}

impl Name {
    /// Creates a new [`Name`] from any string-like type.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self(name.into())
    }

    /// Creates a new [`Name`] from a statically allocated string.
    ///
    /// This never allocates.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::name::Name;
    /// const MY_NAME: Name = Name::new_static("ComponentName");
    /// # assert_eq!(Name::new("ComponentName"), MY_NAME);
    /// ```
    pub const fn new_static(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    /// Acquires a mutable reference to the inner [`String`].
    ///
    /// This will allocate a new string if the name was previously
    /// created from a borrow.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::name::Name;
    /// let mut name = Name::new("my_component_name");
    /// name.to_mut().make_ascii_uppercase();
    ///
    /// # assert_eq!(name.as_str(), "MY_COMPONENT_NAME");
    /// ```
    #[inline(always)]
    pub fn to_mut(&mut self) -> &mut String {
        self.0.to_mut()
    }

    /// Gets the name of the entity as a `&str`.
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl core::ops::Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl core::fmt::Display for Name {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.0, f)
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
        &self.0
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
        val.0.into_owned()
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

impl<'a> core::fmt::Display for NameOrEntityItem<'a> {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self.name {
            Some(name) => core::fmt::Display::fmt(name, f),
            None => core::fmt::Display::fmt(&self.entity, f),
        }
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
        let d2 = query.get(&world, e2).unwrap();
        // NameOrEntity Display for entities without a Name should be {index}v{generation}
        assert_eq!(d1.to_string(), "0v1");
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
