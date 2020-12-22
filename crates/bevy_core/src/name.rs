use bevy_reflect::{Reflect, ReflectComponent};
use bevy_utils::AHasher;
use std::{
    hash::{Hash, Hasher},
    ops::Deref,
};

/// Component used to identify a entity. Stores a hash for faster comparisons
#[derive(Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct Name {
    hash: u64, // TODO: Shouldn't be serialized
    name: String,
}

impl Default for Name {
    fn default() -> Self {
        Name::new("".to_string())
    }
}

impl Name {
    pub fn new(name: String) -> Self {
        let mut name = Name { name, hash: 0 };
        name.update_hash();
        name
    }

    #[inline(always)]
    pub fn set(&mut self, name: String) {
        *self = Name::new(name);
    }

    #[inline(always)]
    pub fn mutate<F: FnOnce(&mut String)>(&mut self, f: F) {
        f(&mut self.name);
        self.update_hash();
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self.name.as_str()
    }

    fn update_hash(&mut self) {
        let mut hasher = AHasher::default();
        self.name.hash(&mut hasher);
        self.hash = hasher.finish();
    }
}

impl From<&str> for Name {
    #[inline(always)]
    fn from(name: &str) -> Self {
        Name::new(name.to_owned())
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
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}
