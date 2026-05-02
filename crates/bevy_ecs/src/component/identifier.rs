use core::ops::{BitAndAssign, BitOrAssign, SubAssign};

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
        self.dense.values().chain(self.sparse.values())
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

/// A set of [`ComponentId`]s.
#[derive(Default, PartialEq, Eq)]
pub struct ComponentIdSet {
    dense: FixedBitSet,
    sparse: EntityHashSet,
}

impl ComponentIdSet {
    /// Create a new empty `ComponentIdSet`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            dense: FixedBitSet::new(),
            sparse: EntityHashSet::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn from_bits(bits: FixedBitSet) -> Self {
        Self {
            dense: bits,
            sparse: EntityHashSet::new(),
        }
    }

    /// Adds a [`ComponentId`] to the set.
    #[inline]
    pub fn insert(&mut self, index: ComponentId) {
        let entity = index.id();
        if entity.index_u32() < 1024 {
            self.dense.grow_and_insert(index.index());
        } else {
            self.sparse.insert(entity);
        }
    }

    /// Removes a [`ComponentId`] from the set.
    #[inline]
    pub fn remove(&mut self, index: ComponentId) {
        let entity = index.id();
        if entity.index_u32() < 1024 {
            if index.index() < self.dense.len() {
                self.dense.remove(index.index());
            }
        } else {
            self.sparse.remove(&entity);
        }
    }

    /// Removes all [`ComponentId`]s from the set.
    #[inline]
    pub fn clear(&mut self) {
        self.dense.clear();
        self.sparse.clear();
    }

    /// Returns `true` if the [`ComponentId`] is in the set.
    #[inline]
    pub fn contains(&self, index: ComponentId) -> bool {
        let entity = index.id();
        if entity.index_u32() < 1024 {
            self.dense.contains(index.index())
        } else {
            self.sparse.contains(&entity)
        }
    }

    /// Returns `true` if `self` has no elements in common with `other`. This
    /// is equivalent to checking for an empty intersection.
    #[inline]
    pub fn is_disjoint(&self, other: &ComponentIdSet) -> bool {
        self.dense.is_disjoint(&other.dense) && self.sparse.is_disjoint(&other.sparse)
    }

    /// Returns `true` if the set is a subset of another, i.e. `other` contains
    /// at least all the values in `self`.
    #[inline]
    pub fn is_subset(&self, other: &ComponentIdSet) -> bool {
        self.dense.is_subset(&other.dense) && self.sparse.is_subset(&other.sparse)
    }

    /// Returns `true` if the set is empty.
    #[inline]
    pub fn is_clear(&self) -> bool {
        self.dense.is_clear() && self.sparse.is_empty()
    }

    /// Iterates the [`ComponentId`]s in the set.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = ComponentId> {
        self.dense
            .ones()
            .map(|id| ComponentId::from_u32(id as u32))
            .chain(self.sparse.iter().map(|entity| ComponentId::new(*entity)))
    }

    /// Returns a lazy iterator over the union of two [`ComponentIdSet`]s.
    #[inline]
    pub fn union<'a>(&'a self, other: &'a ComponentIdSet) -> impl Iterator<Item = ComponentId> {
        self.dense
            .union(&other.dense)
            .map(|id| ComponentId::from_u32(id as u32))
            .chain(
                self.sparse
                    .union(&other.sparse)
                    .map(|entity| ComponentId::new(*entity)),
            )
    }

    /// Returns a lazy iterator over the intersection of two [`ComponentIdSet`]s.
    #[inline]
    pub fn intersection<'a>(
        &'a self,
        other: &'a ComponentIdSet,
    ) -> impl Iterator<Item = ComponentId> {
        self.dense
            .intersection(&other.dense)
            .map(|id| ComponentId::from_u32(id as u32))
            .chain(
                self.sparse
                    .intersection(&other.sparse)
                    .map(|entity| ComponentId::new(*entity)),
            )
    }

    /// Returns a lazy iterator over the difference of two [`ComponentIdSet`]s.
    #[inline]
    pub fn difference<'a>(
        &'a self,
        other: &'a ComponentIdSet,
    ) -> impl Iterator<Item = ComponentId> {
        self.dense
            .difference(&other.dense)
            .map(|id| ComponentId::from_u32(id as u32))
            .chain(
                self.sparse
                    .difference(&other.sparse)
                    .map(|entity| ComponentId::new(*entity)),
            )
    }

    /// In-place union of two [`ComponentIdSet`]s.
    #[inline]
    pub fn union_with(&mut self, other: &ComponentIdSet) {
        self.dense.union_with(&other.dense);
        self.sparse.bitor_assign(&other.sparse);
    }

    /// In-place intersection of two [`ComponentIdSet`]s.
    #[inline]
    pub fn intersect_with(&mut self, other: &ComponentIdSet) {
        self.dense.intersect_with(&other.dense);
        self.sparse.bitand_assign(&other.sparse);
    }

    /// In-place difference of two [`ComponentIdSet`]s.
    #[inline]
    pub fn difference_with(&mut self, other: &ComponentIdSet) {
        self.dense.difference_with(&other.dense);
        self.sparse.sub_assign(&other.sparse);
    }

    /// In-place reversed difference of two [`ComponentIdSet`]s.
    /// This sets `self` to be `other.difference(self)`.
    #[inline]
    pub fn difference_from(&mut self, other: &ComponentIdSet) {
        // Calculate `other - self` as `!self & other`
        // We have to grow here because the new bits are going to get flipped to 1.
        self.dense.grow(other.dense.len());
        self.dense.toggle_range(..);
        self.dense.intersect_with(&other.dense);
        self.sparse.clone_from(&(&other.sparse - &self.sparse));
    }
}

impl Clone for ComponentIdSet {
    #[inline]
    fn clone(&self) -> Self {
        ComponentIdSet {
            dense: self.dense.clone(),
            sparse: self.sparse.clone(),
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.dense.clone_from(&source.dense);
        self.sparse.clone_from(&source.sparse);
    }
}

impl core::fmt::Debug for ComponentIdSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // `FixedBitSet` normally has a `Debug` output like:
        // FixedBitSet { data: [ 160 ], length: 8 }
        // Instead, print the list of set values, like:
        // [ 5, 7 ]
        // Don't wrap in `ComponentId`, since that would just output:
        // [ ComponentId(5), ComponentId(7) ]
        f.debug_list()
            .entries(
                self.dense
                    .ones()
                    .chain(self.sparse.iter().map(|index| index.index_u32() as usize)),
            )
            .finish()
    }
}

impl FromIterator<ComponentId> for ComponentIdSet {
    #[inline]
    fn from_iter<T: IntoIterator<Item = ComponentId>>(iter: T) -> Self {
        let mut set = ComponentIdSet::new();
        for index in iter {
            set.insert(index);
        }
        set
    }
}

impl Extend<ComponentId> for ComponentIdSet {
    #[inline]
    fn extend<T: IntoIterator<Item = ComponentId>>(&mut self, iter: T) {
        for index in iter {
            self.insert(index);
        }
    }
}
