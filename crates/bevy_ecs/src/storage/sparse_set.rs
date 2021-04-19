use crate::{
    component::{ComponentId, ComponentInfo, ComponentTicks},
    entity::Entity,
    storage::BlobVec,
};
use std::{cell::UnsafeCell, marker::PhantomData};

#[derive(Debug)]
pub struct SparseArray<I, V = I> {
    values: Vec<Option<V>>,
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

impl<I: SparseSetIndex, V> SparseArray<I, V> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            marker: PhantomData,
        }
    }

    #[inline]
    pub fn insert(&mut self, index: I, value: V) {
        let index = index.sparse_set_index();
        if index >= self.values.len() {
            self.values.resize_with(index + 1, || None);
        }
        self.values[index] = Some(value);
    }

    #[inline]
    pub fn contains(&self, index: I) -> bool {
        let index = index.sparse_set_index();
        self.values.get(index).map(|v| v.is_some()).unwrap_or(false)
    }

    #[inline]
    pub fn get(&self, index: I) -> Option<&V> {
        let index = index.sparse_set_index();
        self.values.get(index).map(|v| v.as_ref()).unwrap_or(None)
    }

    #[inline]
    pub fn get_mut(&mut self, index: I) -> Option<&mut V> {
        let index = index.sparse_set_index();
        self.values
            .get_mut(index)
            .map(|v| v.as_mut())
            .unwrap_or(None)
    }

    #[inline]
    pub fn remove(&mut self, index: I) -> Option<V> {
        let index = index.sparse_set_index();
        self.values.get_mut(index).and_then(|value| value.take())
    }

    #[inline]
    pub fn get_or_insert_with(&mut self, index: I, func: impl FnOnce() -> V) -> &mut V {
        let index = index.sparse_set_index();
        if index < self.values.len() {
            return self.values[index].get_or_insert_with(func);
        }
        self.values.resize_with(index + 1, || None);
        let value = &mut self.values[index];
        *value = Some(func());
        value.as_mut().unwrap()
    }
}

#[derive(Debug)]
pub struct ComponentSparseSet {
    dense: BlobVec,
    ticks: UnsafeCell<Vec<ComponentTicks>>,
    entities: Vec<Entity>,
    sparse: SparseArray<Entity, usize>,
}

impl ComponentSparseSet {
    pub fn new(component_info: &ComponentInfo, capacity: usize) -> Self {
        Self {
            dense: BlobVec::new(component_info.layout(), component_info.drop(), capacity),
            ticks: UnsafeCell::new(Vec::with_capacity(capacity)),
            entities: Vec::with_capacity(capacity),
            sparse: Default::default(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dense.len() == 0
    }

    /// Inserts the `entity` key and component `value` pair into this sparse set.
    /// The caller is responsible for ensuring the value is not dropped. This collection will drop
    /// the value when needed.
    ///
    /// # Safety
    /// The `value` pointer must point to a valid address that matches the `Layout`
    ///  inside the `ComponentInfo` given when constructing this sparse set.
    pub unsafe fn insert(&mut self, entity: Entity, value: *mut u8, change_tick: u32) {
        let dense = &mut self.dense;
        let entities = &mut self.entities;
        let ticks_list = self.ticks.get_mut();
        let dense_index = *self.sparse.get_or_insert_with(entity, move || {
            ticks_list.push(ComponentTicks::new(change_tick));
            entities.push(entity);
            dense.push_uninit()
        });
        // SAFE: dense_index exists thanks to the call above
        self.dense.set_unchecked(dense_index, value);
        ((*self.ticks.get()).get_unchecked_mut(dense_index)).set_changed(change_tick);
    }

    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.sparse.contains(entity)
    }

    /// # Safety
    /// ensure the same entity is not accessed twice at the same time
    #[inline]
    pub fn get(&self, entity: Entity) -> Option<*mut u8> {
        self.sparse.get(entity).map(|dense_index| {
            // SAFE: if the sparse index points to something in the dense vec, it exists
            unsafe { self.dense.get_unchecked(*dense_index) }
        })
    }

    /// # Safety
    /// ensure the same entity is not accessed twice at the same time
    #[inline]
    pub unsafe fn get_with_ticks(&self, entity: Entity) -> Option<(*mut u8, *mut ComponentTicks)> {
        let ticks = &mut *self.ticks.get();
        self.sparse.get(entity).map(move |dense_index| {
            let dense_index = *dense_index;
            // SAFE: if the sparse index points to something in the dense vec, it exists
            (
                self.dense.get_unchecked(dense_index),
                ticks.get_unchecked_mut(dense_index) as *mut ComponentTicks,
            )
        })
    }

    /// # Safety
    /// ensure the same entity is not accessed twice at the same time
    #[inline]
    pub unsafe fn get_ticks(&self, entity: Entity) -> Option<&mut ComponentTicks> {
        let ticks = &mut *self.ticks.get();
        self.sparse.get(entity).map(move |dense_index| {
            let dense_index = *dense_index;
            // SAFE: if the sparse index points to something in the dense vec, it exists
            ticks.get_unchecked_mut(dense_index)
        })
    }

    /// Removes the `entity` from this sparse set and returns a pointer to the associated value (if
    /// it exists). It is the caller's responsibility to drop the returned ptr (if Some is
    /// returned).
    pub fn remove_and_forget(&mut self, entity: Entity) -> Option<*mut u8> {
        self.sparse.remove(entity).map(|dense_index| {
            // SAFE: unique access to ticks
            unsafe {
                (*self.ticks.get()).swap_remove(dense_index);
            }
            self.entities.swap_remove(dense_index);
            let is_last = dense_index == self.dense.len() - 1;
            // SAFE: dense_index was just removed from `sparse`, which ensures that it is valid
            let value = unsafe { self.dense.swap_remove_and_forget_unchecked(dense_index) };
            if !is_last {
                let swapped_entity = self.entities[dense_index];
                *self.sparse.get_mut(swapped_entity).unwrap() = dense_index;
            }
            value
        })
    }

    pub fn remove(&mut self, entity: Entity) -> bool {
        if let Some(dense_index) = self.sparse.remove(entity) {
            self.ticks.get_mut().swap_remove(dense_index);
            self.entities.swap_remove(dense_index);
            let is_last = dense_index == self.dense.len() - 1;
            // SAFE: if the sparse index points to something in the dense vec, it exists
            unsafe { self.dense.swap_remove_and_drop_unchecked(dense_index) }
            if !is_last {
                let swapped_entity = self.entities[dense_index];
                *self.sparse.get_mut(swapped_entity).unwrap() = dense_index;
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        let ticks = self.ticks.get_mut().iter_mut();
        for component_ticks in ticks {
            component_ticks.check_ticks(change_tick);
        }
    }
}

#[derive(Debug)]
pub struct SparseSet<I, V: 'static> {
    dense: Vec<V>,
    indices: Vec<I>,
    sparse: SparseArray<I, usize>,
}

impl<I: SparseSetIndex, V> Default for SparseSet<I, V> {
    fn default() -> Self {
        Self::new()
    }
}
impl<I, V> SparseSet<I, V> {
    pub const fn new() -> Self {
        Self {
            dense: Vec::new(),
            indices: Vec::new(),
            sparse: SparseArray::new(),
        }
    }
}

impl<I: SparseSetIndex, V> SparseSet<I, V> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            dense: Vec::with_capacity(capacity),
            indices: Vec::with_capacity(capacity),
            sparse: Default::default(),
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.dense.capacity()
    }

    pub fn insert(&mut self, index: I, value: V) {
        if let Some(dense_index) = self.sparse.get(index.clone()).cloned() {
            // SAFE: dense indices stored in self.sparse always exist
            unsafe {
                *self.dense.get_unchecked_mut(dense_index) = value;
            }
        } else {
            self.sparse.insert(index.clone(), self.dense.len());
            self.indices.push(index);
            self.dense.push(value);
        }

        // PERF: switch to this. it's faster but it has an invalid memory access on
        // table_add_remove_many let dense = &mut self.dense;
        // let indices = &mut self.indices;
        // let dense_index = *self.sparse.get_or_insert_with(index.clone(), move || {
        //     if dense.len() == dense.capacity() {
        //         dense.reserve(64);
        //         indices.reserve(64);
        //     }
        //     let len = dense.len();
        //     // SAFE: we set the index immediately
        //     unsafe {
        //         dense.set_len(len + 1);
        //         indices.set_len(len + 1);
        //     }
        //     len
        // });
        // // SAFE: index either already existed or was just allocated
        // unsafe {
        //     *self.dense.get_unchecked_mut(dense_index) = value;
        //     *self.indices.get_unchecked_mut(dense_index) = index;
        // }
    }

    pub fn get_or_insert_with(&mut self, index: I, func: impl FnOnce() -> V) -> &mut V {
        if let Some(dense_index) = self.sparse.get(index.clone()).cloned() {
            // SAFE: dense indices stored in self.sparse always exist
            unsafe { self.dense.get_unchecked_mut(dense_index) }
        } else {
            let value = func();
            let dense_index = self.dense.len();
            self.sparse.insert(index.clone(), dense_index);
            self.indices.push(index);
            self.dense.push(value);
            // SAFE: dense index was just populated above
            unsafe { self.dense.get_unchecked_mut(dense_index) }
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.dense.len() == 0
    }

    #[inline]
    pub fn contains(&self, index: I) -> bool {
        self.sparse.contains(index)
    }

    pub fn get(&self, index: I) -> Option<&V> {
        self.sparse.get(index).map(|dense_index| {
            // SAFE: if the sparse index points to something in the dense vec, it exists
            unsafe { self.dense.get_unchecked(*dense_index) }
        })
    }

    pub fn get_mut(&mut self, index: I) -> Option<&mut V> {
        let dense = &mut self.dense;
        self.sparse.get(index).map(move |dense_index| {
            // SAFE: if the sparse index points to something in the dense vec, it exists
            unsafe { dense.get_unchecked_mut(*dense_index) }
        })
    }

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

    pub fn indices(&self) -> impl Iterator<Item = I> + '_ {
        self.indices.iter().cloned()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.dense.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.dense.iter_mut()
    }
}

pub trait SparseSetIndex: Clone {
    fn sparse_set_index(&self) -> usize;
    fn get_sparse_set_index(value: usize) -> Self;
}

impl SparseSetIndex for u8 {
    fn sparse_set_index(&self) -> usize {
        *self as usize
    }

    fn get_sparse_set_index(value: usize) -> Self {
        value as u8
    }
}

impl SparseSetIndex for u16 {
    fn sparse_set_index(&self) -> usize {
        *self as usize
    }

    fn get_sparse_set_index(value: usize) -> Self {
        value as u16
    }
}

impl SparseSetIndex for u32 {
    fn sparse_set_index(&self) -> usize {
        *self as usize
    }

    fn get_sparse_set_index(value: usize) -> Self {
        value as u32
    }
}

impl SparseSetIndex for u64 {
    fn sparse_set_index(&self) -> usize {
        *self as usize
    }

    fn get_sparse_set_index(value: usize) -> Self {
        value as u64
    }
}

impl SparseSetIndex for usize {
    fn sparse_set_index(&self) -> usize {
        *self
    }

    fn get_sparse_set_index(value: usize) -> Self {
        value
    }
}

#[derive(Default)]
pub struct SparseSets {
    sets: SparseSet<ComponentId, ComponentSparseSet>,
}

impl SparseSets {
    pub fn get_or_insert(&mut self, component_info: &ComponentInfo) -> &mut ComponentSparseSet {
        if !self.sets.contains(component_info.id()) {
            self.sets.insert(
                component_info.id(),
                ComponentSparseSet::new(component_info, 64),
            );
        }

        self.sets.get_mut(component_info.id()).unwrap()
    }

    pub fn get(&self, component_id: ComponentId) -> Option<&ComponentSparseSet> {
        self.sets.get(component_id)
    }

    pub fn get_mut(&mut self, component_id: ComponentId) -> Option<&mut ComponentSparseSet> {
        self.sets.get_mut(component_id)
    }

    pub(crate) fn check_change_ticks(&mut self, change_tick: u32) {
        for set in self.sets.values_mut() {
            set.check_change_ticks(change_tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{entity::Entity, storage::SparseSet};

    #[derive(Debug, Eq, PartialEq)]
    struct Foo(usize);

    #[test]
    fn sparse_set() {
        let mut set = SparseSet::<Entity, Foo>::default();
        let e0 = Entity::new(0);
        let e1 = Entity::new(1);
        let e2 = Entity::new(2);
        let e3 = Entity::new(3);
        let e4 = Entity::new(4);

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
            assert_eq!(iter_results, vec![&Foo(1), &Foo(2), &Foo(3)])
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
}
