// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// modified by Bevy contributors

use crate::{
    alloc::{
        alloc::{alloc, dealloc, Layout},
        vec::Vec,
    },
    world::ComponentId,
    Entity,
};
use bevy_utils::AHasher;
use core::{
    any::TypeId,
    cell::UnsafeCell,
    hash::{BuildHasherDefault, Hasher},
    mem,
    ptr::{self, NonNull},
};
use std::collections::HashMap;

use crate::{borrow::AtomicBorrow, Component};

/// A collection of entities having the same component types
///
/// Accessing `Archetype`s is only required for complex dynamic scheduling. To manipulate entities,
/// go through the `World`.
#[derive(Debug)]
pub struct Archetype {
    types: Vec<TypeInfo>,
    state: ComponentIdMap<TypeState>,
    len: usize,
    entities: Vec<Entity>,
    // UnsafeCell allows unique references into `data` to be constructed while shared references
    // containing the `Archetype` exist
    data: UnsafeCell<NonNull<u8>>,
    data_size: usize,
    grow_size: usize,
}

impl Archetype {
    #[allow(missing_docs)]
    pub fn new(types: Vec<TypeInfo>) -> Self {
        Self::with_grow(types, 64)
    }

    #[allow(missing_docs)]
    pub fn with_grow(types: Vec<TypeInfo>, grow_size: usize) -> Self {
        debug_assert!(
            types.windows(2).all(|x| x[0] < x[1]),
            "type info unsorted or contains duplicates"
        );
        let mut state = HashMap::with_capacity_and_hasher(types.len(), Default::default());
        for ty in &types {
            state.insert(ty.id, TypeState::new());
        }
        Self {
            state,
            types,
            entities: Vec::new(),
            len: 0,
            data: UnsafeCell::new(NonNull::dangling()),
            data_size: 0,
            grow_size,
        }
    }

    pub(crate) fn clear(&mut self) {
        for ty in &self.types {
            for index in 0..self.len {
                unsafe {
                    let removed = self
                        .get_dynamic(ty.id, ty.layout.size(), index)
                        .unwrap()
                        .as_ptr();
                    (ty.drop)(removed);
                }
            }
        }
        self.len = 0;
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn has<T: Component>(&self) -> bool {
        self.has_dynamic(TypeId::of::<T>().into())
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn has_component(&self, ty: ComponentId) -> bool {
        self.has_dynamic(ty)
    }

    pub(crate) fn has_dynamic(&self, id: ComponentId) -> bool {
        self.state.contains_key(&id)
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn get<T: Component>(&self) -> Option<NonNull<T>> {
        let state = self.state.get(&TypeId::of::<T>().into())?;
        Some(unsafe {
            NonNull::new_unchecked(
                (*self.data.get()).as_ptr().add(state.offset).cast::<T>() as *mut T
            )
        })
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn get_with_type_state<T: Component>(&self) -> Option<(NonNull<T>, &TypeState)> {
        let state = self.state.get(&TypeId::of::<T>().into())?;
        Some(unsafe {
            (
                NonNull::new_unchecked(
                    (*self.data.get()).as_ptr().add(state.offset).cast::<T>() as *mut T
                ),
                state,
            )
        })
    }

    #[allow(missing_docs)]
    pub fn get_type_state(&self, ty: ComponentId) -> Option<&TypeState> {
        self.state.get(&ty)
    }

    #[allow(missing_docs)]
    pub fn get_type_state_mut(&mut self, ty: ComponentId) -> Option<&mut TypeState> {
        self.state.get_mut(&ty)
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn borrow<T: Component>(&self) {
        self.borrow_component(std::any::TypeId::of::<T>().into())
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn borrow_component(&self, id: ComponentId) {
        if self.state.get(&id).map_or(false, |x| !x.borrow.borrow()) {
            panic!("{:?} already borrowed uniquely", id);
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn borrow_mut<T: Component>(&self) {
        self.borrow_component_mut(std::any::TypeId::of::<T>().into())
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn borrow_component_mut(&self, id: ComponentId) {
        if self
            .state
            .get(&id)
            .map_or(false, |x| !x.borrow.borrow_mut())
        {
            panic!("{:?} already borrowed", id);
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn release<T: Component>(&self) {
        self.release_component(std::any::TypeId::of::<T>().into());
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn release_component(&self, id: ComponentId) {
        if let Some(x) = self.state.get(&id) {
            x.borrow.release();
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn release_mut<T: Component>(&self) {
        self.release_component_mut(std::any::TypeId::of::<T>().into())
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn release_component_mut(&self, id: ComponentId) {
        if let Some(x) = self.state.get(&id) {
            x.borrow.release_mut();
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[allow(missing_docs)]
    pub fn iter_entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter().take(self.len)
    }

    #[inline]
    pub(crate) fn entities(&self) -> NonNull<Entity> {
        unsafe { NonNull::new_unchecked(self.entities.as_ptr() as *mut _) }
    }

    pub(crate) fn get_entity(&self, index: usize) -> Entity {
        self.entities[index]
    }

    #[allow(missing_docs)]
    pub fn types(&self) -> &[TypeInfo] {
        &self.types
    }

    /// # Safety
    /// `index` must be in-bounds
    pub(crate) unsafe fn get_dynamic(
        &self,
        ty: ComponentId,
        size: usize,
        index: usize,
    ) -> Option<NonNull<u8>> {
        debug_assert!(index < self.len);
        Some(NonNull::new_unchecked(
            (*self.data.get())
                .as_ptr()
                .add(self.state.get(&ty)?.offset + size * index)
                .cast::<u8>(),
        ))
    }

    /// # Safety
    /// Every type must be written immediately after this call
    pub unsafe fn allocate(&mut self, id: Entity) -> usize {
        if self.len == self.entities.len() {
            self.grow(self.len.max(self.grow_size));
        }

        self.entities[self.len] = id;
        self.len += 1;
        self.len - 1
    }

    pub(crate) fn reserve(&mut self, additional: usize) {
        if additional > (self.capacity() - self.len()) {
            self.grow(additional - (self.capacity() - self.len()));
        }
    }

    fn capacity(&self) -> usize {
        self.entities.len()
    }

    #[allow(missing_docs)]
    pub fn clear_trackers(&mut self) {
        for type_state in self.state.values_mut() {
            type_state.clear_trackers();
        }
    }

    fn grow(&mut self, increment: usize) {
        unsafe {
            let old_count = self.len;
            let count = old_count + increment;
            self.entities.resize(
                self.entities.len() + increment,
                Entity {
                    id: u32::MAX,
                    generation: u32::MAX,
                },
            );

            for type_state in self.state.values_mut() {
                type_state.mutated_entities.resize_with(count, || false);
                type_state.added_entities.resize_with(count, || false);
            }

            let old_data_size = mem::replace(&mut self.data_size, 0);
            let mut old_offsets = Vec::with_capacity(self.types.len());
            for ty in &self.types {
                self.data_size = align(self.data_size, ty.layout.align());
                let ty_state = self.state.get_mut(&ty.id).unwrap();
                old_offsets.push(ty_state.offset);
                ty_state.offset = self.data_size;
                self.data_size += ty.layout.size() * count;
            }
            let new_data = if self.data_size == 0 {
                NonNull::dangling()
            } else {
                NonNull::new(alloc(
                    Layout::from_size_align(
                        self.data_size,
                        self.types.first().map_or(1, |x| x.layout.align()),
                    )
                    .unwrap(),
                ))
                .unwrap()
            };
            if old_data_size != 0 {
                for (i, ty) in self.types.iter().enumerate() {
                    let old_off = old_offsets[i];
                    let new_off = self.state.get(&ty.id).unwrap().offset;
                    ptr::copy_nonoverlapping(
                        (*self.data.get()).as_ptr().add(old_off),
                        new_data.as_ptr().add(new_off),
                        ty.layout.size() * old_count,
                    );
                }
                dealloc(
                    (*self.data.get()).as_ptr().cast(),
                    Layout::from_size_align_unchecked(
                        old_data_size,
                        self.types.first().map_or(1, |x| x.layout.align()),
                    ),
                );
            }

            self.data = UnsafeCell::new(new_data);
        }
    }

    /// Returns the ID of the entity moved into `index`, if any
    pub(crate) unsafe fn remove(&mut self, index: usize) -> Option<Entity> {
        let last = self.len - 1;
        for ty in &self.types {
            let removed = self
                .get_dynamic(ty.id, ty.layout.size(), index)
                .unwrap()
                .as_ptr();
            (ty.drop)(removed);
            if index != last {
                // TODO: copy component tracker state here
                ptr::copy_nonoverlapping(
                    self.get_dynamic(ty.id, ty.layout.size(), last)
                        .unwrap()
                        .as_ptr(),
                    removed,
                    ty.layout.size(),
                );

                let type_state = self.state.get_mut(&ty.id).unwrap();
                type_state.mutated_entities[index] = type_state.mutated_entities[last];
                type_state.added_entities[index] = type_state.added_entities[last];
            }
        }
        self.len = last;
        if index != last {
            self.entities[index] = self.entities[last];
            Some(self.entities[last])
        } else {
            None
        }
    }

    /// Returns the ID of the entity moved into `index`, if any
    pub(crate) unsafe fn move_to(
        &mut self,
        index: usize,
        mut f: impl FnMut(*mut u8, ComponentId, usize, bool, bool),
    ) -> Option<Entity> {
        let last = self.len - 1;
        for ty in &self.types {
            let moved = self
                .get_dynamic(ty.id, ty.layout.size(), index)
                .unwrap()
                .as_ptr();
            let type_state = self.state.get(&ty.id).unwrap();
            let is_added = type_state.added_entities[index];
            let is_mutated = type_state.mutated_entities[index];
            f(moved, ty.id(), ty.layout().size(), is_added, is_mutated);
            if index != last {
                ptr::copy_nonoverlapping(
                    self.get_dynamic(ty.id, ty.layout.size(), last)
                        .unwrap()
                        .as_ptr(),
                    moved,
                    ty.layout.size(),
                );
                let type_state = self.state.get_mut(&ty.id).unwrap();
                type_state.added_entities[index] = type_state.added_entities[last];
                type_state.mutated_entities[index] = type_state.mutated_entities[last];
            }
        }
        self.len -= 1;
        if index != last {
            self.entities[index] = self.entities[last];
            Some(self.entities[last])
        } else {
            None
        }
    }

    /// # Safety
    ///
    ///  - `component` must point to valid memory
    ///  - the component `ty`pe must be registered
    ///  - `index` must be in-bound
    ///  - `size` must be the size of the component
    ///  - the storage array must be big enough
    pub unsafe fn put_dynamic(
        &mut self,
        component: *mut u8,
        ty: ComponentId,
        size: usize,
        index: usize,
        added: bool,
    ) {
        let state = self.state.get_mut(&ty).unwrap();
        if added {
            state.added_entities[index] = true;
        }
        let ptr = (*self.data.get())
            .as_ptr()
            .add(state.offset + size * index)
            .cast::<u8>();
        ptr::copy_nonoverlapping(component, ptr, size);
    }
}

impl Drop for Archetype {
    fn drop(&mut self) {
        self.clear();
        if self.data_size != 0 {
            unsafe {
                dealloc(
                    (*self.data.get()).as_ptr().cast(),
                    Layout::from_size_align_unchecked(
                        self.data_size,
                        self.types.first().map_or(1, |x| x.layout.align()),
                    ),
                );
            }
        }
    }
}

/// Metadata about a type stored in an archetype
#[derive(Debug)]
pub struct TypeState {
    offset: usize,
    borrow: AtomicBorrow,
    mutated_entities: Vec<bool>,
    added_entities: Vec<bool>,
}

impl TypeState {
    fn new() -> Self {
        Self {
            offset: 0,
            borrow: AtomicBorrow::new(),
            mutated_entities: Vec::new(),
            added_entities: Vec::new(),
        }
    }

    fn clear_trackers(&mut self) {
        for mutated in self.mutated_entities.iter_mut() {
            *mutated = false;
        }

        for added in self.added_entities.iter_mut() {
            *added = false;
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn mutated(&self) -> NonNull<bool> {
        unsafe { NonNull::new_unchecked(self.mutated_entities.as_ptr() as *mut bool) }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn added(&self) -> NonNull<bool> {
        unsafe { NonNull::new_unchecked(self.added_entities.as_ptr() as *mut bool) }
    }
}

/// Metadata required to store a component
#[derive(Debug, Copy, Clone)]
pub struct TypeInfo {
    id: ComponentId,
    layout: Layout,
    drop: unsafe fn(*mut u8),
}

impl TypeInfo {
    /// Metadata for `T`
    pub fn of<T: 'static>() -> Self {
        unsafe fn drop_ptr<T>(x: *mut u8) {
            x.cast::<T>().drop_in_place()
        }

        Self {
            id: TypeId::of::<T>().into(),
            layout: Layout::new::<T>(),
            drop: drop_ptr::<T>,
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn id(&self) -> ComponentId {
        self.id
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    pub(crate) unsafe fn drop(&self, data: *mut u8) {
        (self.drop)(data)
    }
}

impl PartialOrd for TypeInfo {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TypeInfo {
    /// Order by alignment, descending. Ties broken with ComponentId.
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.layout
            .align()
            .cmp(&other.layout.align())
            .reverse()
            .then_with(|| self.id.cmp(&other.id))
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TypeInfo {}

fn align(x: usize, alignment: usize) -> usize {
    debug_assert!(alignment.is_power_of_two());
    (x + alignment - 1) & (!alignment + 1)
}

/// A hasher optimized for hashing a single [`ComponentId`].
///
/// [`ComponentID`] is either a Rust [`TypeId`], which is already thoroughly hashed so there's no
/// reason to hash it again, or it is an external ID represented by a u64, that can also be taken
/// without changes for a hash.
///
/// For [`u64`]'s and [`u128`]'s we don't do anything to the data, for the rest of the types we
/// fallback to [`AHasher`].
#[derive(Default)]
pub(crate) struct ComponentIdHasher {
    hash: u64,
}

impl Hasher for ComponentIdHasher {
    fn write_u64(&mut self, n: u64) {
        // Only a single value can be hashed, so the old hash should be zero.
        debug_assert_eq!(self.hash, 0);
        self.hash = n;
    }

    // Tolerate TypeId being either u64 or u128.
    fn write_u128(&mut self, n: u128) {
        debug_assert_eq!(self.hash, 0);
        self.hash = n as u64;
    }

    fn write(&mut self, bytes: &[u8]) {
        debug_assert_eq!(self.hash, 0);

        // This will only be called if TypeId is neither u64 nor u128, which is not anticipated. In
        // that case we'll just fall back to using a different hash implementation.
        let mut hasher = AHasher::default();
        hasher.write(bytes);
        self.hash = hasher.finish();
    }

    fn finish(&self) -> u64 {
        self.hash
    }
}

/// A HashMap with ComponentId keys
///
/// Because ComponentId is already a fully-hashed u64 (including data in the high seven bits, which
/// hashbrown needs), there is no need to hash it again. Instead, this uses the much faster no-op
/// hash.
pub(crate) type ComponentIdMap<V> = HashMap<ComponentId, V, BuildHasherDefault<ComponentIdHasher>>;
