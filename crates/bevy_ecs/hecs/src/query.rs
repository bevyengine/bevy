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

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use std::vec;

use crate::{access::QueryAccess, archetype::Archetype, Component, Entity, MissingComponent};

/// A collection of component types to fetch from a `World`
pub trait Query {
    #[doc(hidden)]
    type Fetch: for<'a> Fetch<'a>;
}

/// A fetch that is read only. This should only be implemented for read-only fetches.
pub unsafe trait ReadOnlyFetch {}

/// A fetch that will always match every entity in an archetype (aka Fetch::should_skip always returns false)
pub trait UnfilteredFetch {}

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

    /// if this returns true, the nth item should be skipped during iteration
    ///
    /// # Safety
    /// shouldn't be called if there is no current item
    unsafe fn should_skip(&self, _n: usize) -> bool {
        false
    }

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
impl UnfilteredFetch for EntityFetch {}

impl Query for Entity {
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

impl<'a, T: Component> Query for &'a T {
    type Fetch = FetchRead<T>;
}

#[doc(hidden)]
pub struct FetchRead<T>(NonNull<T>);

unsafe impl<T> ReadOnlyFetch for FetchRead<T> {}
impl<T> UnfilteredFetch for FetchRead<T> {}

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

impl<'a, T: Component> Query for &'a mut T {
    type Fetch = FetchMut<T>;
}

impl<T: Query> Query for Option<T> {
    type Fetch = TryFetch<T::Fetch>;
}

/// Unique borrow of an entity's component
pub struct Mut<'a, T: Component> {
    pub(crate) value: &'a mut T,
    pub(crate) mutated: &'a mut bool,
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
            mutated: &mut *type_state.mutated().as_ptr().add(index),
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
        *self.mutated = true;
        self.value
    }
}

impl<'a, T: Component + core::fmt::Debug> core::fmt::Debug for Mut<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.value.fmt(f)
    }
}

impl<'a, T: Component> Query for Mut<'a, T> {
    type Fetch = FetchMut<T>;
}
#[doc(hidden)]
pub struct FetchMut<T>(NonNull<T>, NonNull<bool>);
impl<T> UnfilteredFetch for FetchMut<T> {}

impl<'a, T: Component> Fetch<'a> for FetchMut<T> {
    type Item = Mut<'a, T>;

    const DANGLING: Self = Self(NonNull::dangling(), NonNull::dangling());

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        archetype
            .get_with_type_state::<T>()
            .map(|(components, type_state)| {
                Self(
                    NonNull::new_unchecked(components.as_ptr().add(offset)),
                    NonNull::new_unchecked(type_state.mutated().as_ptr().add(offset)),
                )
            })
    }

    #[inline]
    unsafe fn fetch(&self, n: usize) -> Mut<'a, T> {
        Mut {
            value: &mut *self.0.as_ptr().add(n),
            mutated: &mut *self.1.as_ptr().add(n),
        }
    }

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::write::<T>()
    }
}

macro_rules! impl_or_query {
    ( $( $T:ident ),+ ) => {
        impl<$( $T: Query ),+> Query for Or<($( $T ),+)> {
            type Fetch = FetchOr<($( $T::Fetch ),+)>;
        }

        impl<'a, $( $T: Fetch<'a> ),+> Fetch<'a> for FetchOr<($( $T ),+)> {
            type Item = ($( $T::Item ),+);

            const DANGLING: Self = Self(($( $T::DANGLING ),+));

            fn access() -> QueryAccess {
                QueryAccess::union(vec![
                    $($T::access(),)+
                ])
            }


            unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
                Some(Self(( $( $T::get(archetype, offset)?),+ )))
            }

            #[allow(non_snake_case)]
            unsafe fn fetch(&self, n: usize) -> Self::Item {
                let ($( $T ),+) = &self.0;
                ($( $T.fetch(n) ),+)
            }

             #[allow(non_snake_case)]
            unsafe fn should_skip(&self, n: usize) -> bool {
                let ($( $T ),+) = &self.0;
                true $( && $T.should_skip(n) )+
            }
        }
    };
}

impl_or_query!(Q1, Q2);
impl_or_query!(Q1, Q2, Q3);
impl_or_query!(Q1, Q2, Q3, Q4);
impl_or_query!(Q1, Q2, Q3, Q4, Q5);
impl_or_query!(Q1, Q2, Q3, Q4, Q5, Q6);
impl_or_query!(Q1, Q2, Q3, Q4, Q5, Q6, Q7);
impl_or_query!(Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8);
impl_or_query!(Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9);
impl_or_query!(Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8, Q9, Q10);

/// Query transformer performing a logical or on a pair of queries
/// Intended to be used on Mutated or Changed queries.
/// # Example
/// ```
/// # use bevy_hecs::*;
/// let mut world = World::new();
/// world.spawn((123, true, 1., Some(1)));
/// world.spawn((456, false, 2., Some(0)));
/// for mut b in world.query_mut::<Mut<i32>>().skip(1).take(1) {
///     *b += 1;
/// }
/// let components = world
///     .query_mut::<Or<(Mutated<bool>, Mutated<i32>, Mutated<f64>, Mutated<Option<i32>>)>>()
///     .map(|(b, i, f, o)| (*b, *i))
///     .collect::<Vec<_>>();
/// assert_eq!(components, &[(false, 457)]);
/// ```
pub struct Or<T>(PhantomData<T>);
//pub struct Or<Q1, Q2, Q3>(PhantomData<(Q1, Q2, Q3)>);

#[doc(hidden)]
pub struct FetchOr<T>(T);

/// Query transformer that retrieves components of type `T` that have been mutated since the start of the frame.
/// Added components do not count as mutated.
pub struct Mutated<'a, T> {
    value: &'a T,
}

impl<'a, T: Component> Deref for Mutated<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T: Component> Query for Mutated<'a, T> {
    type Fetch = FetchMutated<T>;
}

#[doc(hidden)]
pub struct FetchMutated<T>(NonNull<T>, NonNull<bool>);

impl<'a, T: Component> Fetch<'a> for FetchMutated<T> {
    type Item = Mutated<'a, T>;

    const DANGLING: Self = Self(NonNull::dangling(), NonNull::dangling());

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        archetype
            .get_with_type_state::<T>()
            .map(|(components, type_state)| {
                Self(
                    NonNull::new_unchecked(components.as_ptr().add(offset)),
                    NonNull::new_unchecked(type_state.mutated().as_ptr().add(offset)),
                )
            })
    }

    unsafe fn should_skip(&self, n: usize) -> bool {
        // skip if the current item wasn't mutated
        !*self.1.as_ptr().add(n)
    }

    #[inline]
    unsafe fn fetch(&self, n: usize) -> Self::Item {
        Mutated {
            value: &*self.0.as_ptr().add(n),
        }
    }
}

/// Query transformer that retrieves components of type `T` that have been added since the start of the frame.
pub struct Added<'a, T> {
    value: &'a T,
}

impl<'a, T: Component> Deref for Added<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T: Component> Query for Added<'a, T> {
    type Fetch = FetchAdded<T>;
}

#[doc(hidden)]
pub struct FetchAdded<T>(NonNull<T>, NonNull<bool>);
unsafe impl<T> ReadOnlyFetch for FetchAdded<T> {}

impl<'a, T: Component> Fetch<'a> for FetchAdded<T> {
    type Item = Added<'a, T>;

    const DANGLING: Self = Self(NonNull::dangling(), NonNull::dangling());

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        archetype
            .get_with_type_state::<T>()
            .map(|(components, type_state)| {
                Self(
                    NonNull::new_unchecked(components.as_ptr().add(offset)),
                    NonNull::new_unchecked(type_state.added().as_ptr().add(offset)),
                )
            })
    }

    unsafe fn should_skip(&self, n: usize) -> bool {
        // skip if the current item wasn't added
        !*self.1.as_ptr().add(n)
    }

    #[inline]
    unsafe fn fetch(&self, n: usize) -> Self::Item {
        Added {
            value: &*self.0.as_ptr().add(n),
        }
    }
}

/// Query transformer that retrieves components of type `T` that have either been mutated or added since the start of the frame.
pub struct Changed<'a, T> {
    value: &'a T,
}

impl<'a, T: Component> Deref for Changed<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T: Component> Query for Changed<'a, T> {
    type Fetch = FetchChanged<T>;
}

#[doc(hidden)]
pub struct FetchChanged<T>(NonNull<T>, NonNull<bool>, NonNull<bool>);
unsafe impl<T> ReadOnlyFetch for FetchChanged<T> {}

impl<'a, T: Component> Fetch<'a> for FetchChanged<T> {
    type Item = Changed<'a, T>;

    const DANGLING: Self = Self(
        NonNull::dangling(),
        NonNull::dangling(),
        NonNull::dangling(),
    );

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::read::<T>()
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        archetype
            .get_with_type_state::<T>()
            .map(|(components, type_state)| {
                Self(
                    NonNull::new_unchecked(components.as_ptr().add(offset)),
                    NonNull::new_unchecked(type_state.added().as_ptr().add(offset)),
                    NonNull::new_unchecked(type_state.mutated().as_ptr().add(offset)),
                )
            })
    }

    unsafe fn should_skip(&self, n: usize) -> bool {
        // skip if the current item wasn't added or mutated
        !*self.1.as_ptr().add(n) && !*self.2.as_ptr().add(n)
    }

    #[inline]
    unsafe fn fetch(&self, n: usize) -> Self::Item {
        Changed {
            value: &*self.0.as_ptr().add(n),
        }
    }
}

#[doc(hidden)]
pub struct TryFetch<T>(Option<T>);
unsafe impl<T> ReadOnlyFetch for TryFetch<T> where T: ReadOnlyFetch {}
impl<T> UnfilteredFetch for TryFetch<T> where T: UnfilteredFetch {}

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

    unsafe fn should_skip(&self, n: usize) -> bool {
        self.0.as_ref().map_or(false, |fetch| fetch.should_skip(n))
    }
}

/// Query transformer skipping entities that have a `T` component
///
/// See also `QueryBorrow::without`.
///
/// # Example
/// ```
/// # use bevy_hecs::*;
/// let mut world = World::new();
/// let a = world.spawn((123, true, "abc"));
/// let b = world.spawn((456, false));
/// let c = world.spawn((42, "def"));
/// let entities = world.query::<Without<bool, (Entity, &i32)>>()
///     .map(|(e, &i)| (e, i))
///     .collect::<Vec<_>>();
/// assert_eq!(entities, &[(c, 42)]);
/// ```
pub struct Without<T, Q>(PhantomData<(Q, fn(T))>);

impl<T: Component, Q: Query> Query for Without<T, Q> {
    type Fetch = FetchWithout<T, Q::Fetch>;
}

#[doc(hidden)]
pub struct FetchWithout<T, F>(F, PhantomData<fn(T)>);
unsafe impl<'a, T: Component, F: Fetch<'a>> ReadOnlyFetch for FetchWithout<T, F> where
    F: ReadOnlyFetch
{
}
impl<T, F> UnfilteredFetch for FetchWithout<T, F> where F: UnfilteredFetch {}

impl<'a, T: Component, F: Fetch<'a>> Fetch<'a> for FetchWithout<T, F> {
    type Item = F::Item;

    const DANGLING: Self = Self(F::DANGLING, PhantomData);

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::without::<T>(F::access())
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        if archetype.has::<T>() {
            return None;
        }
        Some(Self(F::get(archetype, offset)?, PhantomData))
    }

    unsafe fn fetch(&self, n: usize) -> F::Item {
        self.0.fetch(n)
    }

    unsafe fn should_skip(&self, n: usize) -> bool {
        self.0.should_skip(n)
    }
}

/// Query transformer skipping entities that do not have a `T` component
///
/// See also `QueryBorrow::with`.
///
/// # Example
/// ```
/// # use bevy_hecs::*;
/// let mut world = World::new();
/// let a = world.spawn((123, true, "abc"));
/// let b = world.spawn((456, false));
/// let c = world.spawn((42, "def"));
/// let entities = world.query::<With<bool, (Entity, &i32)>>()
///     .map(|(e, &i)| (e, i))
///     .collect::<Vec<_>>();
/// assert_eq!(entities.len(), 2);
/// assert!(entities.contains(&(a, 123)));
/// assert!(entities.contains(&(b, 456)));
/// ```
pub struct With<T, Q>(PhantomData<(Q, fn(T))>);

impl<T: Component, Q: Query> Query for With<T, Q> {
    type Fetch = FetchWith<T, Q::Fetch>;
}

#[doc(hidden)]
pub struct FetchWith<T, F>(F, PhantomData<fn(T)>);
unsafe impl<'a, T: Component, F: Fetch<'a>> ReadOnlyFetch for FetchWith<T, F> where F: ReadOnlyFetch {}
impl<T, F> UnfilteredFetch for FetchWith<T, F> where F: UnfilteredFetch {}

impl<'a, T: Component, F: Fetch<'a>> Fetch<'a> for FetchWith<T, F> {
    type Item = F::Item;

    const DANGLING: Self = Self(F::DANGLING, PhantomData);

    #[inline]
    fn access() -> QueryAccess {
        QueryAccess::with::<T>(F::access())
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        if !archetype.has::<T>() {
            return None;
        }
        Some(Self(F::get(archetype, offset)?, PhantomData))
    }

    unsafe fn fetch(&self, n: usize) -> F::Item {
        self.0.fetch(n)
    }

    unsafe fn should_skip(&self, n: usize) -> bool {
        self.0.should_skip(n)
    }
}

struct ChunkInfo<Q: Query> {
    fetch: Q::Fetch,
    len: usize,
}

/// Iterator over the set of entities with the components in `Q`
pub struct QueryIter<'w, Q: Query> {
    archetypes: &'w [Archetype],
    archetype_index: usize,
    chunk_info: ChunkInfo<Q>,
    chunk_position: usize,
}

impl<'w, Q: Query> QueryIter<'w, Q> {
    // #[allow(clippy::declare_interior_mutable_const)] // no trait bounds on const fns
    // const EMPTY: Q::Fetch = Q::Fetch::DANGLING;
    const EMPTY: ChunkInfo<Q> = ChunkInfo {
        fetch: Q::Fetch::DANGLING,
        len: 0,
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

impl<'w, Q: Query> Iterator for QueryIter<'w, Q> {
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
                        .map(|fetch| ChunkInfo {
                            fetch,
                            len: archetype.len(),
                        })
                        .unwrap_or(Self::EMPTY);
                    continue;
                }

                if self
                    .chunk_info
                    .fetch
                    .should_skip(self.chunk_position as usize)
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
impl<'w, Q: Query> ExactSizeIterator for QueryIter<'w, Q>
where
    Q::Fetch: UnfilteredFetch,
{
    fn len(&self) -> usize {
        self.archetypes
            .iter()
            .filter(|&archetype| unsafe { Q::Fetch::get(archetype, 0).is_some() })
            .map(|x| x.len())
            .sum()
    }
}

struct ChunkIter<Q: Query> {
    fetch: Q::Fetch,
    position: usize,
    len: usize,
}

impl<Q: Query> ChunkIter<Q> {
    unsafe fn next<'a>(&mut self) -> Option<<Q::Fetch as Fetch<'a>>::Item> {
        loop {
            if self.position == self.len {
                return None;
            }

            if self.fetch.should_skip(self.position as usize) {
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
pub struct BatchedIter<'w, Q: Query> {
    archetypes: &'w [Archetype],
    archetype_index: usize,
    batch_size: usize,
    batch: usize,
    _marker: PhantomData<Q>,
}

impl<'w, Q: Query> BatchedIter<'w, Q> {
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

unsafe impl<'w, Q: Query> Send for BatchedIter<'w, Q> {}
unsafe impl<'w, Q: Query> Sync for BatchedIter<'w, Q> {}

impl<'w, Q: Query> Iterator for BatchedIter<'w, Q> {
    type Item = Batch<'w, Q>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let archetype = self.archetypes.get(self.archetype_index)?;
            let offset = self.batch_size * self.batch;
            if offset >= archetype.len() {
                self.archetype_index += 1;
                self.batch = 0;
                continue;
            }
            if let Some(fetch) = unsafe { Q::Fetch::get(archetype, offset) } {
                self.batch += 1;
                return Some(Batch {
                    _marker: PhantomData,
                    state: ChunkIter {
                        fetch,
                        position: 0,
                        len: self.batch_size.min(archetype.len() - offset),
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
pub struct Batch<'q, Q: Query> {
    _marker: PhantomData<&'q ()>,
    state: ChunkIter<Q>,
}

impl<'q, 'w, Q: Query> Iterator for Batch<'q, Q> {
    type Item = <Q::Fetch as Fetch<'q>>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let components = unsafe { self.state.next()? };
        Some(components)
    }
}

unsafe impl<'q, Q: Query> Send for Batch<'q, Q> {}
unsafe impl<'q, Q: Query> Sync for Batch<'q, Q> {}

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

            #[allow(unused_variables)]
            unsafe fn should_skip(&self, n: usize) -> bool {
                #[allow(non_snake_case)]
                let ($($name,)*) = self;
                $($name.should_skip(n)||)* false
            }
        }

        impl<$($name: Query),*> Query for ($($name,)*) {
            type Fetch = ($($name::Fetch,)*);
        }

        unsafe impl<$($name: ReadOnlyFetch),*> ReadOnlyFetch for ($($name,)*) {}
        impl<$($name: UnfilteredFetch),*> UnfilteredFetch for ($($name,)*) {}
    };
}

smaller_tuples_too!(tuple_impl, O, N, M, L, K, J, I, H, G, F, E, D, C, B, A);

#[cfg(test)]
mod tests {
    use crate::{Entity, Mut, Mutated, World};
    use std::{vec, vec::Vec};

    use super::*;

    struct A(usize);
    struct B(usize);
    struct C;

    #[test]
    fn added_queries() {
        let mut world = World::default();
        let e1 = world.spawn((A(0),));

        fn get_added<Com: Component>(world: &World) -> Vec<Entity> {
            world
                .query::<(Added<Com>, Entity)>()
                .map(|(_added, e)| e)
                .collect::<Vec<Entity>>()
        };

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
            .query::<(Entity, Added<A>, Added<B>)>()
            .map(|a| a.0)
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

        fn get_changed_a(world: &mut World) -> Vec<Entity> {
            world
                .query_mut::<(Mutated<A>, Entity)>()
                .map(|(_a, e)| e)
                .collect::<Vec<Entity>>()
        };

        assert_eq!(get_changed_a(&mut world), vec![e1, e3]);

        // ensure changing an entity's archetypes also moves its mutated state
        world.insert(e1, (C,)).unwrap();

        assert_eq!(get_changed_a(&mut world), vec![e3, e1], "changed entities list should not change (although the order will due to archetype moves)");

        // spawning a new A entity should not change existing mutated state
        world.insert(e1, (A(0), B)).unwrap();
        assert_eq!(
            get_changed_a(&mut world),
            vec![e3, e1],
            "changed entities list should not change"
        );

        // removing an unchanged entity should not change mutated state
        world.despawn(e2).unwrap();
        assert_eq!(
            get_changed_a(&mut world),
            vec![e3, e1],
            "changed entities list should not change"
        );

        // removing a changed entity should remove it from enumeration
        world.despawn(e1).unwrap();
        assert_eq!(
            get_changed_a(&mut world),
            vec![e3],
            "e1 should no longer be returned"
        );

        world.clear_trackers();

        assert!(world
            .query_mut::<(Mutated<A>, Entity)>()
            .map(|(_a, e)| e)
            .collect::<Vec<Entity>>()
            .is_empty());
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

        let a_b_changed = world
            .query_mut::<(Mutated<A>, Mutated<B>, Entity)>()
            .map(|(_a, _b, e)| e)
            .collect::<Vec<Entity>>();
        assert_eq!(a_b_changed, vec![e2]);
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

        let a_b_changed = world
            .query_mut::<(Or<(Mutated<A>, Mutated<B>)>, Entity)>()
            .map(|((_a, _b), e)| e)
            .collect::<Vec<Entity>>();
        // e1 has mutated A, e3 has mutated B, e2 has mutated A and B, _e4 has no mutated component
        assert_eq!(a_b_changed, vec![e1, e2, e3]);
    }

    #[test]
    fn changed_query() {
        let mut world = World::default();
        let e1 = world.spawn((A(0), B(0)));

        fn get_changed(world: &World) -> Vec<Entity> {
            world
                .query::<(Changed<A>, Entity)>()
                .map(|(_a, e)| e)
                .collect::<Vec<Entity>>()
        };
        assert_eq!(get_changed(&world), vec![e1]);
        world.clear_trackers();
        assert_eq!(get_changed(&world), vec![]);
        *world.get_mut(e1).unwrap() = A(1);
        assert_eq!(get_changed(&world), vec![e1]);
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
