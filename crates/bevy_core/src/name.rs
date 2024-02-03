use bevy_ecs::query::QueryData;
use bevy_ecs::{component::Component, entity::Entity, reflect::ReflectComponent};

use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_utils::AHasher;
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    ops::Deref,
};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Component used to identify an entity. Stores a hash for faster comparisons.
///
/// The hash is eagerly re-computed upon each update to the name.
///
/// [`Name`] should not be treated as a globally unique identifier for entities,
/// as multiple entities can have the same name.  [`Entity`] should be
/// used instead as the default unique identifier.
#[derive(Reflect, Component, Clone)]
#[reflect(Component, Default, Debug)]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
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

impl std::fmt::Display for Name {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.name, f)
    }
}

impl std::fmt::Debug for Name {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.name, f)
    }
}

/// Convenient query for giving a human friendly name to an entity.
///
/// ```
/// # use bevy_core::prelude::*;
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)] pub struct Score(f32);
/// fn increment_score(mut scores: Query<(DebugName, &mut Score)>) {
///     for (name, mut score) in &mut scores {
///         score.0 += 1.0;
///         if score.0.is_nan() {
///             bevy_utils::tracing::error!("Score for {:?} is invalid", name);
///         }
///     }
/// }
/// # bevy_ecs::system::assert_is_system(increment_score);
/// ```
#[derive(QueryData)]
pub struct DebugName {
    /// A [`Name`] that the entity might have that is displayed if available.
    pub name: Option<&'static Name>,
    /// The unique identifier of the entity as a fallback.
    pub entity: Entity,
}

impl<'a> std::fmt::Debug for DebugNameItem<'a> {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.name {
            Some(name) => write!(f, "{:?} ({:?})", &name, &self.entity),
            None => std::fmt::Debug::fmt(&self.entity, f),
        }
    }
}

/* Conversions from strings */

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

/* Conversions to strings */

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
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl Deref for Name {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.name.as_ref()
    }
}
