//! Entity paths for referring to bones.

use std::{
    fmt::{self, Debug, Formatter, Write},
    hash::{Hash, Hasher},
    sync::{Arc, Mutex, OnceLock, Weak},
};

use bevy_core::Name;
use bevy_reflect::Reflect;
use bevy_utils::prelude::default;
use weak_table::WeakHashSet;

static ENTITY_PATH_STORE: OnceLock<Mutex<EntityPathStore>> = OnceLock::new();

/// Path to an entity, with [`Name`]s. Each entity in a path must have a name.
#[derive(Clone, Reflect)]
#[reflect_value]
pub struct EntityPath(Arc<EntityPathNode>);

#[derive(PartialEq, Eq, Hash)]
struct EntityPathNode {
    name: Name,
    parent: Option<EntityPath>,
}

// This could use a `RwLock`, but we actually never read from this, so a mutex
// is actually slightly more efficient!
#[derive(Default)]
struct EntityPathStore(WeakHashSet<Weak<EntityPathNode>>);

pub struct EntityPathIter<'a>(Option<&'a EntityPath>);

impl EntityPathStore {
    fn create_path(&mut self, node: EntityPathNode) -> EntityPath {
        match self.0.get(&node) {
            Some(node) => EntityPath(node),
            None => {
                let node = Arc::new(node);
                self.0.insert(node.clone());
                EntityPath(node)
            }
        }
    }
}

impl EntityPath {
    pub fn from_name(name: Name) -> EntityPath {
        ENTITY_PATH_STORE
            .get_or_init(|| default())
            .lock()
            .unwrap()
            .create_path(EntityPathNode { name, parent: None })
    }

    pub fn from_names(names: &[Name]) -> EntityPath {
        let mut store = ENTITY_PATH_STORE.get_or_init(|| default()).lock().unwrap();

        let mut names = names.iter();
        let root_name = names
            .next()
            .expect("Entity path must have at least one name in it");

        let mut path = store.create_path(EntityPathNode {
            name: root_name.clone(),
            parent: None,
        });
        for name in names {
            path = store.create_path(EntityPathNode {
                name: name.clone(),
                parent: Some(path),
            });
        }

        path
    }

    pub fn extend(&self, name: Name) -> EntityPath {
        ENTITY_PATH_STORE
            .get_or_init(|| default())
            .lock()
            .unwrap()
            .create_path(EntityPathNode {
                name,
                parent: Some(self.clone()),
            })
    }

    pub fn iter(&self) -> EntityPathIter {
        EntityPathIter(Some(self))
    }

    pub fn root(&self) -> &Name {
        &self.iter().last().unwrap().0.name
    }

    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn name(&self) -> &Name {
        &self.0.name
    }
}

impl PartialEq for EntityPath {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for EntityPath {}

impl Hash for EntityPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash by address. This is safe because entity paths are unique.
        (self.0.as_ref() as *const EntityPathNode).hash(state)
    }
}

impl Debug for EntityPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut names = vec![];
        let mut current_path = Some(self.clone());
        while let Some(path) = current_path {
            names.push(path.0.name.clone());
            current_path = path.0.parent.clone();
        }

        for (name_index, name) in names.iter().rev().enumerate() {
            if name_index > 0 {
                f.write_char('/')?;
            }
            f.write_str(name)?;
        }

        Ok(())
    }
}

impl<'a> Iterator for EntityPathIter<'a> {
    type Item = &'a EntityPath;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            None => None,
            Some(node) => {
                self.0 = node.0.parent.as_ref();
                Some(node)
            }
        }
    }
}
