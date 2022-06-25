use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_utils::AHasher;
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    ops::Deref,
};

/// Component used to identify an entity. Stores a hash for faster comparisons.
/// The hash is eagerly re-computed upon each update to the name.
///
/// [`Name`] should not be treated as a globally unique identifier for entities,
/// as multiple entities can have the same name.  [`bevy_ecs::entity::Entity`] should be
/// used instead as the default unique identifier.
///
/// # Examples
///
/// A [`Name`] can be created from any string-like type:
///
/// ```
/// # use bevy_core::Name;
/// #
/// let name1 = Name::new("Bevy");
/// let name2 = Name::new("Bevy".to_string());
///
/// assert_eq!(name1, name2);
/// ```
///
/// Using [`Name`] in an app:
///
/// ```no_run
/// # use bevy_app::App;
/// # use bevy_core::Name;
/// # use bevy_ecs::prelude::*;
/// #
/// fn main() {
///     App::new()
///         .add_startup_system(spawn_system)
///         .add_system(search_system);
/// }
///
/// /// Add a new entity with a name
/// fn spawn_system(mut commands: Commands) {
///     commands.spawn().insert(Name::new("Bevy"));
/// }
///
/// /// Search for the entitity by name
/// fn search_system(query: Query<(Entity, &Name)>) {
///     // Creating the variable before so that it's hashed only once
///     let name_being_searched = Name::new("Bevy");
///
///     for (entity, name) in query.iter() {
///         if *name == name_being_searched {
///             // Do something with the entity
///             // NOTE: Multiple entities can have the same name!
///         }
///     }
/// }
/// ```
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Name {
    hash: u64, // TODO: Shouldn't be serialized
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
    ///
    /// # Examples
    ///
    /// With a [`&str`](str):
    ///
    /// ```
    /// # use bevy_core::Name;
    /// #
    /// let name = Name::new("Bevy");
    ///
    /// assert_eq!(name.as_str(), "Bevy");
    /// ```
    ///
    /// With a [`String`]:
    ///
    /// ```
    /// # use bevy_core::Name;
    /// #
    /// let title = "Dr.";
    /// let name = Name::new(format!("{title} Who"));
    ///
    /// assert_eq!(name.as_str(), "Dr. Who");
    /// ```
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        let name = name.into();
        let mut name = Name { name, hash: 0 };
        name.update_hash();
        name
    }

    /// Sets the entity's name.
    ///
    /// The internal hash will be re-computed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_core::Name;
    /// #
    /// let mut name = Name::new("Bevy");
    /// # assert_eq!(name.as_str(), "Bevy");
    /// name.set("Bevy Engine");
    ///
    /// assert_eq!(name.as_str(), "Bevy Engine");
    /// ```
    #[inline(always)]
    pub fn set(&mut self, name: impl Into<Cow<'static, str>>) {
        *self = Name::new(name);
    }

    /// Updates the name of the entity in place.
    ///
    /// This will allocate a new string if the name was previously
    /// created from a borrow.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_core::Name;
    /// #
    /// let mut name = Name::new("Bevy");
    /// # assert_eq!(name.as_str(), "Bevy");
    /// name.mutate(|val| *val += " Engine");
    ///
    /// assert_eq!(name.as_str(), "Bevy Engine");
    /// ```
    #[inline(always)]
    pub fn mutate<F: FnOnce(&mut String)>(&mut self, f: F) {
        f(self.name.to_mut());
        self.update_hash();
    }

    /// Gets the name of the entity as a `&str`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_core::Name;
    /// #
    /// let name = Name::new("Bevy");
    ///
    /// assert_eq!(name.as_str(), "Bevy");
    /// ```
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
        self.name.partial_cmp(&other.name)
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
