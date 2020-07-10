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

use crate::alloc::alloc::{alloc, dealloc, Layout};
use crate::alloc::boxed::Box;
use crate::alloc::{vec, vec::Vec};
use core::any::{type_name, TypeId};
use core::cell::UnsafeCell;
use core::mem;
use core::ptr::{self, NonNull};

use hashbrown::HashMap;

use crate::borrow::AtomicBorrow;
use crate::query::Fetch;
use crate::{Access, Component, Query};

/// A collection of entities having the same component types
///
/// Accessing `Archetype`s is only required for complex dynamic scheduling. To manipulate entities,
/// go through the `World`.
pub struct Archetype {
    types: Vec<TypeInfo>,
    state: HashMap<TypeId, TypeState>,
    len: u32,
    entities: Box<[u32]>,
    // UnsafeCell allows unique references into `data` to be constructed while shared references
    // containing the `Archetype` exist
    data: UnsafeCell<NonNull<u8>>,
    data_size: usize,
    grow_size: u32,
}

impl Archetype {
    #[allow(missing_docs)] 
    pub fn new(types: Vec<TypeInfo>) -> Self {
        debug_assert!(
            types.windows(2).all(|x| x[0] < x[1]),
            "type info unsorted or contains duplicates"
        );
        Self {
            types,
            state: HashMap::default(),
            entities: Box::new([]),
            len: 0,
            data: UnsafeCell::new(NonNull::dangling()),
            data_size: 0,
            grow_size: 64,
        }
    }

    #[allow(missing_docs)] 
    pub fn with_grow(types: Vec<TypeInfo>, grow_size: u32) -> Self {
        debug_assert!(
            types.windows(2).all(|x| x[0] < x[1]),
            "type info unsorted or contains duplicates"
        );
        Self {
            types,
            state: HashMap::default(),
            entities: Box::new([]),
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

    pub(crate) fn has<T: Component>(&self) -> bool {
        self.has_dynamic(TypeId::of::<T>())
    }

    pub(crate) fn has_dynamic(&self, id: TypeId) -> bool {
        self.state.contains_key(&id)
    }

    #[allow(missing_docs)] 
    pub fn get<T: Component>(&self) -> Option<NonNull<T>> {
        let state = self.state.get(&TypeId::of::<T>())?;
        Some(unsafe {
            NonNull::new_unchecked(
                (*self.data.get()).as_ptr().add(state.offset).cast::<T>() as *mut T
            )
        })
    }

    #[allow(missing_docs)] 
    pub fn borrow<T: Component>(&self) {
        if self
            .state
            .get(&TypeId::of::<T>())
            .map_or(false, |x| !x.borrow.borrow())
        {
            panic!("{} already borrowed uniquely", type_name::<T>());
        }
    }

    #[allow(missing_docs)] 
    pub fn borrow_mut<T: Component>(&self) {
        if self
            .state
            .get(&TypeId::of::<T>())
            .map_or(false, |x| !x.borrow.borrow_mut())
        {
            panic!("{} already borrowed", type_name::<T>());
        }
    }

    #[allow(missing_docs)] 
    pub fn release<T: Component>(&self) {
        if let Some(x) = self.state.get(&TypeId::of::<T>()) {
            x.borrow.release();
        }
    }

    #[allow(missing_docs)] 
    pub fn release_mut<T: Component>(&self) {
        if let Some(x) = self.state.get(&TypeId::of::<T>()) {
            x.borrow.release_mut();
        }
    }

    #[allow(missing_docs)] 
    pub fn len(&self) -> u32 {
        self.len
    }

    #[allow(missing_docs)] 
    pub fn iter_entities(&self) -> impl Iterator<Item=&u32> {
        self.entities.iter().take(self.len as usize)
    }

    pub(crate) fn entities(&self) -> NonNull<u32> {
        unsafe { NonNull::new_unchecked(self.entities.as_ptr() as *mut _) }
    }

    pub(crate) fn entity_id(&self, index: u32) -> u32 {
        self.entities[index as usize]
    }

    #[allow(missing_docs)] 
    pub fn types(&self) -> &[TypeInfo] {
        &self.types
    }

    /// `index` must be in-bounds
    pub(crate) unsafe fn get_dynamic(
        &self,
        ty: TypeId,
        size: usize,
        index: u32,
    ) -> Option<NonNull<u8>> {
        debug_assert!(index < self.len);
        Some(NonNull::new_unchecked(
            (*self.data.get())
                .as_ptr()
                .add(self.state.get(&ty)?.offset + size * index as usize)
                .cast::<u8>(),
        ))
    }

    /// Every type must be written immediately after this call
    pub unsafe fn allocate(&mut self, id: u32) -> u32 {
        if self.len as usize == self.entities.len() {
            self.grow(self.len.max(self.grow_size));
        }

        self.entities[self.len as usize] = id;
        self.len += 1;
        self.len - 1
    }

    pub(crate) fn reserve(&mut self, additional: u32) {
        if additional > (self.capacity() - self.len()) {
            self.grow(additional - (self.capacity() - self.len()));
        }
    }

    fn capacity(&self) -> u32 {
        self.entities.len() as u32
    }

    fn grow(&mut self, increment: u32) {
        unsafe {
            let old_count = self.len as usize;
            let count = old_count + increment as usize;
            let mut new_entities = vec![!0; count].into_boxed_slice();
            new_entities[0..old_count].copy_from_slice(&self.entities[0..old_count]);
            self.entities = new_entities;

            let old_data_size = mem::replace(&mut self.data_size, 0);
            let mut state = HashMap::with_capacity(self.types.len());
            for ty in &self.types {
                self.data_size = align(self.data_size, ty.layout.align());
                state.insert(ty.id, TypeState::new(self.data_size));
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
                for ty in &self.types {
                    let old_off = self.state.get(&ty.id).unwrap().offset;
                    let new_off = state.get(&ty.id).unwrap().offset;
                    ptr::copy_nonoverlapping(
                        (*self.data.get()).as_ptr().add(old_off),
                        new_data.as_ptr().add(new_off),
                        ty.layout.size() * old_count,
                    );
                }
            }

            self.data = UnsafeCell::new(new_data);
            self.state = state;
        }
    }

    /// Returns the ID of the entity moved into `index`, if any
    pub(crate) unsafe fn remove(&mut self, index: u32) -> Option<u32> {
        let last = self.len - 1;
        for ty in &self.types {
            let removed = self
                .get_dynamic(ty.id, ty.layout.size(), index)
                .unwrap()
                .as_ptr();
            (ty.drop)(removed);
            if index != last {
                ptr::copy_nonoverlapping(
                    self.get_dynamic(ty.id, ty.layout.size(), last)
                        .unwrap()
                        .as_ptr(),
                    removed,
                    ty.layout.size(),
                );
            }
        }
        self.len = last;
        if index != last {
            self.entities[index as usize] = self.entities[last as usize];
            Some(self.entities[last as usize])
        } else {
            None
        }
    }

    /// Returns the ID of the entity moved into `index`, if any
    pub(crate) unsafe fn move_to(
        &mut self,
        index: u32,
        mut f: impl FnMut(*mut u8, TypeId, usize),
    ) -> Option<u32> {
        let last = self.len - 1;
        for ty in &self.types {
            let moved = self
                .get_dynamic(ty.id, ty.layout.size(), index)
                .unwrap()
                .as_ptr();
            f(moved, ty.id(), ty.layout().size());
            if index != last {
                ptr::copy_nonoverlapping(
                    self.get_dynamic(ty.id, ty.layout.size(), last)
                        .unwrap()
                        .as_ptr(),
                    moved,
                    ty.layout.size(),
                );
            }
        }
        self.len -= 1;
        if index != last {
            self.entities[index as usize] = self.entities[last as usize];
            Some(self.entities[last as usize])
        } else {
            None
        }
    }

    #[allow(missing_docs)] 
    pub unsafe fn put_dynamic(
        &mut self,
        component: *mut u8,
        ty: TypeId,
        size: usize,
        index: u32,
    ) {
        let ptr = self
            .get_dynamic(ty, size, index)
            .unwrap()
            .as_ptr()
            .cast::<u8>();
        ptr::copy_nonoverlapping(component, ptr, size);
    }

    /// How, if at all, `Q` will access entities in this archetype
    pub fn access<Q: Query>(&self) -> Option<Access> {
        Q::Fetch::access(self)
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

struct TypeState {
    offset: usize,
    borrow: AtomicBorrow,
}

impl TypeState {
    fn new(offset: usize) -> Self {
        Self {
            offset,
            borrow: AtomicBorrow::new(),
        }
    }
}

/// Metadata required to store a component
#[derive(Debug, Copy, Clone)]
pub struct TypeInfo {
    id: TypeId,
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
            id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            drop: drop_ptr::<T>,
        }
    }

    #[allow(missing_docs)] 
    pub fn id(&self) -> TypeId {
        self.id
    }

    #[allow(missing_docs)] 
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
