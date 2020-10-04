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
    alloc::vec::Vec, borrow::EntityRef, query::ReadOnlyFetch, BatchedIter, EntityReserver, Fetch,
    Mut, QueryIter, RefMut,
};
use bevy_utils::{HashMap, HashSet};
use core::{
    any::TypeId,
    cmp::{Ord, Ordering},
    fmt,
    hash::{Hash, Hasher},
    mem, ptr,
};

#[cfg(feature = "std")]
use std::error::Error;

use crate::{
    archetype::Archetype,
    entities::{Entities, Location},
    Bundle, DynamicBundle, Entity, MissingComponent, NoSuchEntity, Query, Ref,
};

/// An unordered collection of entities, each having any number of distinctly typed components
///
/// Similar to `HashMap<Entity, Vec<Box<dyn Any>>>` where each `Vec` never contains two of the same
/// type, but far more efficient to traverse.
///
/// The components of entities who have the same set of component types are stored in contiguous
/// runs, allowing for extremely fast, cache-friendly iteration.
#[derive(Debug)]
pub struct World {
    entities: Entities,
    index: HashMap<Vec<ComponentId>, u32>,
    removed_components: HashMap<ComponentId, Vec<Entity>>,
    #[allow(missing_docs)]
    pub archetypes: Vec<Archetype>,
    archetype_generation: u64,
}

impl World {
    /// Create an empty world
    pub fn new() -> Self {
        // `flush` assumes archetype 0 always exists, representing entities with no components.
        let mut archetypes = Vec::new();
        archetypes.push(Archetype::new(Vec::new()));
        let mut index = HashMap::default();
        index.insert(Vec::new(), 0);
        Self {
            entities: Entities::default(),
            index,
            archetypes,
            archetype_generation: 0,
            removed_components: HashMap::default(),
        }
    }

    /// Create an entity with certain components
    ///
    /// Returns the ID of the newly created entity.
    ///
    /// Arguments can be tuples, structs annotated with `#[derive(Bundle)]`, or the result of
    /// calling `build` on an `EntityBuilder`, which is useful if the set of components isn't
    /// statically known. To spawn an entity with only one component, use a one-element tuple like
    /// `(x,)`.
    ///
    /// Any type that satisfies `Send + Sync + 'static` can be used as a component.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let a = world.spawn((123, "abc"));
    /// let b = world.spawn((456, true));
    /// ```
    pub fn spawn(&mut self, components: impl DynamicBundle) -> Entity {
        // Ensure all entity allocations are accounted for so `self.entities` can realloc if
        // necessary
        self.flush();

        let entity = self.entities.alloc();
        let archetype_id = components.with_ids(|ids| {
            self.index.get(ids).copied().unwrap_or_else(|| {
                let x = self.archetypes.len() as u32;
                self.archetypes.push(Archetype::new(components.type_info()));
                self.index.insert(ids.to_vec(), x);
                self.archetype_generation += 1;
                x
            })
        });

        let archetype = &mut self.archetypes[archetype_id as usize];
        unsafe {
            let index = archetype.allocate(entity);
            components.put(|ptr, ty, size| {
                archetype.put_dynamic(ptr, ty, size, index, true);
                true
            });
            self.entities.meta[entity.id as usize].location = Location {
                archetype: archetype_id,
                index,
            };
        }

        entity
    }

    /// Efficiently spawn a large number of entities with the same components
    ///
    /// Faster than calling `spawn` repeatedly with the same components.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let entities = world.spawn_batch((0..1_000).map(|i| (i, "abc"))).collect::<Vec<_>>();
    /// for i in 0..1_000 {
    ///     assert_eq!(*world.get::<i32>(entities[i]).unwrap(), i as i32);
    /// }
    /// ```
    pub fn spawn_batch<I>(&mut self, iter: I) -> SpawnBatchIter<'_, I::IntoIter>
    where
        I: IntoIterator,
        I::Item: Bundle,
    {
        // Ensure all entity allocations are accounted for so `self.entities` can realloc if
        // necessary
        self.flush();

        let iter = iter.into_iter();
        let (lower, upper) = iter.size_hint();
        let archetype_id = self.reserve_inner::<I::Item>(upper.unwrap_or(lower) as u32);

        SpawnBatchIter {
            inner: iter,
            entities: &mut self.entities,
            archetype_id,
            archetype: &mut self.archetypes[archetype_id as usize],
        }
    }

    /// Destroy an entity and all its components
    pub fn despawn(&mut self, entity: Entity) -> Result<(), NoSuchEntity> {
        self.flush();

        let loc = self.entities.free(entity)?;
        let archetype = &mut self.archetypes[loc.archetype as usize];
        if let Some(moved) = unsafe { archetype.remove(loc.index) } {
            self.entities.get_mut(moved).unwrap().index = loc.index;
        }
        for ty in archetype.types() {
            let removed_entities = self
                .removed_components
                .entry(ty.id())
                .or_insert_with(Vec::new);
            removed_entities.push(entity);
        }
        Ok(())
    }

    /// Ensure `additional` entities with exact components `T` can be spawned without reallocating
    pub fn reserve<T: Bundle>(&mut self, additional: u32) {
        self.reserve_inner::<T>(additional);
    }

    /// Reserves an entity.
    pub fn reserve_entity(&self) -> Entity {
        self.entities.reserve_entity()
    }

    fn reserve_inner<T: Bundle>(&mut self, additional: u32) -> u32 {
        self.flush();
        self.entities.reserve(additional);

        let archetype_id = T::with_static_ids(|ids| {
            self.index.get(ids).copied().unwrap_or_else(|| {
                let x = self.archetypes.len() as u32;
                self.archetypes.push(Archetype::new(T::static_type_info()));
                self.index.insert(ids.to_vec(), x);
                self.archetype_generation += 1;
                x
            })
        });

        self.archetypes[archetype_id as usize].reserve(additional as usize);
        archetype_id
    }

    /// Despawn all entities
    ///
    /// Preserves allocated storage for reuse.
    pub fn clear(&mut self) {
        for archetype in &mut self.archetypes {
            for ty in archetype.types() {
                let removed_entities = self
                    .removed_components
                    .entry(ty.id())
                    .or_insert_with(Vec::new);
                removed_entities.extend(archetype.iter_entities().copied());
            }
            archetype.clear();
        }
        self.entities.clear();
    }

    /// Whether `entity` still exists
    pub fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(entity)
    }

    /// Returns true if the given entity has a component with the given type id.
    pub fn has_component_type(&self, entity: Entity, ty: ComponentId) -> bool {
        self.get_entity_location(entity)
            .map(|location| &self.archetypes[location.archetype as usize])
            .map(|archetype| archetype.has_component(ty))
            .unwrap_or(false)
    }

    /// Efficiently iterate over all entities that have certain components
    ///
    /// Calling `iter` on the returned value yields `(Entity, Q)` tuples, where `Q` is some query
    /// type. A query type is `&T`, `&mut T`, a tuple of query types, or an `Option` wrapping a
    /// query type, where `T` is any component type. Components queried with `&mut` must only appear
    /// once. Entities which do not have a component type referenced outside of an `Option` will be
    /// skipped.
    ///
    /// Entities are yielded in arbitrary order.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let a = world.spawn((123, true, "abc"));
    /// let b = world.spawn((456, false));
    /// let c = world.spawn((42, "def"));
    /// let entities = world.query::<(Entity, &i32, &bool)>()
    ///     .map(|(e, &i, &b)| (e, i, b)) // Copy out of the world
    ///     .collect::<Vec<_>>();
    /// assert_eq!(entities.len(), 2);
    /// assert!(entities.contains(&(a, 123, true)));
    /// assert!(entities.contains(&(b, 456, false)));
    /// ```
    pub fn query<Q: Query>(&self) -> QueryIter<'_, Q>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: read-only access to world and read only query prevents mutable access
        unsafe { self.query_unchecked() }
    }

    /// Efficiently iterate over all entities that have certain components
    ///
    /// Calling `iter` on the returned value yields `(Entity, Q)` tuples, where `Q` is some query
    /// type. A query type is `&T`, `&mut T`, a tuple of query types, or an `Option` wrapping a
    /// query type, where `T` is any component type. Components queried with `&mut` must only appear
    /// once. Entities which do not have a component type referenced outside of an `Option` will be
    /// skipped.
    ///
    /// Entities are yielded in arbitrary order.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let a = world.spawn((123, true, "abc"));
    /// let b = world.spawn((456, false));
    /// let c = world.spawn((42, "def"));
    /// let entities = world.query_mut::<(Entity, &mut i32, &bool)>()
    ///     .map(|(e, i, &b)| (e, *i, b)) // Copy out of the world
    ///     .collect::<Vec<_>>();
    /// assert_eq!(entities.len(), 2);
    /// assert!(entities.contains(&(a, 123, true)));
    /// assert!(entities.contains(&(b, 456, false)));
    /// ```
    pub fn query_mut<Q: Query>(&mut self) -> QueryIter<'_, Q> {
        // SAFE: unique mutable access
        unsafe { self.query_unchecked() }
    }

    /// Like `query`, but instead of returning a single iterator it returns a "batched iterator",
    /// where each batch is `batch_size`. This is generally used for parallel iteration.
    pub fn query_batched<Q: Query>(&self, batch_size: usize) -> BatchedIter<'_, Q>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: read-only access to world and read only query prevents mutable access
        unsafe { self.query_batched_unchecked(batch_size) }
    }

    /// Like `query`, but instead of returning a single iterator it returns a "batched iterator",
    /// where each batch is `batch_size`. This is generally used for parallel iteration.
    pub fn query_batched_mut<Q: Query>(&mut self, batch_size: usize) -> BatchedIter<'_, Q> {
        // SAFE: unique mutable access
        unsafe { self.query_batched_unchecked(batch_size) }
    }

    /// Efficiently iterate over all entities that have certain components
    ///
    /// Calling `iter` on the returned value yields `(Entity, Q)` tuples, where `Q` is some query
    /// type. A query type is `&T`, `&mut T`, a tuple of query types, or an `Option` wrapping a
    /// query type, where `T` is any component type. Components queried with `&mut` must only appear
    /// once. Entities which do not have a component type referenced outside of an `Option` will be
    /// skipped.
    ///
    /// Entities are yielded in arbitrary order.
    ///
    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    pub unsafe fn query_unchecked<Q: Query>(&self) -> QueryIter<'_, Q> {
        QueryIter::new(&self.archetypes)
    }

    /// Like `query`, but instead of returning a single iterator it returns a "batched iterator",
    /// where each batch is `batch_size`. This is generally used for parallel iteration.
    ///
    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn query_batched_unchecked<Q: Query>(
        &self,
        batch_size: usize,
    ) -> BatchedIter<'_, Q> {
        BatchedIter::new(&self.archetypes, batch_size)
    }

    /// Prepare a read only query against a single entity
    ///
    /// Handy for accessing multiple components simultaneously.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let a = world.spawn((123, true, "abc"));
    /// // The returned query must outlive the borrow made by `get`
    /// let (number, flag) = world.query_one::<(&i32, &bool)>(a).unwrap();
    /// assert_eq!(*number, 123);
    /// ```
    pub fn query_one<Q: Query>(
        &self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, NoSuchEntity>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: read-only access to world and read only query prevents mutable access
        unsafe { self.query_one_unchecked::<Q>(entity) }
    }

    /// Prepare a query against a single entity
    ///
    /// Handy for accessing multiple components simultaneously.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let a = world.spawn((123, true, "abc"));
    /// // The returned query must outlive the borrow made by `get`
    /// let (mut number, flag) = world.query_one_mut::<(&mut i32, &bool)>(a).unwrap();
    /// if *flag { *number *= 2; }
    /// assert_eq!(*number, 246);
    /// ```
    pub fn query_one_mut<Q: Query>(
        &mut self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, NoSuchEntity> {
        // SAFE: unique mutable access to world
        unsafe { self.query_one_unchecked::<Q>(entity) }
    }

    /// Prepare a query against a single entity, without checking the safety of mutable queries
    ///
    /// Handy for accessing multiple components simultaneously.
    ///
    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    pub unsafe fn query_one_unchecked<Q: Query>(
        &self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, NoSuchEntity> {
        let loc = self.entities.get(entity)?;
        <Q::Fetch as Fetch>::get(&self.archetypes[loc.archetype as usize], 0)
            .filter(|fetch| !fetch.should_skip(loc.index))
            .map(|fetch| fetch.fetch(loc.index))
            .ok_or(NoSuchEntity)
    }

    /// Borrow the `T` component of `entity`
    pub fn get<T: Component>(&self, entity: Entity) -> Result<&'_ T, ComponentError> {
        unsafe {
            let loc = self.entities.get(entity)?;
            if loc.archetype == 0 {
                return Err(MissingComponent::new::<T>().into());
            }
            Ok(&*self.archetypes[loc.archetype as usize]
                .get::<T>()
                .ok_or_else(MissingComponent::new::<T>)?
                .as_ptr()
                .add(loc.index as usize))
        }
    }

    /// Mutably borrow the `T` component of `entity`
    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Result<Mut<'_, T>, ComponentError> {
        // SAFE: uniquely borrows world
        unsafe { self.get_mut_unchecked(entity) }
    }

    /// Access an entity regardless of its component types
    ///
    /// Does not immediately borrow any component.
    pub fn entity(&mut self, entity: Entity) -> Result<EntityRef<'_>, NoSuchEntity> {
        Ok(match self.entities.get(entity)? {
            Location { archetype: 0, .. } => EntityRef::empty(),
            loc => unsafe { EntityRef::new(&self.archetypes[loc.archetype as usize], loc.index) },
        })
    }

    /// Borrow the `T` component of `entity` without checking if it can be mutated
    ///
    /// # Safety
    /// This does not check for mutable access correctness. To be safe, make sure this is the only
    /// thing accessing this entity's T component.
    pub unsafe fn get_mut_unchecked<T: Component>(
        &self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, ComponentError> {
        let loc = self.entities.get(entity)?;
        if loc.archetype == 0 {
            return Err(MissingComponent::new::<T>().into());
        }
        Ok(Mut::new(
            &self.archetypes[loc.archetype as usize],
            loc.index,
        )?)
    }

    /// Iterate over all entities in the world
    ///
    /// Entities are yielded in arbitrary order. Prefer `World::query` for better performance when
    /// components will be accessed in predictable patterns.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let a = world.spawn(());
    /// let b = world.spawn(());
    /// let ids = world.iter().map(|(id, _)| id).collect::<Vec<_>>();
    /// assert_eq!(ids.len(), 2);
    /// assert!(ids.contains(&a));
    /// assert!(ids.contains(&b));
    /// ```
    pub fn iter(&mut self) -> Iter<'_> {
        Iter::new(&self.archetypes, &self.entities)
    }

    #[allow(missing_docs)]
    pub fn removed<C: Component>(&self) -> &[Entity] {
        self.removed_component(std::any::TypeId::of::<C>().into())
    }

    #[allow(missing_docs)]
    pub fn removed_component(&self, id: ComponentId) -> &[Entity] {
        self.removed_components
            .get(&id)
            .map_or(&[], |entities| entities.as_slice())
    }

    /// Add `components` to `entity`
    ///
    /// Computational cost is proportional to the number of components `entity` has. If an entity
    /// already has a component of a certain type, it is dropped and replaced.
    ///
    /// When inserting a single component, see `insert_one` for convenience.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let e = world.spawn((123, "abc"));
    /// world.insert(e, (456, true));
    /// assert_eq!(*world.get::<i32>(e).unwrap(), 456);
    /// assert_eq!(*world.get::<bool>(e).unwrap(), true);
    /// ```
    pub fn insert(
        &mut self,
        entity: Entity,
        components: impl DynamicBundle,
    ) -> Result<(), NoSuchEntity> {
        use std::collections::hash_map::Entry;

        self.flush();
        let loc = self.entities.get_mut(entity)?;
        unsafe {
            // Assemble Vec<TypeInfo> for the final entity
            let arch = &mut self.archetypes[loc.archetype as usize];
            let mut info = arch.types().to_vec();
            for ty in components.type_info() {
                if let Some(ptr) = arch.get_dynamic(ty.id(), ty.layout().size(), loc.index) {
                    ty.drop(ptr.as_ptr());
                } else {
                    info.push(ty);
                }
            }
            info.sort();

            // Find the archetype it'll live in
            let elements = info.iter().map(|x| x.id()).collect::<Vec<_>>();
            let target = match self.index.entry(elements) {
                Entry::Occupied(x) => *x.get(),
                Entry::Vacant(x) => {
                    let index = self.archetypes.len() as u32;
                    self.archetypes.push(Archetype::new(info));
                    x.insert(index);
                    self.archetype_generation += 1;
                    index
                }
            };

            if target == loc.archetype {
                // Update components in the current archetype
                let arch = &mut self.archetypes[loc.archetype as usize];
                components.put(|ptr, ty, size| {
                    arch.put_dynamic(ptr, ty, size, loc.index, false);
                    true
                });
                return Ok(());
            }

            // Move into a new archetype
            let (source_arch, target_arch) = index2(
                &mut self.archetypes,
                loc.archetype as usize,
                target as usize,
            );
            let target_index = target_arch.allocate(entity);
            loc.archetype = target;
            let old_index = mem::replace(&mut loc.index, target_index);
            if let Some(moved) =
                source_arch.move_to(old_index, |ptr, ty, size, is_added, is_mutated| {
                    target_arch.put_dynamic(ptr, ty, size, target_index, false);
                    let type_state = target_arch.get_type_state_mut(ty).unwrap();
                    *type_state.added().as_ptr().add(target_index) = is_added;
                    *type_state.mutated().as_ptr().add(target_index) = is_mutated;
                })
            {
                self.entities.get_mut(moved).unwrap().index = old_index;
            }

            components.put(|ptr, ty, size| {
                target_arch.put_dynamic(ptr, ty, size, target_index, true);
                true
            });
        }
        Ok(())
    }

    /// Add `component` to `entity`
    ///
    /// See `insert`.
    pub fn insert_one(
        &mut self,
        entity: Entity,
        component: impl Component,
    ) -> Result<(), NoSuchEntity> {
        self.insert(entity, (component,))
    }

    /// Remove components from `entity`
    ///
    /// Computational cost is proportional to the number of components `entity` has. The entity
    /// itself is not removed, even if no components remain; use `despawn` for that. If any
    /// component in `T` is not present in `entity`, no components are removed and an error is
    /// returned.
    ///
    /// When removing a single component, see `remove_one` for convenience.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let e = world.spawn((123, "abc", true));
    /// assert_eq!(world.remove::<(i32, &str)>(e), Ok((123, "abc")));
    /// assert!(world.get::<i32>(e).is_err());
    /// assert!(world.get::<&str>(e).is_err());
    /// assert_eq!(*world.get::<bool>(e).unwrap(), true);
    /// ```
    pub fn remove<T: Bundle>(&mut self, entity: Entity) -> Result<T, ComponentError> {
        use std::collections::hash_map::Entry;

        self.flush();
        let loc = self.entities.get_mut(entity)?;
        unsafe {
            let removed = T::with_static_ids(|ids| ids.iter().copied().collect::<HashSet<_>>());
            let info = self.archetypes[loc.archetype as usize]
                .types()
                .iter()
                .cloned()
                .filter(|x| !removed.contains(&x.id()))
                .collect::<Vec<_>>();
            let elements = info.iter().map(|x| x.id()).collect::<Vec<_>>();
            let target = match self.index.entry(elements) {
                Entry::Occupied(x) => *x.get(),
                Entry::Vacant(x) => {
                    self.archetypes.push(Archetype::new(info));
                    let index = (self.archetypes.len() - 1) as u32;
                    x.insert(index);
                    self.archetype_generation += 1;
                    index
                }
            };
            let old_index = loc.index;
            let source_arch = &self.archetypes[loc.archetype as usize];
            let bundle = T::get(|ty, size| source_arch.get_dynamic(ty, size, old_index))?;
            let (source_arch, target_arch) = index2(
                &mut self.archetypes,
                loc.archetype as usize,
                target as usize,
            );
            let target_index = target_arch.allocate(entity);
            loc.archetype = target;
            loc.index = target_index;
            let removed_components = &mut self.removed_components;
            if let Some(moved) =
                source_arch.move_to(old_index, |src, ty, size, is_added, is_mutated| {
                    // Only move the components present in the target archetype, i.e. the non-removed ones.
                    if let Some(dst) = target_arch.get_dynamic(ty, size, target_index) {
                        ptr::copy_nonoverlapping(src, dst.as_ptr(), size);
                        let state = target_arch.get_type_state_mut(ty).unwrap();
                        *state.added().as_ptr().add(target_index) = is_added;
                        *state.mutated().as_ptr().add(target_index) = is_mutated;
                    } else {
                        let removed_entities =
                            removed_components.entry(ty).or_insert_with(Vec::new);
                        removed_entities.push(entity);
                    }
                })
            {
                self.entities.get_mut(moved).unwrap().index = old_index;
            }
            Ok(bundle)
        }
    }

    /// Remove the `T` component from `entity`
    ///
    /// See `remove`.
    pub fn remove_one<T: Component>(&mut self, entity: Entity) -> Result<T, ComponentError> {
        self.remove::<(T,)>(entity).map(|(x,)| x)
    }

    /// Borrow the `T` component at the given location, without safety checks
    ///
    /// # Safety
    /// This does not check that the location is within bounds of the archetype.
    pub unsafe fn get_ref_at_location_unchecked<T: Component>(
        &self,
        location: Location,
    ) -> Result<Ref<T>, ComponentError> {
        if location.archetype == 0 {
            return Err(MissingComponent::new::<T>().into());
        }
        Ok(Ref::new(
            &self.archetypes[location.archetype as usize],
            location.index,
        )?)
    }

    /// Borrow the `T` component at the given location, without safety checks
    ///
    /// # Safety
    /// This does not check that the location is within bounds of the archetype.
    /// It also does not check for mutable access correctness. To be safe, make sure this is the only
    /// thing accessing this entity's T component.
    pub unsafe fn get_ref_mut_at_location_unchecked<T: Component>(
        &self,
        location: Location,
    ) -> Result<RefMut<T>, ComponentError> {
        if location.archetype == 0 {
            return Err(MissingComponent::new::<T>().into());
        }
        Ok(RefMut::new(
            &self.archetypes[location.archetype as usize],
            location.index,
        )?)
    }

    /// Borrow the `T` component at the given location, without safety checks
    /// # Safety
    /// This does not check that the location is within bounds of the archetype.
    pub unsafe fn get_at_location_unchecked<T: Component>(
        &self,
        location: Location,
    ) -> Result<&T, ComponentError> {
        if location.archetype == 0 {
            return Err(MissingComponent::new::<T>().into());
        }
        Ok(&*self.archetypes[location.archetype as usize]
            .get::<T>()
            .ok_or_else(MissingComponent::new::<T>)?
            .as_ptr()
            .add(location.index as usize))
    }

    /// Borrow the `T` component at the given location, without safety checks
    /// # Safety
    /// This does not check that the location is within bounds of the archetype.
    /// It also does not check for mutable access correctness. To be safe, make sure this is the only
    /// thing accessing this entity's T component.
    pub unsafe fn get_mut_at_location_unchecked<T: Component>(
        &self,
        location: Location,
    ) -> Result<Mut<T>, ComponentError> {
        if location.archetype == 0 {
            return Err(MissingComponent::new::<T>().into());
        }
        Ok(Mut::new(
            &self.archetypes[location.archetype as usize],
            location.index,
        )?)
    }

    /// Uniquely borrow the `T` component of `entity` without safety checks
    ///
    /// Should only be used as a building block for safe abstractions.
    ///
    /// # Safety
    ///
    /// `entity` must have been previously obtained from this `World`, and no borrow of the same
    /// component of `entity` may be live simultaneous to the returned reference.
    pub unsafe fn get_unchecked_mut<T: Component>(
        &self,
        entity: Entity,
    ) -> Result<&mut T, ComponentError> {
        let loc = self.entities.get(entity)?;
        if loc.archetype == 0 {
            return Err(MissingComponent::new::<T>().into());
        }
        Ok(&mut *self.archetypes[loc.archetype as usize]
            .get::<T>()
            .ok_or_else(MissingComponent::new::<T>)?
            .as_ptr()
            .add(loc.index as usize))
    }

    /// Convert all reserved entities into empty entities that can be iterated and accessed
    ///
    /// Invoked implicitly by `spawn`, `despawn`, `insert`, and `remove`.
    pub fn flush(&mut self) {
        let arch = &mut self.archetypes[0];
        for entity_id in self.entities.flush() {
            self.entities.meta[entity_id as usize].location.index = unsafe {
                arch.allocate(Entity {
                    id: entity_id,
                    generation: self.entities.meta[entity_id as usize].generation,
                })
            };
        }
        for i in 0..self.entities.reserved_len() {
            let id = self.entities.reserved(i);
            self.entities.meta[id as usize].location.index = unsafe {
                arch.allocate(Entity {
                    id,
                    generation: self.entities.meta[id as usize].generation,
                })
            };
        }
        self.entities.clear_reserved();
    }

    /// Inspect the archetypes that entities are organized into
    ///
    /// Useful for dynamically scheduling concurrent queries by checking borrows in advance. Does
    /// not provide access to entities.
    pub fn archetypes(&self) -> impl ExactSizeIterator<Item = &'_ Archetype> + '_ {
        self.archetypes.iter()
    }

    /// Returns a distinct value after `archetypes` is changed
    ///
    /// Store the current value after deriving information from `archetypes`, then check whether the
    /// value returned by this function differs before attempting an operation that relies on its
    /// correctness. Useful for determining whether e.g. a concurrent query execution plan is still
    /// correct.
    ///
    /// The generation may be, but is not necessarily, changed as a result of adding or removing any
    /// entity or component.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let initial_gen = world.archetypes_generation();
    /// world.spawn((123, "abc"));
    /// assert_ne!(initial_gen, world.archetypes_generation());
    /// ```
    pub fn archetypes_generation(&self) -> ArchetypesGeneration {
        ArchetypesGeneration(self.archetype_generation)
    }

    /// Retrieves the entity's current location, if it exists
    pub fn get_entity_location(&self, entity: Entity) -> Option<Location> {
        self.entities.get(entity).ok()
    }

    /// Clears each entity's tracker state. For example, each entity's component "mutated" state will be reset to `false`.
    pub fn clear_trackers(&mut self) {
        for archetype in self.archetypes.iter_mut() {
            archetype.clear_trackers();
        }

        self.removed_components.clear();
    }

    /// Gets an entity reserver, which can be used to reserve entity ids in a multi-threaded context.
    pub fn get_entity_reserver(&self) -> EntityReserver {
        self.entities.get_reserver()
    }
}

unsafe impl Send for World {}
unsafe impl Sync for World {}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a mut World {
    type IntoIter = Iter<'a>;
    type Item = (Entity, EntityRef<'a>);

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

fn index2<T>(x: &mut [T], i: usize, j: usize) -> (&mut T, &mut T) {
    assert!(i != j);
    assert!(i < x.len());
    assert!(j < x.len());
    let ptr = x.as_mut_ptr();
    unsafe { (&mut *ptr.add(i), &mut *ptr.add(j)) }
}

/// Errors that arise when accessing components
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ComponentError {
    /// The entity was already despawned
    NoSuchEntity,
    /// The entity did not have a requested component
    MissingComponent(MissingComponent),
}

#[cfg(feature = "std")]
impl Error for ComponentError {}

impl fmt::Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ComponentError::*;
        match *self {
            NoSuchEntity => f.write_str("no such entity"),
            MissingComponent(ref x) => x.fmt(f),
        }
    }
}

impl From<NoSuchEntity> for ComponentError {
    fn from(NoSuchEntity: NoSuchEntity) -> Self {
        ComponentError::NoSuchEntity
    }
}

impl From<MissingComponent> for ComponentError {
    fn from(x: MissingComponent) -> Self {
        ComponentError::MissingComponent(x)
    }
}

/// Uniquely identifies a type of component. This is conceptually similar to
/// Rust's [`TypeId`], but allows for external type IDs to be defined.
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum ComponentId {
    /// A Rust-native [`TypeId`]
    RustTypeId(TypeId),
    /// An arbitrary ID that allows you to identify types defined outside of
    /// this Rust compilation
    ExternalId(u64),
}

#[allow(clippy::derive_hash_xor_eq)] // Fine because we uphold k1 == k2 ⇒ hash(k1) == hash(k2)
impl Hash for ComponentId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            ComponentId::RustTypeId(id) => {
                id.hash(state);
            }
            ComponentId::ExternalId(id) => {
                state.write_u64(*id);
            }
        }
    }
}

impl Ord for ComponentId {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        } else {
            // Sort RustTypeId's as greater than external ids and then sort
            // matching types by their default Ord implementation.
            match self {
                ComponentId::RustTypeId(lhs_rid) => match other {
                    ComponentId::RustTypeId(rhs_rid) => lhs_rid.cmp(rhs_rid),
                    ComponentId::ExternalId(_) => Ordering::Less,
                },
                ComponentId::ExternalId(lhs_eid) => match other {
                    ComponentId::RustTypeId(_) => Ordering::Greater,
                    ComponentId::ExternalId(rhs_edi) => lhs_eid.cmp(rhs_edi),
                },
            }
        }
    }
}

impl PartialOrd for ComponentId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<TypeId> for ComponentId {
    fn from(item: TypeId) -> Self {
        ComponentId::RustTypeId(item)
    }
}

/// Types that can be components, implemented automatically for all `Send + Sync + 'static` types
///
/// This is just a convenient shorthand for `Send + Sync + 'static`, and never needs to be
/// implemented manually.
pub trait Component: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Component for T {}

/// Iterator over all of a world's entities
pub struct Iter<'a> {
    archetypes: core::slice::Iter<'a, Archetype>,
    entities: &'a Entities,
    current: Option<&'a Archetype>,
    index: usize,
}

impl<'a> Iter<'a> {
    fn new(archetypes: &'a [Archetype], entities: &'a Entities) -> Self {
        Self {
            archetypes: archetypes.iter(),
            entities,
            current: None,
            index: 0,
        }
    }
}

unsafe impl Send for Iter<'_> {}
unsafe impl Sync for Iter<'_> {}

impl<'a> Iterator for Iter<'a> {
    type Item = (Entity, EntityRef<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.current {
                None => {
                    self.current = Some(self.archetypes.next()?);
                    self.index = 0;
                }
                Some(current) => {
                    if self.index == current.len() {
                        self.current = None;
                        continue;
                    }
                    let index = self.index;
                    self.index += 1;
                    let id = current.get_entity(index);
                    return Some((id, unsafe { EntityRef::new(current, index) }));
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.entities.meta.len()))
    }
}

impl<A: DynamicBundle> Extend<A> for World {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = A>,
    {
        for x in iter {
            self.spawn(x);
        }
    }
}

impl<A: DynamicBundle> core::iter::FromIterator<A> for World {
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        let mut world = World::new();
        world.extend(iter);
        world
    }
}

/// Determines freshness of information derived from `World::archetypes`
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ArchetypesGeneration(pub u64);

/// Entity IDs created by `World::spawn_batch`
pub struct SpawnBatchIter<'a, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    inner: I,
    entities: &'a mut Entities,
    archetype_id: u32,
    archetype: &'a mut Archetype,
}

impl<I> Drop for SpawnBatchIter<'_, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    fn drop(&mut self) {
        for _ in self {}
    }
}

impl<I> Iterator for SpawnBatchIter<'_, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let components = self.inner.next()?;
        let entity = self.entities.alloc();
        unsafe {
            let index = self.archetype.allocate(entity);
            components.put(|ptr, ty, size| {
                self.archetype.put_dynamic(ptr, ty, size, index, true);
                true
            });
            self.entities.meta[entity.id as usize].location = Location {
                archetype: self.archetype_id,
                index,
            };
        }
        Some(entity)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I, T> ExactSizeIterator for SpawnBatchIter<'_, I>
where
    I: ExactSizeIterator<Item = T>,
    T: Bundle,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}
