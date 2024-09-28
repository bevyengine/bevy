#[cfg(feature = "bevy_reflect")]
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::{component::Component, entity::Entity, query::QueryData};

use alloc::borrow::Cow;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::std_traits::ReflectDefault;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::AHasher;
use core::{
    hash::{Hash, Hasher},
    ops::Deref,
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
    reflect(Component, Default, Debug)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Deserialize, Serialize)
)]
pub struct Name {
    hash: u64, // Won't be serialized (see: `bevy_core::serde` module)
    name: Cow<'static, str>,
}

impl Default for Name {
    fn default() -> Self {
        Name::new("")
    }
}

impl Name {
    /// Creates a new [`Name`] from any string-like type.
    ///
    /// The internal hash will be computed immediately.
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        let name = name.into();
        let mut name = Name { name, hash: 0 };
        name.update_hash();
        name
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
    pub fn mutate<F: FnOnce(&mut String)>(&mut self, f: F) {
        f(self.name.to_mut());
        self.update_hash();
    }

    /// Gets the name of the entity as a `&str`.
    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.name
    }

    fn update_hash(&mut self) {
        let mut hasher = AHasher::default();
        self.name.hash(&mut hasher);
        self.hash = hasher.finish();
    }
}

impl core::fmt::Display for Name {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.name, f)
    }
}

impl core::fmt::Debug for Name {
    #[inline(always)]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Debug::fmt(&self.name, f)
    }
}

/// Convenient query for giving a human friendly name to an entity.
///
/// ```
/// # use bevy_core::prelude::*;
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)] pub struct Score(f32);
/// fn increment_score(mut scores: Query<(NameOrEntity, &mut Score)>) {
///     for (name, mut score) in &mut scores {
///         score.0 += 1.0;
///         if score.0.is_nan() {
///             bevy_utils::tracing::error!("Score for {name} is invalid");
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
        &self.name
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
        val.name.into_owned()
    }
}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        if self.hash != other.hash {
            // Makes the common case of two strings not been equal very fast
            return false;
        }

        self.name.eq(&other.name)
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
        self.name.cmp(&other.name)
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.name.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

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
