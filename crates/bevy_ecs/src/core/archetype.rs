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

use crate::{AtomicBorrow, Component, Entity};
use bitflags::bitflags;
use std::{
    alloc::{alloc, dealloc, Layout},
    any::{type_name, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    mem,
    ptr::{self, NonNull},
};

/// A collection of entities having the same component types
///
/// Accessing `Archetype`s is only required for complex dynamic scheduling. To manipulate entities,
/// go through the `World`.
#[derive(Debug)]
pub struct Archetype {
    types: Vec<TypeInfo>,
    state: TypeIdMap<TypeState>,
    len: usize,
    entities: Vec<Entity>,
    // UnsafeCell allows unique references into `data` to be constructed while shared references
    // containing the `Archetype` exist
    data: UnsafeCell<NonNull<u8>>,
    data_size: usize,
    grow_size: usize,
}

impl Archetype {
    fn assert_type_info(types: &[TypeInfo]) {
        types.windows(2).for_each(|x| match x[0].cmp(&x[1]) {
            core::cmp::Ordering::Less => (),
            #[cfg(debug_assertions)]
            core::cmp::Ordering::Equal => panic!(
                "attempted to allocate entity with duplicate {} components; \
                 each type must occur at most once!",
                x[0].type_name
            ),
            #[cfg(not(debug_assertions))]
            core::cmp::Ordering::Equal => panic!(
                "attempted to allocate entity with duplicate components; \
                 each type must occur at most once!"
            ),
            core::cmp::Ordering::Greater => panic!("Type info is unsorted."),
        });
    }

    #[allow(missing_docs)]
    pub fn new(types: Vec<TypeInfo>) -> Self {
        Self::with_grow(types, 64)
    }

    #[allow(missing_docs)]
    pub fn with_grow(types: Vec<TypeInfo>, grow_size: usize) -> Self {
        Self::assert_type_info(&types);
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
        self.has_dynamic(TypeId::of::<T>())
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn has_type(&self, ty: TypeId) -> bool {
        self.has_dynamic(ty)
    }

    pub(crate) fn has_dynamic(&self, id: TypeId) -> bool {
        self.state.contains_key(&id)
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn get<T: Component>(&self) -> Option<NonNull<T>> {
        let state = self.state.get(&TypeId::of::<T>())?;
        Some(unsafe {
            NonNull::new_unchecked(
                (*self.data.get()).as_ptr().add(state.offset).cast::<T>() as *mut T
            )
        })
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn get_with_type_state<T: Component>(&self) -> Option<(NonNull<T>, &TypeState)> {
        let state = self.state.get(&TypeId::of::<T>())?;
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
    pub fn get_type_state(&self, ty: TypeId) -> Option<&TypeState> {
        self.state.get(&ty)
    }

    #[allow(missing_docs)]
    pub fn get_type_state_mut(&mut self, ty: TypeId) -> Option<&mut TypeState> {
        self.state.get_mut(&ty)
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn borrow<T: Component>(&self) {
        if self
            .state
            .get(&TypeId::of::<T>())
            .map_or(false, |x| !x.borrow.borrow())
        {
            panic!("{} already borrowed uniquely.", type_name::<T>());
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn borrow_mut<T: Component>(&self) {
        if self
            .state
            .get(&TypeId::of::<T>())
            .map_or(false, |x| !x.borrow.borrow_mut())
        {
            panic!("{} already borrowed.", type_name::<T>());
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn release<T: Component>(&self) {
        if let Some(x) = self.state.get(&TypeId::of::<T>()) {
            x.borrow.release();
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn release_mut<T: Component>(&self) {
        if let Some(x) = self.state.get(&TypeId::of::<T>()) {
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
        ty: TypeId,
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
            let new_capacity = self.capacity() + increment;
            self.entities.resize(
                new_capacity,
                Entity {
                    id: u32::MAX,
                    generation: u32::MAX,
                },
            );

            for type_state in self.state.values_mut() {
                type_state
                    .component_flags
                    .resize_with(new_capacity, ComponentFlags::empty);
            }

            let old_data_size = mem::replace(&mut self.data_size, 0);
            let mut old_offsets = Vec::with_capacity(self.types.len());
            for ty in &self.types {
                self.data_size = align(self.data_size, ty.layout.align());
                let ty_state = self.state.get_mut(&ty.id).unwrap();
                old_offsets.push(ty_state.offset);
                ty_state.offset = self.data_size;
                self.data_size += ty.layout.size() * new_capacity;
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
                type_state.component_flags[index] = type_state.component_flags[last];
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
        mut f: impl FnMut(*mut u8, TypeId, usize, ComponentFlags),
    ) -> Option<Entity> {
        let last = self.len - 1;
        for ty in &self.types {
            let moved = self
                .get_dynamic(ty.id, ty.layout.size(), index)
                .unwrap()
                .as_ptr();
            let type_state = self.state.get(&ty.id).unwrap();
            let flags = type_state.component_flags[index];
            f(moved, ty.id(), ty.layout().size(), flags);
            if index != last {
                ptr::copy_nonoverlapping(
                    self.get_dynamic(ty.id, ty.layout.size(), last)
                        .unwrap()
                        .as_ptr(),
                    moved,
                    ty.layout.size(),
                );
                let type_state = self.state.get_mut(&ty.id).unwrap();
                type_state.component_flags[index] = type_state.component_flags[last];
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
        ty: TypeId,
        size: usize,
        index: usize,
        flags: ComponentFlags,
    ) {
        let state = self.state.get_mut(&ty).unwrap();
        state.component_flags[index] = flags;
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
    component_flags: Vec<ComponentFlags>,
}

bitflags! {
    pub struct ComponentFlags: u8 {
        const ADDED = 1;
        const MUTATED = 2;
    }
}

impl TypeState {
    fn new() -> Self {
        Self {
            offset: 0,
            borrow: AtomicBorrow::new(),
            component_flags: Vec::new(),
        }
    }

    fn clear_trackers(&mut self) {
        for flags in self.component_flags.iter_mut() {
            *flags = ComponentFlags::empty();
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn component_flags(&self) -> NonNull<ComponentFlags> {
        unsafe { NonNull::new_unchecked(self.component_flags.as_ptr() as *mut ComponentFlags) }
    }
}

/// Metadata required to store a component
#[derive(Debug, Copy, Clone)]
pub struct TypeInfo {
    id: TypeId,
    layout: Layout,
    drop: unsafe fn(*mut u8),
    type_name: &'static str,
}

impl TypeInfo {
    /// Metadata for `T`
    pub fn of<T: 'static>() -> Self {
        unsafe fn drop_ptr<T>(x: *mut u8) {
            x.cast::<T>().drop_in_place()
        }

        Self {
            id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop: drop_ptr::<T>,
            type_name: core::any::type_name::<T>(),
        }
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn id(&self) -> TypeId {
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

    #[allow(missing_docs)]
    #[inline]
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }
}

impl PartialOrd for TypeInfo {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TypeInfo {
    /// Order by alignment, descending. Ties broken with TypeId.
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

/// A hasher optimized for hashing a single TypeId.
///
/// We don't use RandomState from std or Random state from Ahash
/// because fxhash is [proved to be faster](https://github.com/bevyengine/bevy/pull/1119#issuecomment-751361215)
/// and we don't need Hash Dos attack protection here
/// since TypeIds generated during compilation and there is no reason to user attack himself.
pub(crate) type TypeIdMap<V> = HashMap<TypeId, V, fxhash::FxBuildHasher>;
