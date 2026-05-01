use crate::{
    entity::{Entity, EntityHashMap, EntityHashSet, EntityIndex},
    storage::{SparseArray, SparseSetIndex},
};
use bevy_ecs_macros::MapEntities;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use fixedbitset::FixedBitSet;

/// A value which uniquely identifies the type of a [`Component`] or [`Resource`] within a
/// [`World`](crate::world::World).
///
/// Each time a new `Component` type is registered within a `World` using
/// e.g. [`World::register_component`](crate::world::World::register_component) or
/// [`World::register_component_with_descriptor`](crate::world::World::register_component_with_descriptor)
/// or a Resource with e.g. [`World::init_resource`](crate::world::World::init_resource),
/// a corresponding `ComponentId` is created to track it.
///
/// While the distinction between `ComponentId` and [`TypeId`] may seem superficial, breaking them
/// into two separate but related concepts allows components to exist outside of Rust's type system.
/// Each Rust type registered as a `Component` will have a corresponding `ComponentId`, but additional
/// `ComponentId`s may exist in a `World` to track components which cannot be
/// represented as Rust types for scripting or other advanced use-cases.
///
/// A `ComponentId` is tightly coupled to its parent `World`. Attempting to use a `ComponentId` from
/// one `World` to access the metadata of a `Component` in a different `World` is undefined behavior
/// and must not be attempted.
///
/// Given a type `T` which implements [`Component`] (including [`Resource`]), the `ComponentId` for `T` can be retrieved
/// from a `World` using [`World::component_id()`](crate::world::World::component_id) or via [`Components::component_id()`].
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, MapEntities)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq, Clone)
)]
pub struct ComponentId(pub(super) Entity);

impl ComponentId {
    /// Creates a new [`ComponentId`].
    ///
    /// The `index` is a unique value associated with each type of component in a given world.
    /// Usually, this value is taken from a counter incremented for each type of component registered with the world.
    #[inline]
    pub const fn new(index: Entity) -> ComponentId {
        ComponentId(index)
    }

    /// Creates a new [`ComponentId`].
    ///
    /// Panics if the index is `u32::MAX`.
    #[inline]
    pub const fn from_u32(index: u32) -> ComponentId {
        ComponentId(Entity::from_raw_u32(index).unwrap())
    }

    /// Returns the index of the current component.
    // TODO: Track down all uses and improve data structures for performance.
    #[inline]
    pub fn index(self) -> usize {
        self.0.index_u32() as usize
    }

    /// Returns the inner entity from the `ComponentId`
    #[inline]
    pub fn id(self) -> Entity {
        self.0
    }
}

// Identical implementation as Entity
impl SparseSetIndex for ComponentId {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.0.sparse_set_index()
    }

    #[inline]
    fn get_sparse_set_index(value: usize) -> Self {
        Self(Entity::get_sparse_set_index(value))
    }
}

#[derive(Debug)]
pub(crate) struct ComponentIdMap<V> {
    dense: SparseArray<EntityIndex, V>,
    sparse: EntityHashMap<V>,
}

impl<V> Default for ComponentIdMap<V> {
    fn default() -> ComponentIdMap<V> {
        ComponentIdMap {
            dense: SparseArray::new(),
            sparse: EntityHashMap::new(),
        }
    }
}

impl<V> ComponentIdMap<V> {
    pub(crate) fn contains_key(&self, id: &ComponentId) -> bool {
        let index = id.id().index();
        if index.index() < 1024 {
            self.dense.contains(index)
        } else {
            self.sparse.contains_key(&id.id())
        }
    }

    pub(crate) fn get(&self, id: &ComponentId) -> Option<&V> {
        let index = id.id().index();
        if index.index() < 1024 {
            self.dense.get(index)
        } else {
            self.sparse.get(&id.id())
        }
    }

    pub(crate) fn get_mut(&mut self, id: &ComponentId) -> Option<&mut V> {
        let index = id.id().index();
        if index.index() < 1024 {
            self.dense.get_mut(index)
        } else {
            self.sparse.get_mut(&id.id())
        }
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &V> + '_ {
        self.dense
            .iter()
            .map(|(_, v)| v)
            .chain(self.sparse.values())
    }

    pub(crate) fn len(&self) -> usize {
        self.dense.len() + self.sparse.len()
    }

    /// # Safety
    ///
    /// This operation is safe if a key does not exist in the map.
    pub unsafe fn insert_unique_unchecked(&mut self, id: ComponentId, value: V) {
        let index = id.id().index();
        if index.index() < 1024 {
            self.dense.insert(index, value);
        } else {
            // SAFETY: safety contract is ensured by the caller.
            unsafe { self.sparse.insert_unique_unchecked(id.id(), value) };
        }
    }
}
