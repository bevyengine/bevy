use crate::{
    component::{ComponentId, ComponentInfo, ComponentTicks, Tick, TickCells},
    entity::Entity,
    storage::{Column, TableRow},
};
use bevy_ptr::{OwningPtr, Ptr};
use std::{cell::UnsafeCell, hash::Hash, marker::PhantomData};

type EntityIndex = u32;

#[derive(Debug)]
pub(crate) struct SparseArray<I, V = I> {
    values: Vec<Option<V>>,
    marker: PhantomData<I>,
}

/// A space-optimized version of [`SparseArray`] that cannot be changed
/// after construction.
#[derive(Debug)]
pub(crate) struct ImmutableSparseArray<I, V = I> {
    values: Box<[Option<V>]>,
    marker: PhantomData<I>,
}

impl<I: SparseSetIndex, V> Default for SparseArray<I, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I, V> SparseArray<I, V> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            values: Vec::new(),
            marker: PhantomData,
        }
    }
}

macro_rules! impl_sparse_array {
    ($ty:ident) => {
        impl<I: SparseSetIndex, V> $ty<I, V> {
            /// Returns `true` if the collection contains a value for the specified `index`.
            #[inline]
            pub fn contains(&self, index: I) -> bool {
                let index = index.sparse_set_index();
                self.values.get(index).map(|v| v.is_some()).unwrap_or(false)
            }

            /// Returns a reference to the value at `index`.
            ///
            /// Returns `None` if `index` does not have a value or if `index` is out of bounds.
            #[inline]
            pub fn get(&self, index: I) -> Option<&V> {
                let index = index.sparse_set_index();
                self.values.get(index).map(|v| v.as_ref()).unwrap_or(None)
            }
        }
    };
}

impl_sparse_array!(SparseArray);
impl_sparse_array!(ImmutableSparseArray);

impl<I: SparseSetIndex, V> SparseArray<I, V> {
    /// Inserts `value` at `index` in the array.
    ///
    /// If `index` is out-of-bounds, this will enlarge the buffer to accommodate it.
    #[inline]
    pub fn insert(&mut self, index: I, value: V) {
        let index = index.sparse_set_index();
        if index >= self.values.len() {
            self.values.resize_with(index + 1, || None);
        }
        self.values[index] = Some(value);
    }

    /// Returns a mutable reference to the value at `index`.
    ///
    /// Returns `None` if `index` does not have a value or if `index` is out of bounds.
    #[inline]
    pub fn get_mut(&mut self, index: I) -> Option<&mut V> {
        let index = index.sparse_set_index();
        self.values
            .get_mut(index)
            .map(|v| v.as_mut())
            .unwrap_or(None)
    }

    /// Removes and returns the value stored at `index`.
    ///
    /// Returns `None` if `index` did not have a value or if `index` is out of bounds.
    #[inline]
    pub fn remove(&mut self, index: I) -> Option<V> {
        let index = index.sparse_set_index();
        self.values.get_mut(index).and_then(|value| value.take())
    }

    /// Removes all of the values stored within.
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Converts the [`SparseArray`] into an immutable variant.
    pub(crate) fn into_immutable(self) -> ImmutableSparseArray<I, V> {
        ImmutableSparseArray {
            values: self.values.into_boxed_slice(),
            marker: PhantomData,
        }
    }
}

/// A sparse data structure of [`Component`](crate::component::Component)s.
///
/// Designed for relatively fast insertions and deletions.
#[derive(Debug)]
pub struct ComponentSparseSet {
    dense: Column,
    // Internally this only relies on the Entity index to keep track of where the component data is
    // stored for entities that are alive. The generation is not required, but is stored
    // in debug builds to validate that access is correct.
    #[cfg(not(debug_assertions))]
    entities: Vec<EntityIndex>,
    #[cfg(debug_assertions)]
    entities: Vec<Entity>,
    sparse: SparseArray<EntityIndex, u32>,
}

impl ComponentSparseSet {
    /// Creates a new [`ComponentSparseSet`] with a given component type layout and
    /// initial `capacity`.
    pub(crate) fn new(component_info: &ComponentInfo, capacity: usize) -> Self {
        Self {
            dense: Column::with_capacity(component_info, capacity),
            entities: Vec::with_capacity(capacity),
            sparse: Default::default(),
        }
    }

    /// Removes all of the values stored within.
    pub(crate) fn clear(&mut self) {
        self.dense.clear();
        self.entities.clear();
        self.sparse.clear();
    }

    /// Returns the number of component values in the sparse set.
    #[inline]
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    /// Returns `true` if the sparse set contains no component values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dense.len() == 0
    }

    /// Inserts the `entity` key and component `value` pair into this sparse
    /// set.
    ///
    /// # Safety
    /// The `value` pointer must point to a valid address that matches the [`Layout`](std::alloc::Layout)
    /// inside the [`ComponentInfo`] given when constructing this sparse set.
    pub(crate) unsafe fn insert(
        &mut self,
        entity: Entity,
        value: OwningPtr<'_>,
        change_tick: Tick,
    ) {
        if let Some(&dense_index) = self.sparse.get(entity.index()) {
            #[cfg(debug_assertions)]
            assert_eq!(entity, self.entities[dense_index as usize]);
            self.dense
                .replace(TableRow::new(dense_index as usize), value, change_tick);
        } else {
            let dense_index = self.dense.len();
            self.dense.push(value, ComponentTicks::new(change_tick));
            self.sparse.insert(entity.index(), dense_index as u32);
            #[cfg(debug_assertions)]
            assert_eq!(self.entities.len(), dense_index);
            #[cfg(not(debug_assertions))]
            self.entities.push(entity.index());
            #[cfg(debug_assertions)]
            self.entities.push(entity);
        }
    }

    /// Returns `true` if the sparse set has a component value for the provided `entity`.
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        #[cfg(debug_assertions)]
        {
            if let Some(&dense_index) = self.sparse.get(entity.index()) {
                #[cfg(debug_assertions)]
                assert_eq!(entity, self.entities[dense_index as usize]);
                true
            } else {
                false
            }
        }
        #[cfg(not(debug_assertions))]
        self.sparse.contains(entity.index())
    }

    /// Returns a reference to the entity's component value.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<Ptr<'_>> {
        self.sparse.get(entity.index()).map(|dense_index| {
            let dense_index = (*dense_index) as usize;
            #[cfg(debug_assertions)]
            assert_eq!(entity, self.entities[dense_index]);
            // SAFETY: if the sparse index points to something in the dense vec, it exists
            unsafe { self.dense.get_data_unchecked(TableRow::new(dense_index)) }
        })
    }

    /// Returns references to the entity's component value and its added and changed ticks.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get_with_ticks(&self, entity: Entity) -> Option<(Ptr<'_>, TickCells<'_>)> {
        let dense_index = TableRow::new(*self.sparse.get(entity.index())? as usize);
        #[cfg(debug_assertions)]
        assert_eq!(entity, self.entities[dense_index.index()]);
        // SAFETY: if the sparse index points to something in the dense vec, it exists
        unsafe {
            Some((
                self.dense.get_data_unchecked(dense_index),
                TickCells {
                    added: self.dense.get_added_tick_unchecked(dense_index),
                    changed: self.dense.get_changed_tick_unchecked(dense_index),
                },
            ))
        }
    }

    /// Returns a reference to the "added" tick of the entity's component value.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get_added_tick(&self, entity: Entity) -> Option<&UnsafeCell<Tick>> {
        let dense_index = *self.sparse.get(entity.index())? as usize;
        #[cfg(debug_assertions)]
        assert_eq!(entity, self.entities[dense_index]);
        // SAFETY: if the sparse index points to something in the dense vec, it exists
        unsafe {
            Some(
                self.dense
                    .get_added_tick_unchecked(TableRow::new(dense_index)),
            )
        }
    }

    /// Returns a reference to the "changed" tick of the entity's component value.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get_changed_tick(&self, entity: Entity) -> Option<&UnsafeCell<Tick>> {
        let dense_index = *self.sparse.get(entity.index())? as usize;
        #[cfg(debug_assertions)]
        assert_eq!(entity, self.entities[dense_index]);
        // SAFETY: if the sparse index points to something in the dense vec, it exists
        unsafe {
            Some(
                self.dense
                    .get_changed_tick_unchecked(TableRow::new(dense_index)),
            )
        }
    }

    /// Returns a reference to the "added" and "changed" ticks of the entity's component value.
    ///
    /// Returns `None` if `entity` does not have a component in the sparse set.
    #[inline]
    pub fn get_ticks(&self, entity: Entity) -> Option<ComponentTicks> {
        let dense_index = *self.sparse.get(entity.index())? as usize;
        #[cfg(debug_assertions)]
        assert_eq!(entity, self.entities[dense_index]);
        // SAFETY: if the sparse index points to something in the dense vec, it exists
        unsafe { Some(self.dense.get_ticks_unchecked(TableRow::new(dense_index))) }
    }

    /// Removes the `entity` from this sparse set and returns a pointer to the associated value (if
    /// it exists).
    #[must_use = "The returned pointer must be used to drop the removed component."]
    pub(crate) fn remove_and_forget(&mut self, entity: Entity) -> Option<OwningPtr<'_>> {
        self.sparse.remove(entity.index()).map(|dense_index| {
            let dense_index = dense_index as usize;
            #[cfg(debug_assertions)]
            assert_eq!(entity, self.entities[dense_index]);
            self.entities.swap_remove(dense_index);
            let is_last = dense_index == self.dense.len() - 1;
            // SAFETY: dense_index was just removed from `sparse`, which ensures that it is valid
            let (value, _) = unsafe {
                self.dense
                    .swap_remove_and_forget_unchecked(TableRow::new(dense_index))
            };
            if !is_last {
                let swapped_entity = self.entities[dense_index];
                #[cfg(not(debug_assertions))]
                let index = swapped_entity;
                #[cfg(debug_assertions)]
                let index = swapped_entity.index();
                *self.sparse.get_mut(index).unwrap() = dense_index as u32;
            }
            value
        })
    }

    /// Removes (and drops) the entity's component value from the sparse set.
    ///
    /// Returns `true` if `entity` had a component value in the sparse set.
    pub(crate) fn remove(&mut self, entity: Entity) -> bool {
        if let Some(dense_index) = self.sparse.remove(entity.index()) {
            let dense_index = dense_index as usize;
            #[cfg(debug_assertions)]
            assert_eq!(entity, self.entities[dense_index]);
            self.entities.swap_remove(dense_index);
            let is_last = dense_index == self.dense.len() - 1;
            // SAFETY: if the sparse index points to something in the dense vec, it exists
            unsafe { self.dense.swap_remove_unchecked(TableRow::new(dense_index)) }
            if !is_last {
                let swapped_entity = self.entities[dense_index];
                #[cfg(not(debug_assertions))]
                let index = swapped_entity;
                #[cfg(debug_assertions)]
                let index = swapped_entity.index();
                *self.sparse.get_mut(index).unwrap() = dense_index as u32;
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        self.dense.check_change_ticks(change_tick);
    }
}

/// A data structure that blends dense and sparse storage
///
/// `I` is the type of the indices, while `V` is the type of data stored in the dense storage.
#[derive(Debug)]
pub struct SparseSet<I, V: 'static> {
    dense: Vec<V>,
    indices: Vec<I>,
    sparse: SparseArray<I, usize>,
}

/// A space-optimized version of [`SparseSet`] that cannot be changed
/// after construction.
#[derive(Debug)]
pub(crate) struct ImmutableSparseSet<I, V: 'static> {
    dense: Box<[V]>,
    indices: Box<[I]>,
    sparse: ImmutableSparseArray<I, usize>,
}

macro_rules! impl_sparse_set {
    ($ty:ident) => {
        impl<I: SparseSetIndex, V> $ty<I, V> {
            /// Returns the number of elements in the sparse set.
            #[inline]
            pub fn len(&self) -> usize {
                self.dense.len()
            }

            /// Returns `true` if the sparse set contains a value for `index`.
            #[inline]
            pub fn contains(&self, index: I) -> bool {
                self.sparse.contains(index)
            }

            /// Returns a reference to the value for `index`.
            ///
            /// Returns `None` if `index` does not have a value in the sparse set.
            pub fn get(&self, index: I) -> Option<&V> {
                self.sparse.get(index).map(|dense_index| {
                    // SAFETY: if the sparse index points to something in the dense vec, it exists
                    unsafe { self.dense.get_unchecked(*dense_index) }
                })
            }

            /// Returns a mutable reference to the value for `index`.
            ///
            /// Returns `None` if `index` does not have a value in the sparse set.
            pub fn get_mut(&mut self, index: I) -> Option<&mut V> {
                let dense = &mut self.dense;
                self.sparse.get(index).map(move |dense_index| {
                    // SAFETY: if the sparse index points to something in the dense vec, it exists
                    unsafe { dense.get_unchecked_mut(*dense_index) }
                })
            }

            /// Returns an iterator visiting all keys (indices) in arbitrary order.
            pub fn indices(&self) -> impl Iterator<Item = I> + '_ {
                self.indices.iter().cloned()
            }

            /// Returns an iterator visiting all values in arbitrary order.
            pub fn values(&self) -> impl Iterator<Item = &V> {
                self.dense.iter()
            }

            /// Returns an iterator visiting all values mutably in arbitrary order.
            pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
                self.dense.iter_mut()
            }

            /// Returns an iterator visiting all key-value pairs in arbitrary order, with references to the values.
            pub fn iter(&self) -> impl Iterator<Item = (&I, &V)> {
                self.indices.iter().zip(self.dense.iter())
            }

            /// Returns an iterator visiting all key-value pairs in arbitrary order, with mutable references to the values.
            pub fn iter_mut(&mut self) -> impl Iterator<Item = (&I, &mut V)> {
                self.indices.iter().zip(self.dense.iter_mut())
            }
        }
    };
}

impl_sparse_set!(SparseSet);
impl_sparse_set!(ImmutableSparseSet);

impl<I: SparseSetIndex, V> Default for SparseSet<I, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I, V> SparseSet<I, V> {
    /// Creates a new [`SparseSet`].
    pub const fn new() -> Self {
        Self {
            dense: Vec::new(),
            indices: Vec::new(),
            sparse: SparseArray::new(),
        }
    }
}

impl<I: SparseSetIndex, V> SparseSet<I, V> {
    /// Creates a new [`SparseSet`] with a specified initial capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            dense: Vec::with_capacity(capacity),
            indices: Vec::with_capacity(capacity),
            sparse: Default::default(),
        }
    }

    /// Returns the total number of elements the [`SparseSet`] can hold without needing to reallocate.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.dense.capacity()
    }

    /// Inserts `value` at `index`.
    ///
    /// If a value was already present at `index`, it will be overwritten.
    pub fn insert(&mut self, index: I, value: V) {
        if let Some(dense_index) = self.sparse.get(index.clone()).cloned() {
            // SAFETY: dense indices stored in self.sparse always exist
            unsafe {
                *self.dense.get_unchecked_mut(dense_index) = value;
            }
        } else {
            self.sparse.insert(index.clone(), self.dense.len());
            self.indices.push(index);
            self.dense.push(value);
        }
    }

    /// Returns a reference to the value for `index`, inserting one computed from `func`
    /// if not already present.
    pub fn get_or_insert_with(&mut self, index: I, func: impl FnOnce() -> V) -> &mut V {
        if let Some(dense_index) = self.sparse.get(index.clone()).cloned() {
            // SAFETY: dense indices stored in self.sparse always exist
            unsafe { self.dense.get_unchecked_mut(dense_index) }
        } else {
            let value = func();
            let dense_index = self.dense.len();
            self.sparse.insert(index.clone(), dense_index);
            self.indices.push(index);
            self.dense.push(value);
            // SAFETY: dense index was just populated above
            unsafe { self.dense.get_unchecked_mut(dense_index) }
        }
    }

    /// Returns `true` if the sparse set contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dense.len() == 0
    }

    /// Removes and returns the value for `index`.
    ///
    /// Returns `None` if `index` does not have a value in the sparse set.
    pub fn remove(&mut self, index: I) -> Option<V> {
        self.sparse.remove(index).map(|dense_index| {
            let is_last = dense_index == self.dense.len() - 1;
            let value = self.dense.swap_remove(dense_index);
            self.indices.swap_remove(dense_index);
            if !is_last {
                let swapped_index = self.indices[dense_index].clone();
                *self.sparse.get_mut(swapped_index).unwrap() = dense_index;
            }
            value
        })
    }

    /// Clears all of the elements from the sparse set.
    pub fn clear(&mut self) {
        self.dense.clear();
        self.indices.clear();
        self.sparse.clear();
    }

    /// Converts the sparse set into its immutable variant.
    pub(crate) fn into_immutable(self) -> ImmutableSparseSet<I, V> {
        ImmutableSparseSet {
            dense: self.dense.into_boxed_slice(),
            indices: self.indices.into_boxed_slice(),
            sparse: self.sparse.into_immutable(),
        }
    }
}

/// Represents something that can be stored in a [`SparseSet`] as an integer.
///
/// Ideally, the `usize` values should be very small (ie: incremented starting from
/// zero), as the number of bits needed to represent a `SparseSetIndex` in a `FixedBitSet`
/// is proportional to the **value** of those `usize`.
pub trait SparseSetIndex: Clone + PartialEq + Eq + Hash {
    /// Gets the sparse set index corresponding to this instance.
    fn sparse_set_index(&self) -> usize;
    /// Creates a new instance of this type with the specified index.
    fn get_sparse_set_index(value: usize) -> Self;
}

macro_rules! impl_sparse_set_index {
    ($($ty:ty),+) => {
        $(impl SparseSetIndex for $ty {
            #[inline]
            fn sparse_set_index(&self) -> usize {
                *self as usize
            }

            #[inline]
            fn get_sparse_set_index(value: usize) -> Self {
                value as $ty
            }
        })*
    };
}

impl_sparse_set_index!(u8, u16, u32, u64, usize);

/// A collection of [`ComponentSparseSet`] storages, indexed by [`ComponentId`]
///
/// Can be accessed via [`Storages`](crate::storage::Storages)
#[derive(Default)]
pub struct SparseSets {
    sets: SparseSet<ComponentId, ComponentSparseSet>,
}

impl SparseSets {
    /// Returns the number of [`ComponentSparseSet`]s this collection contains.
    #[inline]
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    /// Returns true if this collection contains no [`ComponentSparseSet`]s.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// An Iterator visiting all ([`ComponentId`], [`ComponentSparseSet`]) pairs.
    /// NOTE: Order is not guaranteed.
    pub fn iter(&self) -> impl Iterator<Item = (ComponentId, &ComponentSparseSet)> {
        self.sets.iter().map(|(id, data)| (*id, data))
    }

    /// Gets a reference to the [`ComponentSparseSet`] of a [`ComponentId`].
    #[inline]
    pub fn get(&self, component_id: ComponentId) -> Option<&ComponentSparseSet> {
        self.sets.get(component_id)
    }

    /// Gets a mutable reference of [`ComponentSparseSet`] of a [`ComponentInfo`].
    /// Create a new [`ComponentSparseSet`] if not exists.
    pub(crate) fn get_or_insert(
        &mut self,
        component_info: &ComponentInfo,
    ) -> &mut ComponentSparseSet {
        if !self.sets.contains(component_info.id()) {
            self.sets.insert(
                component_info.id(),
                ComponentSparseSet::new(component_info, 64),
            );
        }

        self.sets.get_mut(component_info.id()).unwrap()
    }

    /// Gets a mutable reference to the [`ComponentSparseSet`] of a [`ComponentId`].
    pub(crate) fn get_mut(&mut self, component_id: ComponentId) -> Option<&mut ComponentSparseSet> {
        self.sets.get_mut(component_id)
    }

    /// Clear entities stored in each [`ComponentSparseSet`]
    pub(crate) fn clear_entities(&mut self) {
        for set in self.sets.values_mut() {
            set.clear();
        }
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: Tick) {
        for set in self.sets.values_mut() {
            set.check_change_ticks(change_tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SparseSets;
    use crate::{
        self as bevy_ecs,
        component::{Component, ComponentDescriptor, ComponentId, ComponentInfo},
        entity::Entity,
        storage::SparseSet,
    };

    #[derive(Debug, Eq, PartialEq)]
    struct Foo(usize);

    #[test]
    fn sparse_set() {
        let mut set = SparseSet::<Entity, Foo>::default();
        let e0 = Entity::from_raw(0);
        let e1 = Entity::from_raw(1);
        let e2 = Entity::from_raw(2);
        let e3 = Entity::from_raw(3);
        let e4 = Entity::from_raw(4);

        set.insert(e1, Foo(1));
        set.insert(e2, Foo(2));
        set.insert(e3, Foo(3));

        assert_eq!(set.get(e0), None);
        assert_eq!(set.get(e1), Some(&Foo(1)));
        assert_eq!(set.get(e2), Some(&Foo(2)));
        assert_eq!(set.get(e3), Some(&Foo(3)));
        assert_eq!(set.get(e4), None);

        {
            let iter_results = set.values().collect::<Vec<_>>();
            assert_eq!(iter_results, vec![&Foo(1), &Foo(2), &Foo(3)]);
        }

        assert_eq!(set.remove(e2), Some(Foo(2)));
        assert_eq!(set.remove(e2), None);

        assert_eq!(set.get(e0), None);
        assert_eq!(set.get(e1), Some(&Foo(1)));
        assert_eq!(set.get(e2), None);
        assert_eq!(set.get(e3), Some(&Foo(3)));
        assert_eq!(set.get(e4), None);

        assert_eq!(set.remove(e1), Some(Foo(1)));

        assert_eq!(set.get(e0), None);
        assert_eq!(set.get(e1), None);
        assert_eq!(set.get(e2), None);
        assert_eq!(set.get(e3), Some(&Foo(3)));
        assert_eq!(set.get(e4), None);

        set.insert(e1, Foo(10));

        assert_eq!(set.get(e1), Some(&Foo(10)));

        *set.get_mut(e1).unwrap() = Foo(11);
        assert_eq!(set.get(e1), Some(&Foo(11)));
    }

    #[test]
    fn sparse_sets() {
        let mut sets = SparseSets::default();

        #[derive(Component, Default, Debug)]
        struct TestComponent1;

        #[derive(Component, Default, Debug)]
        struct TestComponent2;

        assert_eq!(sets.len(), 0);
        assert!(sets.is_empty());

        init_component::<TestComponent1>(&mut sets, 1);
        assert_eq!(sets.len(), 1);

        init_component::<TestComponent2>(&mut sets, 2);
        assert_eq!(sets.len(), 2);

        // check its shape by iter
        let mut collected_sets = sets
            .iter()
            .map(|(id, set)| (id, set.len()))
            .collect::<Vec<_>>();
        collected_sets.sort();
        assert_eq!(
            collected_sets,
            vec![(ComponentId::new(1), 0), (ComponentId::new(2), 0),]
        );

        fn init_component<T: Component>(sets: &mut SparseSets, id: usize) {
            let descriptor = ComponentDescriptor::new::<T>();
            let id = ComponentId::new(id);
            let info = ComponentInfo::new(id, descriptor);
            sets.get_or_insert(&info);
        }
    }
}
