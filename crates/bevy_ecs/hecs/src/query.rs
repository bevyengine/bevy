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

use crate::{archetype::Archetype, Component, Entity, MissingComponent};

/// A collection of component types to fetch from a `World`
pub trait Query {
    #[doc(hidden)]
    type Fetch: for<'a> Fetch<'a>;
}

/// A fetch that is read only. This should only be implemented for read-only fetches.
pub unsafe trait ReadOnlyFetch {}

/// Streaming iterators over contiguous homogeneous ranges of components
pub trait Fetch<'a>: Sized {
    /// Type of value to be fetched
    type Item;

    /// How this query will access `archetype`, if at all
    fn access(archetype: &Archetype) -> Option<Access>;

    /// Acquire dynamic borrows from `archetype`
    fn borrow(archetype: &Archetype);
    /// Construct a `Fetch` for `archetype` if it should be traversed
    ///
    /// # Safety
    /// `offset` must be in bounds of `archetype`
    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self>;
    /// Release dynamic borrows acquired by `borrow`
    fn release(archetype: &Archetype);

    /// if this returns true, the current item will be skipped during iteration
    ///
    /// # Safety
    /// shouldn't be called if there is no current item
    unsafe fn should_skip(&self) -> bool {
        false
    }

    /// Access the next item in this archetype without bounds checking
    ///
    /// # Safety
    /// - Must only be called after `borrow`
    /// - `release` must not be called while `'a` is still live
    /// - Bounds-checking must be performed externally
    /// - Any resulting borrows must be legal (e.g. no &mut to something another iterator might access)
    unsafe fn next(&mut self) -> Self::Item;
}

/// Type of access a `Query` may have to an `Archetype`
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Access {
    /// Read entity IDs only, no components
    Iterate,
    /// Read components
    Read,
    /// Read and write components
    Write,
}

#[derive(Copy, Clone, Debug)]
pub struct EntityFetch(NonNull<Entity>);
unsafe impl ReadOnlyFetch for EntityFetch {}

impl Query for Entity {
    type Fetch = EntityFetch;
}

impl<'a> Fetch<'a> for EntityFetch {
    type Item = Entity;

    #[inline]
    fn access(_archetype: &Archetype) -> Option<Access> {
        Some(Access::Iterate)
    }

    #[inline]
    fn borrow(_archetype: &Archetype) {}

    #[inline]
    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        Some(EntityFetch(NonNull::new_unchecked(
            archetype.entities().as_ptr().add(offset),
        )))
    }

    #[inline]
    fn release(_archetype: &Archetype) {}

    #[inline]
    unsafe fn next(&mut self) -> Self::Item {
        let id = self.0.as_ptr();
        self.0 = NonNull::new_unchecked(id.add(1));
        *id
    }
}

impl<'a, T: Component> Query for &'a T {
    type Fetch = FetchRead<T>;
}

#[doc(hidden)]
pub struct FetchRead<T>(NonNull<T>);

unsafe impl<T> ReadOnlyFetch for FetchRead<T> {}

impl<'a, T: Component> Fetch<'a> for FetchRead<T> {
    type Item = &'a T;

    fn access(archetype: &Archetype) -> Option<Access> {
        if archetype.has::<T>() {
            Some(Access::Read)
        } else {
            None
        }
    }

    fn borrow(archetype: &Archetype) {
        archetype.borrow::<T>();
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        archetype
            .get::<T>()
            .map(|x| Self(NonNull::new_unchecked(x.as_ptr().add(offset))))
    }

    fn release(archetype: &Archetype) {
        archetype.release::<T>();
    }

    #[inline]
    unsafe fn next(&mut self) -> &'a T {
        let x = self.0.as_ptr();
        self.0 = NonNull::new_unchecked(x.add(1));
        &*x
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

impl<'a, T: Component> Fetch<'a> for FetchMut<T> {
    type Item = Mut<'a, T>;

    fn access(archetype: &Archetype) -> Option<Access> {
        if archetype.has::<T>() {
            Some(Access::Write)
        } else {
            None
        }
    }

    fn borrow(archetype: &Archetype) {
        archetype.borrow_mut::<T>();
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

    fn release(archetype: &Archetype) {
        archetype.release_mut::<T>();
    }

    #[inline]
    unsafe fn next(&mut self) -> Mut<'a, T> {
        let component = self.0.as_ptr();
        let mutated = self.1.as_ptr();
        self.0 = NonNull::new_unchecked(component.add(1));
        self.1 = NonNull::new_unchecked(mutated.add(1));
        Mut {
            value: &mut *component,
            mutated: &mut *mutated,
        }
    }
}

macro_rules! impl_or_query {
    ( $( $T:ident ),+ ) => {
        impl<$( $T: Query ),+> Query for Or<($( $T ),+)> {
            type Fetch = FetchOr<($( $T::Fetch ),+)>;
        }

        impl<'a, $( $T: Fetch<'a> ),+> Fetch<'a> for FetchOr<($( $T ),+)> {
            type Item = ($( $T::Item ),+);

            fn access(archetype: &Archetype) -> Option<Access> {
                let mut max_access = None;
                $(
                max_access = max_access.max($T::access(archetype));
                )+
                max_access
            }

            fn borrow(archetype: &Archetype) {
                $(
                    $T::borrow(archetype);
                 )+
            }

            unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
                Some(Self(( $( $T::get(archetype, offset)?),+ )))
            }

            fn release(archetype: &Archetype) {
                $(
                    $T::release(archetype);
                 )+
            }

            #[allow(non_snake_case)]
            unsafe fn next(&mut self) -> Self::Item {
                let ($( $T ),+) = &mut self.0;
                ($( $T.next() ),+)
            }

             #[allow(non_snake_case)]
            unsafe fn should_skip(&self) -> bool {
                let ($( $T ),+) = &self.0;
                true $( && $T.should_skip() )+
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
/// for mut b in world.query_mut::<Mut<i32>>().iter().skip(1).take(1) {
///     *b += 1;
/// }
/// let components = world
///     .query_mut::<Or<(Mutated<bool>, Mutated<i32>, Mutated<f64>, Mutated<Option<i32>>)>>()
///     .iter()
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

    fn access(archetype: &Archetype) -> Option<Access> {
        if archetype.has::<T>() {
            Some(Access::Read)
        } else {
            None
        }
    }

    fn borrow(archetype: &Archetype) {
        archetype.borrow::<T>();
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

    fn release(archetype: &Archetype) {
        archetype.release::<T>();
    }

    unsafe fn should_skip(&self) -> bool {
        // skip if the current item wasn't mutated
        !*self.1.as_ref()
    }

    #[inline]
    unsafe fn next(&mut self) -> Self::Item {
        self.1 = NonNull::new_unchecked(self.1.as_ptr().add(1));
        let value = self.0.as_ptr();
        self.0 = NonNull::new_unchecked(value.add(1));
        Mutated { value: &*value }
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

    fn access(archetype: &Archetype) -> Option<Access> {
        if archetype.has::<T>() {
            Some(Access::Read)
        } else {
            None
        }
    }

    fn borrow(archetype: &Archetype) {
        archetype.borrow::<T>();
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

    fn release(archetype: &Archetype) {
        archetype.release::<T>();
    }

    unsafe fn should_skip(&self) -> bool {
        // skip if the current item wasn't added
        !*self.1.as_ref()
    }

    #[inline]
    unsafe fn next(&mut self) -> Self::Item {
        self.1 = NonNull::new_unchecked(self.1.as_ptr().add(1));
        let value = self.0.as_ptr();
        self.0 = NonNull::new_unchecked(value.add(1));
        Added { value: &*value }
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

    fn access(archetype: &Archetype) -> Option<Access> {
        if archetype.has::<T>() {
            Some(Access::Read)
        } else {
            None
        }
    }

    fn borrow(archetype: &Archetype) {
        archetype.borrow::<T>();
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

    fn release(archetype: &Archetype) {
        archetype.release::<T>();
    }

    unsafe fn should_skip(&self) -> bool {
        // skip if the current item wasn't added or mutated
        !*self.1.as_ref() && !self.2.as_ref()
    }

    #[inline]
    unsafe fn next(&mut self) -> Self::Item {
        self.1 = NonNull::new_unchecked(self.1.as_ptr().add(1));
        self.2 = NonNull::new_unchecked(self.2.as_ptr().add(1));
        let value = self.0.as_ptr();
        self.0 = NonNull::new_unchecked(value.add(1));
        Changed { value: &*value }
    }
}

#[doc(hidden)]
pub struct TryFetch<T>(Option<T>);
unsafe impl<T> ReadOnlyFetch for TryFetch<T> where T: ReadOnlyFetch {}

impl<'a, T: Fetch<'a>> Fetch<'a> for TryFetch<T> {
    type Item = Option<T::Item>;

    fn access(archetype: &Archetype) -> Option<Access> {
        Some(T::access(archetype).unwrap_or(Access::Iterate))
    }

    fn borrow(archetype: &Archetype) {
        T::borrow(archetype)
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        Some(Self(T::get(archetype, offset)))
    }

    fn release(archetype: &Archetype) {
        T::release(archetype)
    }

    unsafe fn next(&mut self) -> Option<T::Item> {
        Some(self.0.as_mut()?.next())
    }

    unsafe fn should_skip(&self) -> bool {
        self.0.as_ref().map_or(false, |fetch| fetch.should_skip())
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
///     .iter()
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

impl<'a, T: Component, F: Fetch<'a>> Fetch<'a> for FetchWithout<T, F> {
    type Item = F::Item;

    fn access(archetype: &Archetype) -> Option<Access> {
        if archetype.has::<T>() {
            None
        } else {
            F::access(archetype)
        }
    }

    fn borrow(archetype: &Archetype) {
        F::borrow(archetype)
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        if archetype.has::<T>() {
            return None;
        }
        Some(Self(F::get(archetype, offset)?, PhantomData))
    }

    fn release(archetype: &Archetype) {
        F::release(archetype)
    }

    unsafe fn next(&mut self) -> F::Item {
        self.0.next()
    }

    unsafe fn should_skip(&self) -> bool {
        self.0.should_skip()
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
///     .iter()
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

impl<'a, T: Component, F: Fetch<'a>> Fetch<'a> for FetchWith<T, F> {
    type Item = F::Item;

    fn access(archetype: &Archetype) -> Option<Access> {
        if archetype.has::<T>() {
            F::access(archetype)
        } else {
            None
        }
    }

    fn borrow(archetype: &Archetype) {
        F::borrow(archetype)
    }

    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        if !archetype.has::<T>() {
            return None;
        }
        Some(Self(F::get(archetype, offset)?, PhantomData))
    }

    fn release(archetype: &Archetype) {
        F::release(archetype)
    }

    unsafe fn next(&mut self) -> F::Item {
        self.0.next()
    }

    unsafe fn should_skip(&self) -> bool {
        self.0.should_skip()
    }
}

/// A borrow of a `World` sufficient to execute the query `Q`
///
/// Note that borrows are not released until this object is dropped.
pub struct QueryBorrow<'w, Q: Query> {
    archetypes: &'w [Archetype],
    borrowed: bool,
    _marker: PhantomData<Q>,
}

impl<'w, Q: Query> QueryBorrow<'w, Q> {
    pub(crate) fn new(archetypes: &'w [Archetype]) -> Self {
        Self {
            archetypes,
            borrowed: false,
            _marker: PhantomData,
        }
    }

    /// Execute the query
    ///
    /// Must be called only once per query.
    pub fn iter<'q>(&'q mut self) -> QueryIter<'q, 'w, Q> {
        self.borrow();
        QueryIter {
            borrow: self,
            archetype_index: 0,
            iter: None,
        }
    }

    /// Like `iter`, but returns child iterators of at most `batch_size` elements
    ///
    /// Useful for distributing work over a threadpool.
    pub fn iter_batched<'q>(&'q mut self, batch_size: usize) -> BatchedIter<'q, 'w, Q> {
        self.borrow();
        BatchedIter {
            borrow: self,
            archetype_index: 0,
            batch_size,
            batch: 0,
        }
    }

    fn borrow(&mut self) {
        if self.borrowed {
            panic!(
                "called QueryBorrow::iter twice on the same borrow; construct a new query instead"
            );
        }

        self.borrowed = true;
    }

    /// Transform the query into one that requires a certain component without borrowing it
    ///
    /// This can be useful when the component needs to be borrowed elsewhere and it isn't necessary
    /// for the iterator to expose its data directly.
    ///
    /// Equivalent to using a query type wrapped in `With`.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let a = world.spawn((123, true, "abc"));
    /// let b = world.spawn((456, false));
    /// let c = world.spawn((42, "def"));
    /// let entities = world.query::<(Entity, &i32)>()
    ///     .with::<bool>()
    ///     .iter()
    ///     .map(|(e, &i)| (e, i)) // Copy out of the world
    ///     .collect::<Vec<_>>();
    /// assert!(entities.contains(&(a, 123)));
    /// assert!(entities.contains(&(b, 456)));
    /// ```
    pub fn with<T: Component>(self) -> QueryBorrow<'w, With<T, Q>> {
        self.transform()
    }

    /// Transform the query into one that skips entities having a certain component
    ///
    /// Equivalent to using a query type wrapped in `Without`.
    ///
    /// # Example
    /// ```
    /// # use bevy_hecs::*;
    /// let mut world = World::new();
    /// let a = world.spawn((123, true, "abc"));
    /// let b = world.spawn((456, false));
    /// let c = world.spawn((42, "def"));
    /// let entities = world.query::<(Entity, &i32)>()
    ///     .without::<bool>()
    ///     .iter()
    ///     .map(|(e, &i)| (e, i)) // Copy out of the world
    ///     .collect::<Vec<_>>();
    /// assert_eq!(entities, &[(c, 42)]);
    /// ```
    pub fn without<T: Component>(self) -> QueryBorrow<'w, Without<T, Q>> {
        self.transform()
    }

    /// Helper to change the type of the query
    fn transform<R: Query>(mut self) -> QueryBorrow<'w, R> {
        let borrow = QueryBorrow {
            archetypes: self.archetypes,
            borrowed: self.borrowed,
            _marker: PhantomData,
        };

        self.borrowed = false;
        borrow
    }
}

unsafe impl<'w, Q: Query> Send for QueryBorrow<'w, Q> {}
unsafe impl<'w, Q: Query> Sync for QueryBorrow<'w, Q> {}

impl<'q, 'w, Q: Query> IntoIterator for &'q mut QueryBorrow<'w, Q> {
    type IntoIter = QueryIter<'q, 'w, Q>;
    type Item = <Q::Fetch as Fetch<'q>>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over the set of entities with the components in `Q`
pub struct QueryIter<'q, 'w, Q: Query> {
    borrow: &'q mut QueryBorrow<'w, Q>,
    archetype_index: usize,
    iter: Option<ChunkIter<Q>>,
}

unsafe impl<'q, 'w, Q: Query> Send for QueryIter<'q, 'w, Q> {}
unsafe impl<'q, 'w, Q: Query> Sync for QueryIter<'q, 'w, Q> {}

impl<'q, 'w, Q: Query> Iterator for QueryIter<'q, 'w, Q> {
    type Item = <Q::Fetch as Fetch<'q>>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter {
                None => {
                    let archetype = self.borrow.archetypes.get(self.archetype_index)?;
                    self.archetype_index += 1;
                    unsafe {
                        self.iter = Q::Fetch::get(archetype, 0).map(|fetch| ChunkIter {
                            fetch,
                            len: archetype.len(),
                        });
                    }
                }
                Some(ref mut iter) => match unsafe { iter.next() } {
                    None => {
                        self.iter = None;
                        continue;
                    }
                    Some(components) => {
                        return Some(components);
                    }
                },
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len();
        (n, Some(n))
    }
}

impl<'q, 'w, Q: Query> ExactSizeIterator for QueryIter<'q, 'w, Q> {
    fn len(&self) -> usize {
        self.borrow
            .archetypes
            .iter()
            .filter(|&x| Q::Fetch::access(x).is_some())
            .map(|x| x.len())
            .sum()
    }
}

struct ChunkIter<Q: Query> {
    fetch: Q::Fetch,
    len: usize,
}

impl<Q: Query> ChunkIter<Q> {
    unsafe fn next<'a>(&mut self) -> Option<<Q::Fetch as Fetch<'a>>::Item> {
        loop {
            if self.len == 0 {
                return None;
            }

            self.len -= 1;
            if self.fetch.should_skip() {
                // we still need to progress the iterator
                let _ = self.fetch.next();
                continue;
            }

            break Some(self.fetch.next());
        }
    }
}

/// Batched version of `QueryIter`
pub struct BatchedIter<'q, 'w, Q: Query> {
    borrow: &'q mut QueryBorrow<'w, Q>,
    archetype_index: usize,
    batch_size: usize,
    batch: usize,
}

unsafe impl<'q, 'w, Q: Query> Send for BatchedIter<'q, 'w, Q> {}
unsafe impl<'q, 'w, Q: Query> Sync for BatchedIter<'q, 'w, Q> {}

impl<'q, 'w, Q: Query> Iterator for BatchedIter<'q, 'w, Q> {
    type Item = Batch<'q, Q>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let archetype = self.borrow.archetypes.get(self.archetype_index)?;
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

            #[allow(unused_variables, unused_mut)]
            fn access(archetype: &Archetype) -> Option<Access> {
                let mut access = Access::Iterate;
                $(
                    access = access.max($name::access(archetype)?);
                )*
                Some(access)
            }

            #[allow(unused_variables)]
            fn borrow(archetype: &Archetype) {
                $($name::borrow(archetype);)*
            }
            #[allow(unused_variables)]
            unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
                Some(($($name::get(archetype, offset)?,)*))
            }
            #[allow(unused_variables)]
            fn release(archetype: &Archetype) {
                $($name::release(archetype);)*
            }

            #[allow(unused_variables)]
            unsafe fn next(&mut self) -> Self::Item {
                #[allow(non_snake_case)]
                let ($($name,)*) = self;
                ($($name.next(),)*)
            }

            unsafe fn should_skip(&self) -> bool {
                #[allow(non_snake_case)]
                let ($($name,)*) = self;
                $($name.should_skip()||)* false
            }
        }

        impl<$($name: Query),*> Query for ($($name,)*) {
            type Fetch = ($($name::Fetch,)*);
        }

        unsafe impl<$($name: ReadOnlyFetch),*> ReadOnlyFetch for ($($name,)*) {}
    };
}

smaller_tuples_too!(tuple_impl, O, N, M, L, K, J, I, H, G, F, E, D, C, B, A);

#[cfg(test)]
mod tests {
    use crate::{Entity, Mut, Mutated, World};
    use std::{vec, vec::Vec};

    use super::*;

    #[test]
    fn access_order() {
        assert!(Access::Write > Access::Read);
        assert!(Access::Read > Access::Iterate);
        assert!(Some(Access::Iterate) > None);
    }

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
                .iter()
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
            .iter()
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

        for (i, mut a) in world.query_mut::<Mut<A>>().iter().enumerate() {
            if i % 2 == 0 {
                a.0 += 1;
            }
        }

        fn get_changed_a(world: &mut World) -> Vec<Entity> {
            world
                .query_mut::<(Mutated<A>, Entity)>()
                .iter()
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
            .iter()
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

        for mut a in world.query_mut::<Mut<A>>().iter() {
            a.0 += 1;
        }

        for mut b in world.query_mut::<Mut<B>>().iter().skip(1).take(1) {
            b.0 += 1;
        }

        let a_b_changed = world
            .query_mut::<(Mutated<A>, Mutated<B>, Entity)>()
            .iter()
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
        for mut a in world.query_mut::<Mut<A>>().iter().take(2) {
            a.0 += 1;
        }
        // Mutate B in entities e2 and e3
        for mut b in world.query_mut::<Mut<B>>().iter().skip(1).take(2) {
            b.0 += 1;
        }

        let a_b_changed = world
            .query_mut::<(Or<(Mutated<A>, Mutated<B>)>, Entity)>()
            .iter()
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
                .iter()
                .map(|(_a, e)| e)
                .collect::<Vec<Entity>>()
        };
        assert_eq!(get_changed(&world), vec![e1]);
        world.clear_trackers();
        assert_eq!(get_changed(&world), vec![]);
        *world.get_mut(e1).unwrap() = A(1);
        assert_eq!(get_changed(&world), vec![e1]);
    }
}
