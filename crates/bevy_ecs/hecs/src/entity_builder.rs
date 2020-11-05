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

use bevy_utils::HashSet;

use crate::{
    alloc::{
        alloc::{alloc, dealloc, Layout},
        boxed::Box,
        vec,
        vec::Vec,
    },
    world::ComponentId,
};

use core::{intrinsics::copy_nonoverlapping, mem::MaybeUninit, ptr};
use ptr::slice_from_raw_parts;

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
        self.add_with_typeinfo(
            TypeInfo::of::<T>(),
            unsafe {
                &*slice_from_raw_parts(
                    &component as *const T as *const u8,
                    std::mem::size_of::<T>(),
                )
            },
            true,
        );
        std::mem::forget(component);
        self
    }

    /// Add a dynamic component given the component ID, the layout and the raw data slice
    #[cfg(feature = "dynamic_api")]
    pub fn add_dynamic(&mut self, info: TypeInfo, data: &[u8]) -> &mut Self {
        self.add_with_typeinfo(info, data, false);
        self
    }

    fn add_with_typeinfo(
        &mut self,
        type_info: TypeInfo,
        data: &[u8],
        skip_size_check: bool,
    ) -> &mut Self {
        if !skip_size_check {
            assert_eq!(
                type_info.layout.size(),
                data.len(),
                "Data length does not match component size"
            );
        }

        if !self.id_set.insert(type_info.id()) {
            return self;
        }
        let end = self.cursor + type_info.layout().size();
        if end > self.storage.len() {
            self.grow(end);
        }
        if type_info.layout().size() != 0 {
            unsafe {
                copy_nonoverlapping(
                    data.as_ptr(),
                    self.storage.as_mut_ptr().add(self.cursor) as *mut u8,
                    data.len(),
                );
            }
        }
        self.info.push((type_info, self.cursor));
        self.cursor += type_info.layout().size();
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

#[cfg(test)]
mod test {
    #[cfg(feature = "dynamic_api")]
    use super::*;

    #[cfg(feature = "dynamic_api")]
    #[test]
    #[should_panic(expected = "Data length does not match component size")]
    fn dynamic_data_invalid_length_panics() {
        const ID1: u64 = 242237625853274575;
        let layout1 = Layout::from_size_align(2, 1).unwrap();

        let mut builder = EntityBuilder::new();

        // This should panic because we said above that the component size was 2, and we are trying
        // to stick 3 bytes into it.
        builder.add_dynamic(TypeInfo::of_external(ID1, layout1, |_| ()), &[1, 2, 3]);
    }
}
