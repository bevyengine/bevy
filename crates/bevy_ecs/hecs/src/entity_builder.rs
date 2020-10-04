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
        boxed::Box,
        vec,
        vec::Vec,
    },
    world::ComponentId,
};

use bevy_utils::HashSet;
use core::{
    any::TypeId,
    mem::{self, MaybeUninit},
    ptr,
};

use crate::{archetype::TypeInfo, Component, DynamicBundle};

/// Helper for incrementally constructing a bundle of components with dynamic component types
///
/// Prefer reusing the same builder over creating new ones repeatedly.
///
/// ```
/// # use bevy_hecs::*;
/// let mut world = World::new();
/// let mut builder = EntityBuilder::new();
/// builder.add(123).add("abc");
/// let e = world.spawn(builder.build()); // builder can now be reused
/// assert_eq!(*world.get::<i32>(e).unwrap(), 123);
/// assert_eq!(*world.get::<&str>(e).unwrap(), "abc");
/// ```
pub struct EntityBuilder {
    storage: Box<[MaybeUninit<u8>]>,
    cursor: usize,
    info: Vec<(TypeInfo, usize)>,
    ids: Vec<ComponentId>,
    id_set: HashSet<ComponentId>,
}

impl EntityBuilder {
    /// Create a builder representing an entity with no components
    pub fn new() -> Self {
        Self {
            cursor: 0,
            storage: Box::new([]),
            info: Vec::new(),
            ids: Vec::new(),
            id_set: HashSet::default(),
        }
    }

    /// Add `component` to the entity
    pub fn add<T: Component>(&mut self, component: T) -> &mut Self {
        if !self.id_set.insert(TypeId::of::<T>().into()) {
            return self;
        }
        let end = self.cursor + mem::size_of::<T>();
        if end > self.storage.len() {
            self.grow(end);
        }
        if mem::size_of::<T>() != 0 {
            unsafe {
                self.storage
                    .as_mut_ptr()
                    .add(self.cursor)
                    .cast::<T>()
                    .write_unaligned(component);
            }
        }
        self.info.push((TypeInfo::of::<T>(), self.cursor));
        self.cursor += mem::size_of::<T>();
        self
    }

    fn grow(&mut self, min_size: usize) {
        let new_len = min_size.next_power_of_two().max(64);
        let mut new_storage = vec![MaybeUninit::uninit(); new_len].into_boxed_slice();
        new_storage[..self.cursor].copy_from_slice(&self.storage[..self.cursor]);
        self.storage = new_storage;
    }

    /// Construct a `Bundle` suitable for spawning
    pub fn build(&mut self) -> BuiltEntity<'_> {
        self.info.sort_unstable_by_key(|x| x.0);
        self.ids.extend(self.info.iter().map(|x| x.0.id()));
        BuiltEntity { builder: self }
    }

    /// Drop previously `add`ed components
    ///
    /// The builder is cleared implicitly when an entity is built, so this doesn't usually need to
    /// be called.
    pub fn clear(&mut self) {
        self.ids.clear();
        self.id_set.clear();
        self.cursor = 0;
        let max_size = self
            .info
            .iter()
            .map(|x| x.0.layout().size())
            .max()
            .unwrap_or(0);
        let max_align = self
            .info
            .iter()
            .map(|x| x.0.layout().align())
            .max()
            .unwrap_or(0);
        unsafe {
            // Suitably aligned storage for drop
            let tmp = if max_size > 0 {
                alloc(Layout::from_size_align(max_size, max_align).unwrap()).cast()
            } else {
                max_align as *mut _
            };
            for (ty, offset) in self.info.drain(..) {
                ptr::copy_nonoverlapping(
                    self.storage[offset..offset + ty.layout().size()]
                        .as_ptr()
                        .cast(),
                    tmp,
                    ty.layout().size(),
                );
                ty.drop(tmp);
            }
            if max_size > 0 {
                dealloc(tmp, Layout::from_size_align(max_size, max_align).unwrap())
            }
        }
    }
}

unsafe impl Send for EntityBuilder {}
unsafe impl Sync for EntityBuilder {}

impl Drop for EntityBuilder {
    fn drop(&mut self) {
        // Ensure buffered components aren't leaked
        self.clear();
    }
}

impl Default for EntityBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The output of an `EntityBuilder`, suitable for passing to `World::spawn` or `World::insert`
pub struct BuiltEntity<'a> {
    builder: &'a mut EntityBuilder,
}

impl DynamicBundle for BuiltEntity<'_> {
    fn with_ids<T>(&self, f: impl FnOnce(&[ComponentId]) -> T) -> T {
        f(&self.builder.ids)
    }

    #[doc(hidden)]
    fn type_info(&self) -> Vec<TypeInfo> {
        self.builder.info.iter().map(|x| x.0).collect()
    }

    unsafe fn put(self, mut f: impl FnMut(*mut u8, ComponentId, usize) -> bool) {
        for (ty, offset) in self.builder.info.drain(..) {
            let ptr = self.builder.storage.as_mut_ptr().add(offset).cast();
            if !f(ptr, ty.id(), ty.layout().size()) {
                ty.drop(ptr);
            }
        }
    }
}

impl Drop for BuiltEntity<'_> {
    fn drop(&mut self) {
        // Ensures components aren't leaked if `store` was never called, and prepares the builder
        // for reuse.
        self.builder.clear();
    }
}
