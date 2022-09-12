use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::QueryEntityError,
    reflect::ReflectComponent,
    system::{Query, SystemParam},
};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_hierarchy::Children;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_reflect::{std_traits::ReflectDefault, FromReflect};
use bevy_utils::AHasher;
use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
    ops::Deref,
};

/// Component used to identify an entity. Stores a hash for faster comparisons
/// The hash is eagerly re-computed upon each update to the name.
///
/// [`Name`] should not be treated as a globally unique identifier for entities,
/// as multiple entities can have the same name.  [`bevy_ecs::entity::Entity`] should be
/// used instead as the default unique identifier.
#[derive(Reflect, FromReflect, Component, Debug, Clone)]
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

/// Path to an entity, with [`Name`]s. Each entity in a path must have a name.
#[derive(Reflect, FromReflect, Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct EntityPath {
    /// Parts of the path
    pub parts: Vec<Name>,
}

/// System param to enable entity lookup of an entity via EntityPath
#[derive(SystemParam)]
pub struct NameLookup<'w, 's> {
    named: Query<'w, 's, (Entity, &'static Name)>,
    children: Query<'w, 's, &'static Children>,
}

/// Errors when looking up an entity by name
#[derive(Debug)]
pub enum LookupError {
    /// An entity could not be found, this either means the entity has been
    /// despawned, or the entity doesn't have the required components
    Query(QueryEntityError),
    /// The root node does not have the corrent name
    // TODO: add expected / found name
    RootNotFound,
    /// A child was not found
    // TODO: add expected name
    ChildNotFound,
    /// The name does not uniquely identify an entity
    // TODO: add name
    NameNotUnique,
}

impl From<QueryEntityError> for LookupError {
    fn from(q: QueryEntityError) -> Self {
        Self::Query(q)
    }
}

impl<'w, 's> NameLookup<'w, 's> {
    /// Find an entity by entity path, may return an error if the root name isn't unique
    pub fn lookup_any(&self, path: &EntityPath) -> Result<Entity, LookupError> {
        let mut path = path.parts.iter();
        let root_name = path.next().unwrap();
        let mut root = None;
        for (entity, name) in self.named.iter() {
            if root_name == name {
                if root.is_some() {
                    return Err(LookupError::NameNotUnique);
                }
                root = Some(entity);
            }
        }
        let mut current_node = root.ok_or(LookupError::RootNotFound)?;
        for part in path {
            current_node = self.find_child(current_node, part)?;
        }
        Ok(current_node)
    }

    /// Find an entity by the root & entity path
    pub fn lookup(&self, root: Entity, path: &EntityPath) -> Result<Entity, LookupError> {
        let mut path = path.parts.iter();
        let (_, root_name) = self.named.get(root)?;
        if root_name != path.next().unwrap() {
            return Err(LookupError::RootNotFound);
        }
        let mut current_node = root;
        for part in path {
            current_node = self.find_child(current_node, part)?;
        }
        Ok(current_node)
    }

    /// Internal function to get the child of `current_node` that has the name `part`
    fn find_child(&self, current_node: Entity, part: &Name) -> Result<Entity, LookupError> {
        let children = self.children.get(current_node)?;
        let mut ret = Err(LookupError::ChildNotFound);
        for child in children {
            if let Ok((_, name)) = self.named.get(*child) {
                if name == part {
                    if ret.is_ok() {
                        return Err(LookupError::NameNotUnique);
                    }
                    ret = Ok(*child);
                }
            }
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_ecs::{
        prelude::Bundle,
        query::With,
        schedule::{ParallelSystemDescriptorCoercion, Stage},
        system::Commands,
        world::World,
    };
    use bevy_hierarchy::BuildChildren;

    use super::*;

    #[derive(Component)]
    struct Root;

    fn create_heirachy(mut cmds: Commands) {
        cmds.spawn()
            .insert(Name::new("root"))
            .insert(Root)
            .with_children(|cmds| {
                cmds.spawn().insert(Name::new("child a"));
                cmds.spawn().insert(Name::new("child b"));
                cmds.spawn().insert(Name::new("child c"));
            });
    }

    #[test]
    fn test_lookup() {
        fn validate(root: Query<Entity, With<Root>>, lookup: NameLookup) {
            let root = root.single();
            let a = lookup
                .lookup(
                    root,
                    &EntityPath {
                        parts: vec![Name::new("root"), Name::new("child a")],
                    },
                )
                .unwrap();
        }

        let mut app = App::empty();
        // app.add_startup_stage_after("startup", "", )
        app.add_startup_system(create_heirachy);
        app.add_startup_system(validate.after(create_heirachy));
    }
}
