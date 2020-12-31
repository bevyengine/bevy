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

use super::{Archetype, Component, Entity, MissingComponent, QueryAccess, QueryFilter};
use crate::{ComponentFlags, EntityFilter};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    vec,
};

/// A collection of component types to fetch from a `World`
pub trait WorldQuery {
    #[doc(hidden)]
    type Fetch: for<'a> Fetch<'a>;
}

/// A fetch that is read only. This should only be implemented for read-only fetches.
pub unsafe trait ReadOnlyFetch {}

/// Streaming iterators over contiguous homogeneous ranges of components
pub trait Fetch<'a>: Sized {
    /// Type of value to be fetched
    type Item;

    /// A value on which `get` may never be called
    #[allow(clippy::declare_interior_mutable_const)] // no const fn in traits
    const DANGLING: Self;

    /// How this query will access `archetype`, if at all
    fn access() -> QueryAccess;

    /// Construct a `Fetch` for `archetype` if it should be traversed
    ///
    /// # Safety
    /// `offset` must be in bounds of `archetype`
    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self>;

    /// Access the `n`th item in this archetype without bounds checking
    ///
    /// # Safety
    /// - Must only be called after `borrow`
    /// - `release` must not be called while `'a` is still live
    /// - Bounds-checking must be performed externally
    /// - Any resulting borrows must be legal (e.g. no &mut to something another iterator might access)
    unsafe fn fetch(&self, n: usize) -> Self::Item;
}

#[derive(Copy, Clone, Debug)]
pub struct EntityFetch(NonNull<Entity>);
unsafe impl ReadOnlyFetch for EntityFetch {}

impl WorldQuery for Entity {
    type Fetch = EntityFetch;
}

impl<'a> Fetch<'a> for EntityFetch {
    type Item = Entity;

    const DANGLING: Self = Self(NonNull::dangling());

    #[inline]
    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        Some(EntityFetch(NonNull::new_unchecked(
            archetype.entities().as_ptr().add(offset),
        )))
    }

    #[inline]
    unsafe fn fetch(&self, n: usize) -> Self::Item {
        *self.0.as_ptr().add(n)
    }

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::None
    }
}

impl<'a, T: Component> WorldQuery for &'a T {
    type Fetch = FetchRead<T>;
}

#[doc(hidden)]
pub struct FetchRead<T>(NonNull<T>);

unsafe impl<T> ReadOnlyFetch for FetchRead<T> {}

impl<'a, T: Component> Fetch<'a> for FetchRead<T> {
    type Item = &'a T;

    const DANGLING: Self = Self(NonNull::dangling());

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        archetype
            .get::<T>()
            .map(|x| Self(NonNull::new_unchecked(x.as_ptr().add(offset))))
    }

    #[inline]
    unsafe fn fetch(&self, n: usize) -> &'a T {
        &*self.0.as_ptr().add(n)
    }

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }
}

impl<'a, T: Component> WorldQuery for &'a mut T {
    type Fetch = FetchMut<T>;
}

impl<T: WorldQuery> WorldQuery for Option<T> {
    type Fetch = TryFetch<T::Fetch>;
}

/// Flags on component `T` that happened since the start of the frame.
#[derive(Debug, Clone)]
pub struct Flags<T: Component> {
    _marker: std::marker::PhantomData<T>,
    with: bool,
    added: bool,
    mutated: bool,
}

impl<T: Component> Flags<T> {
    /// Does the entity have this component
    pub fn with(&self) -> bool {
        self.with
    }

    /// Has this component been added since the start of the frame.
    pub fn added(&self) -> bool {
        self.added
    }

    /// Has this component been mutated since the start of the frame.
    pub fn mutated(&self) -> bool {
        self.mutated
    }

    /// Has this component been either mutated or added since the start of the frame.
    pub fn changed(&self) -> bool {
        self.added || self.mutated
    }
}

impl<T: Component> WorldQuery for Flags<T> {
    type Fetch = FlagsFetch<T>;
}

/// Unique borrow of an entity's component
pub struct Mut<'a, T: Component> {
    pub(crate) value: &'a mut T,
    pub(crate) flags: &'a mut ComponentFlags,
}

impl<'a, T: Component> Mut<'a, T> {
    /// Creates a new mutable reference to a component. This is unsafe because the index bounds are not checked.
    ///
    /// # Safety
    /// This doesn't check the bounds of index in archetype
    pub unsafe fn new(archetype: &'a Archetype, index: usize) -> Result<Self, MissingComponent> {
        let (target, type_state) = archetype
            .get_with_type_state::<T>()
            .ok_or_else(MissingComponent::new::<T>)?;
        Ok(Self {
            value: &mut *target.as_ptr().add(index),
            flags: &mut *type_state.component_flags().as_ptr().add(index),
        })
    }
}

unsafe impl<T: Component> Send for Mut<'_, T> {}
unsafe impl<T: Component> Sync for Mut<'_, T> {}

impl<'a, T: Component> Deref for Mut<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T: Component> DerefMut for Mut<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.flags.insert(ComponentFlags::MUTATED);
        self.value
    }
}

impl<'a, T: Component + core::fmt::Debug> core::fmt::Debug for Mut<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.value.fmt(f)
    }
}

impl<'a, T: Component> WorldQuery for Mut<'a, T> {
    type Fetch = FetchMut<T>;
}
#[doc(hidden)]
pub struct FetchMut<T>(NonNull<T>, NonNull<ComponentFlags>);

impl<'a, T: Component> Fetch<'a> for FetchMut<T> {
    type Item = Mut<'a, T>;

    const DANGLING: Self = Self(NonNull::dangling(), NonNull::dangling());

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        archetype
            .get_with_type_state::<T>()
            .map(|(components, type_state)| {
                Self(
                    NonNull::new_unchecked(components.as_ptr().add(offset)),
                    NonNull::new_unchecked(type_state.component_flags().as_ptr().add(offset)),
                )
            })
    }

    #[inline]
    unsafe fn fetch(&self, n: usize) -> Mut<'a, T> {
        Mut {
            value: &mut *self.0.as_ptr().add(n),
            flags: &mut *self.1.as_ptr().add(n),
        }
    }

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::write::<T>()
    }
}

#[doc(hidden)]
pub struct TryFetch<T>(Option<T>);
unsafe impl<T> ReadOnlyFetch for TryFetch<T> where T: ReadOnlyFetch {}

impl<'a, T: Fetch<'a>> Fetch<'a> for TryFetch<T> {
    type Item = Option<T::Item>;

    const DANGLING: Self = Self(None);

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::optional(T::access())
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        Some(Self(T::get(archetype, offset)))
    }

    unsafe fn fetch(&self, n: usize) -> Option<T::Item> {
        Some(self.0.as_ref()?.fetch(n))
    }
}

#[doc(hidden)]
pub struct FlagsFetch<T>(Option<NonNull<ComponentFlags>>, PhantomData<T>);
unsafe impl<T> ReadOnlyFetch for FlagsFetch<T> {}

impl<'a, T: Component> Fetch<'a> for FlagsFetch<T> {
    type Item = Flags<T>;

    const DANGLING: Self = Self(None, PhantomData::<T>);

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        Some(Self(
            archetype
                .get_type_state(std::any::TypeId::of::<T>())
                .map(|type_state| {
                    NonNull::new_unchecked(type_state.component_flags().as_ptr().add(offset))
                }),
            PhantomData::<T>,
        ))
    }

    unsafe fn fetch(&self, n: usize) -> Self::Item {
        if let Some(flags) = self.0.as_ref() {
            let flags = *flags.as_ptr().add(n);
            Self::Item {
                _marker: PhantomData::<T>,
                with: true,
                added: flags.contains(ComponentFlags::ADDED),
                mutated: flags.contains(ComponentFlags::MUTATED),
            }
        } else {
            Self::Item {
                _marker: PhantomData::<T>,
                with: false,
                added: false,
                mutated: false,
            }
        }
    }
}

struct ChunkInfo<Q: WorldQuery, F: QueryFilter> {
    fetch: Q::Fetch,
    filter: F::EntityFilter,
    len: usize,
}

/// Iterator over the set of entities with the components in `Q`
pub struct QueryIter<'w, Q: WorldQuery, F: QueryFilter> {
    archetypes: &'w [Archetype],
    archetype_index: usize,
    chunk_info: ChunkInfo<Q, F>,
    chunk_position: usize,
}

impl<'w, Q: WorldQuery, F: QueryFilter> QueryIter<'w, Q, F> {
    const EMPTY: ChunkInfo<Q, F> = ChunkInfo {
        fetch: Q::Fetch::DANGLING,
        len: 0,
        filter: F::EntityFilter::DANGLING,
    };

    /// Creates a new QueryIter
    #[inline]
    pub(crate) fn new(archetypes: &'w [Archetype]) -> Self {
        Self {
            archetypes,
            archetype_index: 0,
            chunk_info: Self::EMPTY,
            chunk_position: 0,
        }
    }
}

impl<'w, Q: WorldQuery, F: QueryFilter> Iterator for QueryIter<'w, Q, F> {
    type Item = <Q::Fetch as Fetch<'w>>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            loop {
                if self.chunk_position == self.chunk_info.len {
                    let archetype = self.archetypes.get(self.archetype_index)?;
                    self.archetype_index += 1;
                    self.chunk_position = 0;
                    self.chunk_info = Q::Fetch::get(archetype, 0)
                        .and_then(|fetch| {
                            Some(ChunkInfo {
                                fetch,
                                len: archetype.len(),
                                filter: F::get_entity_filter(archetype)?,
                            })
                        })
                        .unwrap_or(Self::EMPTY);
                    continue;
                }

                if !self
                    .chunk_info
                    .filter
                    .matches_entity(self.chunk_position as usize)
                {
                    self.chunk_position += 1;
                    continue;
                }

                let item = Some(self.chunk_info.fetch.fetch(self.chunk_position as usize));
                self.chunk_position += 1;
                return item;
            }
        }
    }
}

// if the Fetch is an UnfilteredFetch, then we can cheaply compute the length of the query by getting
// the length of each matching archetype
impl<'w, Q: WorldQuery> ExactSizeIterator for QueryIter<'w, Q, ()> {
    fn len(&self) -> usize {
        self.archetypes
            .iter()
            .filter(|&archetype| unsafe { Q::Fetch::get(archetype, 0).is_some() })
            .map(|x| x.len())
            .sum()
    }
}

struct ChunkIter<Q: WorldQuery, F: QueryFilter> {
    fetch: Q::Fetch,
    filter: F::EntityFilter,
    position: usize,
    len: usize,
}

impl<Q: WorldQuery, F: QueryFilter> ChunkIter<Q, F> {
    unsafe fn next<'a>(&mut self) -> Option<<Q::Fetch as Fetch<'a>>::Item> {
        loop {
            if self.position == self.len {
                return None;
            }

            if !self.filter.matches_entity(self.position as usize) {
                self.position += 1;
                continue;
            }

            let item = Some(self.fetch.fetch(self.position as usize));
            self.position += 1;
            return item;
        }
    }
}

/// Batched version of `QueryIter`
pub struct BatchedIter<'w, Q: WorldQuery, F: QueryFilter> {
    archetypes: &'w [Archetype],
    archetype_index: usize,
    batch_size: usize,
    batch: usize,
    _marker: PhantomData<(Q, F)>,
}

impl<'w, Q: WorldQuery, F: QueryFilter> BatchedIter<'w, Q, F> {
    pub(crate) fn new(archetypes: &'w [Archetype], batch_size: usize) -> Self {
        Self {
            archetypes,
            archetype_index: 0,
            batch_size,
            batch: 0,
            _marker: Default::default(),
        }
    }
}

unsafe impl<'w, Q: WorldQuery, F: QueryFilter> Send for BatchedIter<'w, Q, F> {}
unsafe impl<'w, Q: WorldQuery, F: QueryFilter> Sync for BatchedIter<'w, Q, F> {}

impl<'w, Q: WorldQuery, F: QueryFilter> Iterator for BatchedIter<'w, Q, F> {
    type Item = Batch<'w, Q, F>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let archetype = self.archetypes.get(self.archetype_index)?;
            let offset = self.batch_size * self.batch;
            if offset >= archetype.len() {
                self.archetype_index += 1;
                self.batch = 0;
                continue;
            }
            if let (Some(fetch), Some(filter)) = (
                unsafe { Q::Fetch::get(archetype, offset) },
                F::get_entity_filter(archetype),
            ) {
                self.batch += 1;
                return Some(Batch {
                    _marker: PhantomData,
                    state: ChunkIter {
                        fetch,
                        position: 0,
                        len: self.batch_size.min(archetype.len() - offset),
                        filter,
                    },
                });
            } else {
                self.archetype_index += 1;
                debug_assert_eq!(
                    self.batch, 0,
                    "query fetch should always reject at the first batch or not at all"
                );
                continue;
            }
        }
    }
}

/// A sequence of entities yielded by `BatchedIter`
pub struct Batch<'q, Q: WorldQuery, F: QueryFilter> {
    _marker: PhantomData<&'q ()>,
    state: ChunkIter<Q, F>,
}

impl<'q, 'w, Q: WorldQuery, F: QueryFilter> Iterator for Batch<'q, Q, F> {
    type Item = <Q::Fetch as Fetch<'q>>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let components = unsafe { self.state.next()? };
        Some(components)
    }
}

unsafe impl<'q, Q: WorldQuery, F: QueryFilter> Send for Batch<'q, Q, F> {}
unsafe impl<'q, Q: WorldQuery, F: QueryFilter> Sync for Batch<'q, Q, F> {}

macro_rules! tuple_impl {
    ($($name: ident),*) => {
        impl<'a, $($name: Fetch<'a>),*> Fetch<'a> for ($($name,)*) {
            type Item = ($($name::Item,)*);
            const DANGLING: Self = ($($name::DANGLING,)*);

            #[allow(unused_variables, unused_mut)]
            fn access() -> QueryAccess {
                QueryAccess::union(vec![
                    $($name::access(),)*
                ])
            }

            #[allow(unused_variables)]
            unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
                Some(($($name::get(archetype, offset)?,)*))
            }

            #[allow(unused_variables)]
            unsafe fn fetch(&self, n: usize) -> Self::Item {
                #[allow(non_snake_case)]
                let ($($name,)*) = self;
                ($($name.fetch(n),)*)
            }
        }

        impl<$($name: WorldQuery),*> WorldQuery for ($($name,)*) {
            type Fetch = ($($name::Fetch,)*);
        }

        unsafe impl<$($name: ReadOnlyFetch),*> ReadOnlyFetch for ($($name,)*) {}
    };
}

smaller_tuples_too!(tuple_impl, O, N, M, L, K, J, I, H, G, F, E, D, C, B, A);

#[cfg(test)]
mod tests {
    use crate::core::{Added, Changed, Component, Entity, Flags, Mutated, Or, QueryFilter, World};
    use std::{vec, vec::Vec};

    use super::Mut;

    struct A(usize);
    struct B(usize);
    struct C;

    #[test]
    fn added_queries() {
        let mut world = World::default();
        let e1 = world.spawn((A(0),));

        fn get_added<Com: Component>(world: &World) -> Vec<Entity> {
            world
                .query_filtered::<Entity, Added<Com>>()
                .collect::<Vec<Entity>>()
        }

        assert_eq!(get_added::<A>(&world), vec![e1]);
        world.insert(e1, (B(0),)).unwrap();
        assert_eq!(get_added::<A>(&world), vec![e1]);
        assert_eq!(get_added::<B>(&world), vec![e1]);

        world.clear_trackers();
        assert!(get_added::<A>(&world).is_empty());
        let e2 = world.spawn((A(1), B(1)));
        assert_eq!(get_added::<A>(&world), vec![e2]);
        assert_eq!(get_added::<B>(&world), vec![e2]);

        let added = world
            .query_filtered::<Entity, (Added<A>, Added<B>)>()
            .collect::<Vec<Entity>>();
        assert_eq!(added, vec![e2]);
    }

    #[test]
    fn mutated_trackers() {
        let mut world = World::default();
        let e1 = world.spawn((A(0), B(0)));
        let e2 = world.spawn((A(0), B(0)));
        let e3 = world.spawn((A(0), B(0)));
        world.spawn((A(0), B));

        for (i, mut a) in world.query_mut::<Mut<A>>().enumerate() {
            if i % 2 == 0 {
                a.0 += 1;
            }
        }

        fn get_filtered<F: QueryFilter>(world: &mut World) -> Vec<Entity> {
            world.query_filtered::<Entity, F>().collect::<Vec<Entity>>()
        }

        assert_eq!(get_filtered::<Mutated<A>>(&mut world), vec![e1, e3]);

        // ensure changing an entity's archetypes also moves its mutated state
        world.insert(e1, (C,)).unwrap();

        assert_eq!(get_filtered::<Mutated<A>>(&mut world), vec![e3, e1], "changed entities list should not change (although the order will due to archetype moves)");

        // spawning a new A entity should not change existing mutated state
        world.insert(e1, (A(0), B)).unwrap();
        assert_eq!(
            get_filtered::<Mutated<A>>(&mut world),
            vec![e3, e1],
            "changed entities list should not change"
        );

        // removing an unchanged entity should not change mutated state
        world.despawn(e2).unwrap();
        assert_eq!(
            get_filtered::<Mutated<A>>(&mut world),
            vec![e3, e1],
            "changed entities list should not change"
        );

        // removing a changed entity should remove it from enumeration
        world.despawn(e1).unwrap();
        assert_eq!(
            get_filtered::<Mutated<A>>(&mut world),
            vec![e3],
            "e1 should no longer be returned"
        );

        world.clear_trackers();

        assert!(get_filtered::<Mutated<A>>(&mut world).is_empty());

        let e4 = world.spawn(());

        world.insert_one(e4, A(0)).unwrap();
        assert!(get_filtered::<Mutated<A>>(&mut world).is_empty());
        assert_eq!(get_filtered::<Added<A>>(&mut world), vec![e4]);

        world.insert_one(e4, A(1)).unwrap();
        assert_eq!(get_filtered::<Mutated<A>>(&mut world), vec![e4]);

        world.clear_trackers();

        // ensure inserting multiple components set mutated state for
        // already existing components and set added state for
        // non existing components even when changing archetype.
        world.insert(e4, (A(0), B(0))).unwrap();

        assert!(get_filtered::<Added<A>>(&mut world).is_empty());
        assert_eq!(get_filtered::<Mutated<A>>(&mut world), vec![e4]);
        assert_eq!(get_filtered::<Added<B>>(&mut world), vec![e4]);
        assert!(get_filtered::<Mutated<B>>(&mut world).is_empty());
    }

    #[test]
    fn multiple_mutated_query() {
        let mut world = World::default();
        world.spawn((A(0), B(0)));
        let e2 = world.spawn((A(0), B(0)));
        world.spawn((A(0), B(0)));

        for mut a in world.query_mut::<Mut<A>>() {
            a.0 += 1;
        }

        for mut b in world.query_mut::<Mut<B>>().skip(1).take(1) {
            b.0 += 1;
        }

        let a_b_mutated = world
            .query_filtered_mut::<Entity, (Mutated<A>, Mutated<B>)>()
            .collect::<Vec<Entity>>();
        assert_eq!(a_b_mutated, vec![e2]);
    }

    #[test]
    fn or_mutated_query() {
        let mut world = World::default();
        let e1 = world.spawn((A(0), B(0)));
        let e2 = world.spawn((A(0), B(0)));
        let e3 = world.spawn((A(0), B(0)));
        let _e4 = world.spawn((A(0), B(0)));

        // Mutate A in entities e1 and e2
        for mut a in world.query_mut::<Mut<A>>().take(2) {
            a.0 += 1;
        }
        // Mutate B in entities e2 and e3
        for mut b in world.query_mut::<Mut<B>>().skip(1).take(2) {
            b.0 += 1;
        }

        let a_b_mutated = world
            .query_filtered_mut::<Entity, Or<(Mutated<A>, Mutated<B>)>>()
            .collect::<Vec<Entity>>();
        // e1 has mutated A, e3 has mutated B, e2 has mutated A and B, _e4 has no mutated component
        assert_eq!(a_b_mutated, vec![e1, e2, e3]);
    }

    #[test]
    fn changed_query() {
        let mut world = World::default();
        let e1 = world.spawn((A(0), B(0)));

        fn get_changed(world: &World) -> Vec<Entity> {
            world
                .query_filtered::<Entity, Changed<A>>()
                .collect::<Vec<Entity>>()
        }
        assert_eq!(get_changed(&world), vec![e1]);
        world.clear_trackers();
        assert_eq!(get_changed(&world), vec![]);
        *world.get_mut(e1).unwrap() = A(1);
        assert_eq!(get_changed(&world), vec![e1]);
    }

    #[test]
    fn flags_query() {
        let mut world = World::default();
        let e1 = world.spawn((A(0), B(0)));
        world.spawn((B(0),));

        fn get_flags(world: &World) -> Vec<Flags<A>> {
            world.query::<Flags<A>>().collect::<Vec<Flags<A>>>()
        }
        let flags = get_flags(&world);
        assert!(flags[0].with());
        assert!(flags[0].added());
        assert!(!flags[0].mutated());
        assert!(flags[0].changed());
        assert!(!flags[1].with());
        assert!(!flags[1].added());
        assert!(!flags[1].mutated());
        assert!(!flags[1].changed());
        world.clear_trackers();
        let flags = get_flags(&world);
        assert!(flags[0].with());
        assert!(!flags[0].added());
        assert!(!flags[0].mutated());
        assert!(!flags[0].changed());
        *world.get_mut(e1).unwrap() = A(1);
        let flags = get_flags(&world);
        assert!(flags[0].with());
        assert!(!flags[0].added());
        assert!(flags[0].mutated());
        assert!(flags[0].changed());
    }

    #[test]
    fn exact_size_query() {
        let mut world = World::default();
        world.spawn((A(0), B(0)));
        world.spawn((A(0), B(0)));
        world.spawn((C,));

        assert_eq!(world.query::<(&A, &B)>().len(), 2);
        // the following example shouldn't compile because Changed<A> is not an UnfilteredFetch
        // assert_eq!(world.query::<(Changed<A>, &B)>().len(), 2);
    }
}
