use bevy_property::Properties;
use std::ops::Deref;

// NOTE: This is used by the animation system to find the right entity to animate

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Component containing the name used to identify a entity. Keep in mind that
/// multiple entities may have the same name.
///
/// *NOTE* Once created you can't change it's contents because this `Name`
/// component also stores the string hash to drastically improve performance comparison
#[derive(Debug, Clone, Properties)]
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
        let mut hasher = DefaultHasher::default();
        name.hash(&mut hasher);
        let hash = hasher.finish();

        Name { name, hash }
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        self.name.as_str()
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
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Ord for Name {
    #[inline(always)]
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
