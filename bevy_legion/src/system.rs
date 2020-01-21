use crate::borrow::{AtomicRefCell, Ref, RefMut};
use crate::command::CommandBuffer;
use crate::cons::{ConsAppend, ConsFlatten};
use crate::entity::Entity;
use crate::filter::EntityFilter;
use crate::query::ReadOnly;
use crate::query::{ChunkDataIter, ChunkEntityIter, ChunkViewIter, Query, Read, View, Write};
use crate::resource::{Resource, ResourceSet, ResourceTypeId};
use crate::schedule::ArchetypeAccess;
use crate::schedule::{Runnable, Schedulable};
use crate::storage::Tag;
use crate::storage::{Component, ComponentTypeId, TagTypeId};
use crate::world::World;
use bit_set::BitSet;
use derivative::Derivative;
use std::any::TypeId;
use std::borrow::Cow;
use std::marker::PhantomData;
use tracing::{debug, info, span, Level};

#[cfg(feature = "par-iter")]
use crate::filter::{ArchetypeFilterData, ChunkFilterData, ChunksetFilterData, Filter};

#[cfg(feature = "par-iter")]
use crate::iterator::FissileIterator;

#[cfg(feature = "par-iter")]
use crate::query::Chunk;

/// Structure used by `SystemAccess` for describing access to the provided `T`
#[derive(Derivative, Debug, Clone)]
#[derivative(Default(bound = ""))]
pub struct Access<T> {
    reads: Vec<T>,
    writes: Vec<T>,
}

/// Structure describing the resource and component access conditions of the system.
#[derive(Derivative, Debug, Clone)]
#[derivative(Default(bound = ""))]
pub struct SystemAccess {
    pub resources: Access<ResourceTypeId>,
    pub components: Access<ComponentTypeId>,
    pub tags: Access<TagTypeId>,
}

// FIXME: This would have an associated lifetime and would hold references instead of pointers,
// but this is a workaround for lack of GATs and bugs around HRTBs combined with associated types.
// See https://github.com/rust-lang/rust/issues/62529

#[derive(Clone)]
enum QueryRef<V: for<'v> View<'v>, F: EntityFilter> {
    Ptr(*const Query<V, F>),
    Owned(Query<V, F>),
}

impl<V: for<'v> View<'v>, F: EntityFilter> QueryRef<V, F> {
    unsafe fn get(&self) -> &Query<V, F> {
        match self {
            QueryRef::Ptr(ptr) => &**ptr,
            QueryRef::Owned(query) => query,
        }
    }

    unsafe fn into_owned_query(self) -> Query<V, F> {
        match self {
            QueryRef::Ptr(ptr) => (*ptr).clone(),
            QueryRef::Owned(query) => query,
        }
    }
}

// * implement QuerySet for tuples of queries
// * likely actually wrapped in another struct, to cache the archetype sets for each query
// * prepared queries will each re-use the archetype set results in their iterators so
// that the archetype filters don't need to be run again - can also cache this between runs
// and only append new archetype matches each frame
// * per-query archetype matches stored as simple Vec<usize> - filter_archetypes() updates them and writes
// the union of all queries into the BitSet provided, to be used to schedule the system as a whole

/// A query that is usable from within a system.
#[derive(Clone)]
pub struct SystemQuery<V, F>
where
    V: for<'v> View<'v>,
    F: EntityFilter,
{
    query: QueryRef<V, F>,
}

// # Safety
// `SystemQuery` does not auto-implement `Send` because it contains a `*const Query`.
// It is safe to implement `Send` because no mutable state is shared between instances.
unsafe impl<V, F> Send for SystemQuery<V, F>
where
    V: for<'v> View<'v>,
    F: EntityFilter,
{
}

// # Safety
// `SystemQuery` does not auto-implement `Sync` because it contains a `*const Query`.
// It is safe to implement `Sync` because no internal mutation occurs behind a safe `&self`.
unsafe impl<V, F> Sync for SystemQuery<V, F>
where
    V: for<'v> View<'v>,
    F: EntityFilter,
{
}

impl<V, F> SystemQuery<V, F>
where
    V: for<'v> View<'v>,
    F: EntityFilter,
{
    /// Safety: input references might not outlive a created instance of `SystemQuery`.
    unsafe fn new(query: &Query<V, F>) -> Self {
        SystemQuery {
            query: QueryRef::Ptr(query as *const Query<V, F>),
        }
    }

    /// Adds an additional filter to the query.
    pub fn filter<T: EntityFilter>(
        self,
        filter: T,
    ) -> SystemQuery<V, <F as std::ops::BitAnd<T>>::Output>
    where
        F: std::ops::BitAnd<T>,
        <F as std::ops::BitAnd<T>>::Output: EntityFilter,
    {
        let query = unsafe { self.query.into_owned_query() }.filter(filter);
        SystemQuery {
            query: QueryRef::Owned(query),
        }
    }

    // These methods are not unsafe, because we guarantee that `SystemQuery` lifetime is never actually
    // in user's hands and access to internal pointers is impossible. There is no way to move the object out
    // of mutable reference through public API, because there is no way to get access to more than a single instance at a time.
    // The unsafety is an implementation detail. It can be fully safe once GATs are in the language.

    /// Gets an iterator which iterates through all chunks that match the query.
    /// Does not perform static borrow checking.
    ///
    /// # Safety
    ///
    /// Incorrectly accessing components that are already borrowed elsewhere is undefined behavior.
    ///
    /// # Panics
    ///
    /// This function may panic if other code is concurrently accessing the same components.
    #[inline]
    pub unsafe fn iter_chunks_unchecked<'a, 'b>(
        &'b self,
        world: &SubWorld,
    ) -> ChunkViewIter<'a, 'b, V, F::ArchetypeFilter, F::ChunksetFilter, F::ChunkFilter> {
        self.query.get().iter_chunks_unchecked(&*world.world)
    }

    /// Gets an iterator which iterates through all chunks that match the query.
    #[inline]
    pub fn iter_chunks<'a, 'b>(
        &'b self,
        world: &SubWorld,
    ) -> ChunkViewIter<'a, 'b, V, F::ArchetypeFilter, F::ChunksetFilter, F::ChunkFilter>
    where
        V: ReadOnly,
    {
        // safe because the view can only read data immutably
        unsafe { self.iter_chunks_unchecked(world) }
    }

    /// Gets an iterator which iterates through all chunks that match the query.
    #[inline]
    pub fn iter_chunks_mut<'a, 'b>(
        &'b self,
        world: &mut SubWorld,
    ) -> ChunkViewIter<'a, 'b, V, F::ArchetypeFilter, F::ChunksetFilter, F::ChunkFilter> {
        // safe because the &mut SubWorld ensures exclusivity
        unsafe { self.iter_chunks_unchecked(world) }
    }

    /// Gets an iterator which iterates through all entity data that matches the query, and also yields the the `Entity` IDs.
    /// Does not perform static borrow checking.
    ///
    /// # Safety
    ///
    /// Incorrectly accessing components that are already borrowed elsewhere is undefined behavior.
    ///
    /// # Panics
    ///
    /// This function may panic if other code is concurrently accessing the same components.
    #[inline]
    pub unsafe fn iter_entities_unchecked<'a, 'b>(
        &'b self,
        world: &SubWorld,
    ) -> ChunkEntityIter<
        'a,
        V,
        ChunkViewIter<'a, 'b, V, F::ArchetypeFilter, F::ChunksetFilter, F::ChunkFilter>,
    > {
        self.query.get().iter_entities_unchecked(&*world.world)
    }

    /// Gets an iterator which iterates through all entity data that matches the query, and also yields the the `Entity` IDs.
    #[inline]
    pub fn iter_entities<'a, 'b>(
        &'b self,
        world: &SubWorld,
    ) -> ChunkEntityIter<
        'a,
        V,
        ChunkViewIter<'a, 'b, V, F::ArchetypeFilter, F::ChunksetFilter, F::ChunkFilter>,
    >
    where
        V: ReadOnly,
    {
        // safe because the view can only read data immutably
        unsafe { self.iter_entities_unchecked(world) }
    }

    /// Gets an iterator which iterates through all entity data that matches the query, and also yields the the `Entity` IDs.
    #[inline]
    pub fn iter_entities_mut<'a, 'b>(
        &'b self,
        world: &mut SubWorld,
    ) -> ChunkEntityIter<
        'a,
        V,
        ChunkViewIter<'a, 'b, V, F::ArchetypeFilter, F::ChunksetFilter, F::ChunkFilter>,
    > {
        // safe because the &mut SubWorld ensures exclusivity
        unsafe { self.iter_entities_unchecked(world) }
    }

    /// Gets an iterator which iterates through all entity data that matches the query.
    /// Does not perform static borrow checking.
    ///
    /// # Safety
    ///
    /// Incorrectly accessing components that are already borrowed elsewhere is undefined behavior.
    ///
    /// # Panics
    ///
    /// This function may panic if other code is concurrently accessing the same components.
    #[inline]
    pub unsafe fn iter_unchecked<'a, 'data>(
        &'a self,
        world: &SubWorld,
    ) -> ChunkDataIter<
        'data,
        V,
        ChunkViewIter<'data, 'a, V, F::ArchetypeFilter, F::ChunksetFilter, F::ChunkFilter>,
    > {
        self.query.get().iter_unchecked(&*world.world)
    }

    /// Gets an iterator which iterates through all entity data that matches the query.
    #[inline]
    pub fn iter<'a, 'data>(
        &'a self,
        world: &SubWorld,
    ) -> ChunkDataIter<
        'data,
        V,
        ChunkViewIter<'data, 'a, V, F::ArchetypeFilter, F::ChunksetFilter, F::ChunkFilter>,
    >
    where
        V: ReadOnly,
    {
        // safe because the view can only read data immutably
        unsafe { self.iter_unchecked(world) }
    }

    /// Gets an iterator which iterates through all entity data that matches the query.
    #[inline]
    pub fn iter_mut<'a, 'data>(
        &'a self,
        world: &mut SubWorld,
    ) -> ChunkDataIter<
        'data,
        V,
        ChunkViewIter<'data, 'a, V, F::ArchetypeFilter, F::ChunksetFilter, F::ChunkFilter>,
    > {
        // safe because the &mut SubWorld ensures exclusivity
        unsafe { self.iter_unchecked(world) }
    }

    /// Iterates through all entity data that matches the query.
    /// Does not perform static borrow checking.
    ///
    /// # Safety
    ///
    /// Incorrectly accessing components that are already borrowed elsewhere is undefined behavior.
    ///
    /// # Panics
    ///
    /// This function may panic if other code is concurrently accessing the same components.
    #[inline]
    pub unsafe fn for_each_unchecked<'a, 'data, T>(&'a self, world: &SubWorld, f: T)
    where
        T: Fn(<<V as View<'data>>::Iter as Iterator>::Item),
    {
        self.query.get().for_each_unchecked(&*world.world, f)
    }

    /// Iterates through all entity data that matches the query.
    #[inline]
    pub fn for_each<'a, 'data, T>(&'a self, world: &SubWorld, f: T)
    where
        T: Fn(<<V as View<'data>>::Iter as Iterator>::Item),
        V: ReadOnly,
    {
        // safe because the view can only read data immutably
        unsafe { self.for_each_unchecked(world, f) }
    }

    /// Iterates through all entity data that matches the query.
    #[inline]
    pub fn for_each_mut<'a, 'data, T>(&'a self, world: &mut SubWorld, f: T)
    where
        T: Fn(<<V as View<'data>>::Iter as Iterator>::Item),
    {
        // safe because the &mut SubWorld ensures exclusivity
        unsafe { self.for_each_unchecked(world, f) }
    }

    /// Iterates through all entity data that matches the query.
    /// Does not perform static borrow checking.
    ///
    /// # Safety
    ///
    /// Incorrectly accessing components that are already borrowed elsewhere is undefined behavior.
    ///
    /// # Panics
    ///
    /// This function may panic if other code is concurrently accessing the same components.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub unsafe fn for_each_entities_unchecked<'a, 'data, T>(&'a self, world: &SubWorld, f: T)
    where
        T: Fn((Entity, <<V as View<'data>>::Iter as Iterator>::Item)),
    {
        self.query
            .get()
            .for_each_entities_unchecked(&*world.world, f)
    }

    /// Iterates through all entity data that matches the query.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub fn for_each_entities<'a, 'data, T>(&'a self, world: &SubWorld, f: T)
    where
        T: Fn((Entity, <<V as View<'data>>::Iter as Iterator>::Item)),
        V: ReadOnly,
    {
        // safe because the view can only read data immutably
        unsafe { self.for_each_entities_unchecked(world, f) }
    }

    /// Iterates through all entity data that matches the query.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub fn for_each_entities_mut<'a, 'data, T>(&'a self, world: &mut SubWorld, f: T)
    where
        T: Fn((Entity, <<V as View<'data>>::Iter as Iterator>::Item)),
    {
        // safe because the &mut SubWorld ensures exclusivity
        unsafe { self.for_each_entities_unchecked(world, f) }
    }

    /// Iterates through all entities that matches the query in parallel by chunk.
    /// Does not perform static borrow checking.
    ///
    /// # Safety
    ///
    /// Incorrectly accessing components that are already borrowed elsewhere is undefined behavior.
    ///
    /// # Panics
    ///
    /// This function may panic if other code is concurrently accessing the same components.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub unsafe fn par_entities_for_each_unchecked<'a, T>(&'a self, world: &SubWorld, f: T)
    where
        T: Fn((Entity, <<V as View<'a>>::Iter as Iterator>::Item)) + Send + Sync,
        <F::ArchetypeFilter as Filter<ArchetypeFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunksetFilter as Filter<ChunksetFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunkFilter as Filter<ChunkFilterData<'a>>>::Iter: FissileIterator,
    {
        self.query
            .get()
            .par_entities_for_each_unchecked(&*world.world, f)
    }

    /// Iterates through all entities that matches the query in parallel by chunk.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub fn par_entities_for_each<'a, T>(&'a self, world: &SubWorld, f: T)
    where
        T: Fn((Entity, <<V as View<'a>>::Iter as Iterator>::Item)) + Send + Sync,
        <F::ArchetypeFilter as Filter<ArchetypeFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunksetFilter as Filter<ChunksetFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunkFilter as Filter<ChunkFilterData<'a>>>::Iter: FissileIterator,
        V: ReadOnly,
    {
        // safe because the view can only read data immutably
        unsafe { self.par_entities_for_each_unchecked(world, f) }
    }

    /// Iterates through all entities that matches the query in parallel by chunk.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub fn par_entities_for_each_mut<'a, T>(&'a self, world: &mut SubWorld, f: T)
    where
        T: Fn((Entity, <<V as View<'a>>::Iter as Iterator>::Item)) + Send + Sync,
        <F::ArchetypeFilter as Filter<ArchetypeFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunksetFilter as Filter<ChunksetFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunkFilter as Filter<ChunkFilterData<'a>>>::Iter: FissileIterator,
    {
        // safe because the &mut SubWorld ensures exclusivity
        unsafe { self.par_entities_for_each_unchecked(world, f) }
    }

    /// Iterates through all entity data that matches the query in parallel.
    /// Does not perform static borrow checking.
    ///
    /// # Safety
    ///
    /// Incorrectly accessing components that are already borrowed elsewhere is undefined behavior.
    ///
    /// # Panics
    ///
    /// This function may panic if other code is concurrently accessing the same components.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub unsafe fn par_for_each_unchecked<'a, T>(&'a self, world: &SubWorld, f: T)
    where
        T: Fn(<<V as View<'a>>::Iter as Iterator>::Item) + Send + Sync,
        <F::ArchetypeFilter as Filter<ArchetypeFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunksetFilter as Filter<ChunksetFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunkFilter as Filter<ChunkFilterData<'a>>>::Iter: FissileIterator,
    {
        self.query.get().par_for_each_unchecked(&*world.world, f)
    }

    /// Iterates through all entity data that matches the query in parallel.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub fn par_for_each<'a, T>(&'a self, world: &SubWorld, f: T)
    where
        T: Fn(<<V as View<'a>>::Iter as Iterator>::Item) + Send + Sync,
        <F::ArchetypeFilter as Filter<ArchetypeFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunksetFilter as Filter<ChunksetFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunkFilter as Filter<ChunkFilterData<'a>>>::Iter: FissileIterator,
        V: ReadOnly,
    {
        // safe because the view can only read data immutably
        unsafe { self.par_for_each_unchecked(world, f) }
    }

    /// Iterates through all entity data that matches the query in parallel.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub fn par_for_each_mut<'a, T>(&'a self, world: &mut SubWorld, f: T)
    where
        T: Fn(<<V as View<'a>>::Iter as Iterator>::Item) + Send + Sync,
        <F::ArchetypeFilter as Filter<ArchetypeFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunksetFilter as Filter<ChunksetFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunkFilter as Filter<ChunkFilterData<'a>>>::Iter: FissileIterator,
    {
        // safe because the &mut SubWorld ensures exclusivity
        unsafe { self.par_for_each_unchecked(world, f) }
    }

    /// Gets a parallel iterator of chunks that match the query.
    /// Does not perform static borrow checking.
    ///
    /// # Safety
    ///
    /// Incorrectly accessing components that are already borrowed elsewhere is undefined behavior.
    ///
    /// # Panics
    ///
    /// This function may panic if other code is concurrently accessing the same components.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub unsafe fn par_for_each_chunk_unchecked<'a, T>(&'a self, world: &SubWorld, f: T)
    where
        T: Fn(Chunk<'a, V>) + Send + Sync,
        <F::ArchetypeFilter as Filter<ArchetypeFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunksetFilter as Filter<ChunksetFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunkFilter as Filter<ChunkFilterData<'a>>>::Iter: FissileIterator,
    {
        self.query
            .get()
            .par_for_each_chunk_unchecked(&*world.world, f)
    }

    /// Gets a parallel iterator of chunks that match the query.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub fn par_for_each_chunk<'a, T>(&'a self, world: &SubWorld, f: T)
    where
        T: Fn(Chunk<'a, V>) + Send + Sync,
        <F::ArchetypeFilter as Filter<ArchetypeFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunksetFilter as Filter<ChunksetFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunkFilter as Filter<ChunkFilterData<'a>>>::Iter: FissileIterator,
        V: ReadOnly,
    {
        // safe because the view can only read data immutably
        unsafe { self.par_for_each_chunk_unchecked(world, f) }
    }

    /// Gets a parallel iterator of chunks that match the query.
    #[cfg(feature = "par-iter")]
    #[inline]
    pub fn par_for_each_chunk_mut<'a, T>(&'a self, world: &mut SubWorld, f: T)
    where
        T: Fn(Chunk<'a, V>) + Send + Sync,
        <F::ArchetypeFilter as Filter<ArchetypeFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunksetFilter as Filter<ChunksetFilterData<'a>>>::Iter: FissileIterator,
        <F::ChunkFilter as Filter<ChunkFilterData<'a>>>::Iter: FissileIterator,
    {
        // safe because the &mut SubWorld ensures exclusivity
        unsafe { self.par_for_each_chunk_unchecked(world, f) }
    }
}

/// This trait is for providing abstraction across tuples of queries for populating the type
/// information in the system closure. This trait also provides access to the underlying query
/// information.
pub trait QuerySet: Send + Sync {
    type Queries;

    /// Returns the archetypes accessed by this collection of queries. This allows for caching
    /// effiency and granularity for system dispatching.
    fn filter_archetypes(&mut self, world: &World, archetypes: &mut BitSet);

    /// # Safety
    /// prepare call doesn't respect lifetimes of `self` and `world`.
    /// The returned value cannot outlive them.
    unsafe fn prepare(&mut self) -> Self::Queries;
}

macro_rules! impl_queryset_tuple {
    ($($ty: ident),*) => {
        paste::item! {
            #[allow(unused_parens, non_snake_case)]
            impl<$([<$ty V>], [<$ty F>], )*> QuerySet for ($(Query<[<$ty V>], [<$ty F>]>, )*)
            where
                $([<$ty V>]: for<'v> View<'v>,)*
                $([<$ty F>]: EntityFilter + Send + Sync,)*
            {
                type Queries = ( $(SystemQuery<[<$ty V>], [<$ty F>]>, )*  );
                fn filter_archetypes(&mut self, world: &World, bitset: &mut BitSet) {
                    let ($($ty,)*) = self;

                    $(
                        let storage = world.storage();
                        $ty.filter.iter_archetype_indexes(storage).for_each(|id| { bitset.insert(id); });
                    )*
                }
                unsafe fn prepare(&mut self) -> Self::Queries {
                    let ($($ty,)*) = self;
                    ($(SystemQuery::<[<$ty V>], [<$ty F>]>::new($ty),)*)
                }
            }
        }
    };
}

impl QuerySet for () {
    type Queries = ();
    fn filter_archetypes(&mut self, _: &World, _: &mut BitSet) {}
    unsafe fn prepare(&mut self) {}
}

impl<AV, AF> QuerySet for Query<AV, AF>
where
    AV: for<'v> View<'v>,
    AF: EntityFilter + Send + Sync,
{
    type Queries = SystemQuery<AV, AF>;
    fn filter_archetypes(&mut self, world: &World, bitset: &mut BitSet) {
        let storage = world.storage();
        self.filter.iter_archetype_indexes(storage).for_each(|id| {
            bitset.insert(id);
        });
    }
    unsafe fn prepare(&mut self) -> Self::Queries { SystemQuery::<AV, AF>::new(self) }
}

impl_queryset_tuple!(A);
impl_queryset_tuple!(A, B);
impl_queryset_tuple!(A, B, C);
impl_queryset_tuple!(A, B, C, D);
impl_queryset_tuple!(A, B, C, D, E);
impl_queryset_tuple!(A, B, C, D, E, F);
impl_queryset_tuple!(A, B, C, D, E, F, G);
impl_queryset_tuple!(A, B, C, D, E, F, G, H);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y);
impl_queryset_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

/// Provides access to a subset of the entities of a `World`.
pub struct SubWorld {
    world: *const World,
    access: *const Access<ComponentTypeId>,
    archetypes: Option<*const BitSet>,
}

impl SubWorld {
    unsafe fn new(
        world: &World,
        access: &Access<ComponentTypeId>,
        archetypes: &ArchetypeAccess,
    ) -> Self {
        SubWorld {
            world: world as *const World,
            access: access as *const Access<ComponentTypeId>,
            archetypes: if let ArchetypeAccess::Some(ref bitset) = archetypes {
                Some(bitset as *const BitSet)
            } else {
                None
            },
        }
    }
}

unsafe impl Sync for SubWorld {}
unsafe impl Send for SubWorld {}

// TODO: these assertions should have better errors
impl SubWorld {
    fn validate_archetype_access(&self, entity: Entity) -> bool {
        unsafe {
            if let Some(archetypes) = self.archetypes {
                if let Some(location) = (*self.world).entity_allocator.get_location(entity.index())
                {
                    return (*archetypes).contains(location.archetype());
                }
            }
        }

        true
    }

    fn validate_reads<T: Component>(&self, entity: Entity) {
        unsafe {
            if !(*self.access).reads.contains(&ComponentTypeId::of::<T>())
                || !self.validate_archetype_access(entity)
            {
                panic!("Attempted to read a component that this system does not have declared access to. \
                Consider adding a query which contains `{}` and this entity in its result set to the system, \
                or use `SystemBuilder::read_component` to declare global access.",
                std::any::type_name::<T>());
            }
        }
    }

    fn validate_writes<T: Component>(&self, entity: Entity) {
        unsafe {
            if !(*self.access).writes.contains(&ComponentTypeId::of::<T>())
                || !self.validate_archetype_access(entity)
            {
                panic!("Attempted to write to a component that this system does not have declared access to. \
                Consider adding a query which contains `{}` and this entity in its result set to the system, \
                or use `SystemBuilder::write_component` to declare global access.",
                std::any::type_name::<T>());
            }
        }
    }

    /// Borrows component data for the given entity.
    ///
    /// Returns `Some(data)` if the entity was found and contains the specified data.
    /// Otherwise `None` is returned.
    ///
    /// # Panics
    ///
    /// This function may panic if the component was not declared as read by this system.
    #[inline]
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<Ref<T>> {
        self.validate_reads::<T>(entity);
        unsafe { (*self.world).get_component::<T>(entity) }
    }

    /// Borrows component data for the given entity. Does not perform static borrow checking.
    ///
    /// Returns `Some(data)` if the entity was found and contains the specified data.
    /// Otherwise `None` is returned.
    ///
    /// # Safety
    ///
    /// Accessing a component which is already being concurrently accessed elsewhere is undefined behavior.
    ///
    /// # Panics
    ///
    /// This function may panic if any other code is currently borrowing `T` mutable or if the component was not declared
    /// as written by this system.
    #[inline]
    pub unsafe fn get_component_mut_unchecked<T: Component>(
        &self,
        entity: Entity,
    ) -> Option<RefMut<T>> {
        self.validate_writes::<T>(entity);
        (*self.world).get_component_mut_unchecked::<T>(entity)
    }

    /// Mutably borrows entity data for the given entity.
    ///
    /// Returns `Some(data)` if the entity was found and contains the specified data.
    /// Otherwise `None` is returned.
    ///
    /// # Panics
    ///
    /// This function may panic if the component was not declared as written by this system.
    #[inline]
    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<RefMut<T>> {
        // safe because the &mut self ensures exclusivity
        unsafe { self.get_component_mut_unchecked(entity) }
    }

    /// Gets tag data for the given entity.
    ///
    /// Returns `Some(data)` if the entity was found and contains the specified data.
    /// Otherwise `None` is returned.
    #[inline]
    pub fn get_tag<T: Tag>(&self, entity: Entity) -> Option<&T> {
        unsafe { (*self.world).get_tag(entity) }
    }

    /// Determines if the given `Entity` is alive within this `World`.
    #[inline]
    pub fn is_alive(&self, entity: Entity) -> bool { unsafe { (*self.world).is_alive(entity) } }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SystemId {
    name: Cow<'static, str>,
    type_id: TypeId,
}

struct Unspecified;

impl SystemId {
    pub fn of<T: 'static>(name: Option<String>) -> Self {
        Self {
            name: name
                .unwrap_or_else(|| std::any::type_name::<T>().to_string())
                .into(),
            type_id: TypeId::of::<T>(),
        }
    }
}

impl std::fmt::Display for SystemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<T: Into<Cow<'static, str>>> From<T> for SystemId {
    fn from(name: T) -> SystemId {
        SystemId {
            name: name.into(),
            type_id: TypeId::of::<Unspecified>(),
        }
    }
}

/// The concrete type which contains the system closure provided by the user.  This struct should
/// not be instantiated directly, and instead should be created using `SystemBuilder`.
///
/// Implements `Schedulable` which is consumable by the `StageExecutor`, executing the closure.
///
/// Also handles caching of archetype information in a `BitSet`, as well as maintaining the provided
/// information about what queries this system will run and, as a result, its data access.
///
/// Queries are stored generically within this struct, and the `SystemQuery` types are generated
/// on each `run` call, wrapping the world and providing the set to the user in their closure.
pub struct System<R, Q, F>
where
    R: ResourceSet,
    Q: QuerySet,
    F: SystemFn<
        Resources = <R as ResourceSet>::PreparedResources,
        Queries = <Q as QuerySet>::Queries,
    >,
{
    name: SystemId,
    resources: R,
    queries: AtomicRefCell<Q>,
    run_fn: AtomicRefCell<F>,
    archetypes: ArchetypeAccess,

    // These are stored statically instead of always iterated and created from the
    // query types, which would make allocations every single request
    access: SystemAccess,

    // We pre-allocate a command buffer for ourself. Writes are self-draining so we never have to rellocate.
    command_buffer: AtomicRefCell<CommandBuffer>,
}

impl<R, Q, F> Runnable for System<R, Q, F>
where
    R: ResourceSet,
    Q: QuerySet,
    F: SystemFn<
        Resources = <R as ResourceSet>::PreparedResources,
        Queries = <Q as QuerySet>::Queries,
    >,
{
    fn name(&self) -> &SystemId { &self.name }

    fn reads(&self) -> (&[ResourceTypeId], &[ComponentTypeId]) {
        (&self.access.resources.reads, &self.access.components.reads)
    }
    fn writes(&self) -> (&[ResourceTypeId], &[ComponentTypeId]) {
        (
            &self.access.resources.writes,
            &self.access.components.writes,
        )
    }

    fn prepare(&mut self, world: &World) {
        if let ArchetypeAccess::Some(bitset) = &mut self.archetypes {
            self.queries.get_mut().filter_archetypes(world, bitset);
        }
    }

    fn accesses_archetypes(&self) -> &ArchetypeAccess { &self.archetypes }

    fn command_buffer_mut(&self) -> RefMut<CommandBuffer> { self.command_buffer.get_mut() }

    fn run(&self, world: &World) {
        let span = span!(Level::INFO, "System", system = %self.name);
        let _guard = span.enter();

        debug!("Initializing");
        let mut resources = unsafe { R::fetch_unchecked(&world.resources) };
        let mut queries = self.queries.get_mut();
        let mut prepared_queries = unsafe { queries.prepare() };
        let mut world_shim =
            unsafe { SubWorld::new(world, &self.access.components, &self.archetypes) };

        // Give the command buffer a new entity block.
        // This should usually just pull a free block, or allocate a new one...
        // TODO: The BlockAllocator should *ensure* keeping at least 1 free block so this prevents an allocation

        info!("Running");
        use std::ops::DerefMut;
        let mut borrow = self.run_fn.get_mut();
        borrow.deref_mut().run(
            &mut self.command_buffer.get_mut(),
            &mut world_shim,
            &mut resources,
            &mut prepared_queries,
        );
    }
}

/// Supertrait used for defining systems. All wrapper objects for systems implement this trait.
///
/// This trait will generally not be used by users.
pub trait SystemFn {
    type Resources;
    type Queries;

    fn run(
        &mut self,
        commands: &mut CommandBuffer,
        world: &mut SubWorld,
        resources: &mut Self::Resources,
        queries: &mut Self::Queries,
    );
}

struct SystemFnWrapper<R, Q, F: FnMut(&mut CommandBuffer, &mut SubWorld, &mut R, &mut Q) + 'static>(
    F,
    PhantomData<(R, Q)>,
);

impl<F, R, Q> SystemFn for SystemFnWrapper<R, Q, F>
where
    F: FnMut(&mut CommandBuffer, &mut SubWorld, &mut R, &mut Q) + 'static,
{
    type Resources = R;
    type Queries = Q;

    fn run(
        &mut self,
        commands: &mut CommandBuffer,
        world: &mut SubWorld,
        resources: &mut Self::Resources,
        queries: &mut Self::Queries,
    ) {
        (self.0)(commands, world, resources, queries);
    }
}

// This builder uses a Cons/Hlist implemented in cons.rs to generated the static query types
// for this system. Access types are instead stored and abstracted in the top level vec here
// so the underlying ResourceSet type functions from the queries don't need to allocate.
// Otherwise, this leads to excessive alloaction for every call to reads/writes
/// The core builder of `System` types, which are systems within Legion. Systems are implemented
/// as singular closures for a given system - providing queries which should be cached for that
/// system, as well as resource access and other metadata.
/// ```rust
/// # use legion::prelude::*;
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Position;
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Velocity;
/// # #[derive(Copy, Clone, Debug, PartialEq)]
/// # struct Model;
/// #[derive(Copy, Clone, Debug, PartialEq)]
/// struct Static;
/// #[derive(Debug)]
/// struct TestResource {}
///
///  let mut system_one = SystemBuilder::<()>::new("TestSystem")
///            .read_resource::<TestResource>()
///            .with_query(<(Read<Position>, Tagged<Model>)>::query()
///                         .filter(!tag::<Static>() | changed::<Position>()))
///            .build(move |commands, world, resource, queries| {
///               let mut count = 0;
///                {
///                    for (entity, pos) in queries.iter_entities_mut(&mut *world) {
///
///                    }
///                }
///            });
/// ```
pub struct SystemBuilder<Q = (), R = ()> {
    name: SystemId,

    queries: Q,
    resources: R,

    resource_access: Access<ResourceTypeId>,
    component_access: Access<ComponentTypeId>,
    access_all_archetypes: bool,
}

impl SystemBuilder<(), ()> {
    /// Create a new system builder to construct a new system.
    ///
    /// Please note, the `name` argument for this method is just for debugging and visualization
    /// purposes and is not logically used anywhere.
    pub fn new<T: Into<SystemId>>(name: T) -> Self {
        Self {
            name: name.into(),
            queries: (),
            resources: (),
            resource_access: Access::default(),
            component_access: Access::default(),
            access_all_archetypes: false,
        }
    }
}

impl<Q, R> SystemBuilder<Q, R>
where
    Q: 'static + Send + ConsFlatten,
    R: 'static + Send + ConsFlatten,
{
    /// Defines a query to provide this system for its execution. Multiple queries can be provided,
    /// and queries are cached internally for efficiency for filtering and archetype ID handling.
    ///
    /// It is best practice to define your queries here, to allow for the caching to take place.
    /// These queries are then provided to the executing closure as a tuple of queries.
    pub fn with_query<V, F>(
        mut self,
        query: Query<V, F>,
    ) -> SystemBuilder<<Q as ConsAppend<Query<V, F>>>::Output, R>
    where
        V: for<'a> View<'a>,
        F: 'static + EntityFilter,
        Q: ConsAppend<Query<V, F>>,
    {
        self.component_access.reads.extend(V::read_types().iter());
        self.component_access.writes.extend(V::write_types().iter());

        SystemBuilder {
            name: self.name,
            queries: ConsAppend::append(self.queries, query),
            resources: self.resources,
            resource_access: self.resource_access,
            component_access: self.component_access,
            access_all_archetypes: self.access_all_archetypes,
        }
    }

    /// Flag this resource type as being read by this system.
    ///
    /// This will inform the dispatcher to not allow any writes access to this resource while
    /// this system is running. Parralel reads still occur during execution.
    pub fn read_resource<T>(mut self) -> SystemBuilder<Q, <R as ConsAppend<Read<T>>>::Output>
    where
        T: 'static + Resource,
        R: ConsAppend<Read<T>>,
        <R as ConsAppend<Read<T>>>::Output: ConsFlatten,
    {
        self.resource_access.reads.push(ResourceTypeId::of::<T>());

        SystemBuilder {
            name: self.name,
            queries: self.queries,
            resources: ConsAppend::append(self.resources, Read::<T>::default()),
            resource_access: self.resource_access,
            component_access: self.component_access,
            access_all_archetypes: self.access_all_archetypes,
        }
    }

    /// Flag this resource type as being written by this system.
    ///
    /// This will inform the dispatcher to not allow any parallel access to this resource while
    /// this system is running.
    pub fn write_resource<T>(mut self) -> SystemBuilder<Q, <R as ConsAppend<Write<T>>>::Output>
    where
        T: 'static + Resource,
        R: ConsAppend<Write<T>>,
        <R as ConsAppend<Write<T>>>::Output: ConsFlatten,
    {
        self.resource_access.writes.push(ResourceTypeId::of::<T>());

        SystemBuilder {
            name: self.name,
            queries: self.queries,
            resources: ConsAppend::append(self.resources, Write::<T>::default()),
            resource_access: self.resource_access,
            component_access: self.component_access,
            access_all_archetypes: self.access_all_archetypes,
        }
    }

    /// This performs a soft resource block on the component for writing. The dispatcher will
    /// generally handle dispatching read and writes on components based on archetype, allowing
    /// for more granular access and more parallelization of systems.
    ///
    /// Using this method will mark the entire component as read by this system, blocking writing
    /// systems from accessing any archetypes which contain this component for the duration of its
    /// execution.
    ///
    /// This type of access with `SubWorld` is provided for cases where sparse component access
    /// is required and searching entire query spaces for entities is inefficient.
    pub fn read_component<T>(mut self) -> Self
    where
        T: Component,
    {
        self.component_access.reads.push(ComponentTypeId::of::<T>());
        self.access_all_archetypes = true;

        self
    }

    /// This performs a exclusive resource block on the component for writing. The dispatcher will
    /// generally handle dispatching read and writes on components based on archetype, allowing
    /// for more granular access and more parallelization of systems.
    ///
    /// Using this method will mark the entire component as written by this system, blocking other
    /// systems from accessing any archetypes which contain this component for the duration of its
    /// execution.
    ///
    /// This type of access with `SubWorld` is provided for cases where sparse component access
    /// is required and searching entire query spaces for entities is inefficient.
    pub fn write_component<T>(mut self) -> Self
    where
        T: Component,
    {
        self.component_access
            .writes
            .push(ComponentTypeId::of::<T>());
        self.access_all_archetypes = true;

        self
    }

    /// Builds a standard legion `System`. A system is considered a closure for all purposes. This
    /// closure is `FnMut`, allowing for capture of variables for tracking state for this system.
    /// Instead of the classic OOP architecture of a system, this lets you still maintain state
    /// across execution of the systems while leveraging the type semantics of closures for better
    /// ergonomics.
    pub fn build<F>(self, run_fn: F) -> Box<dyn Schedulable>
    where
        <R as ConsFlatten>::Output: ResourceSet + Send + Sync,
        <Q as ConsFlatten>::Output: QuerySet,
        <<R as ConsFlatten>::Output as ResourceSet>::PreparedResources: Send + Sync,
        <<Q as ConsFlatten>::Output as QuerySet>::Queries: Send + Sync,
        F: FnMut(
                &mut CommandBuffer,
                &mut SubWorld,
                &mut <<R as ConsFlatten>::Output as ResourceSet>::PreparedResources,
                &mut <<Q as ConsFlatten>::Output as QuerySet>::Queries,
            ) + Send
            + Sync
            + 'static,
    {
        let run_fn = SystemFnWrapper(run_fn, PhantomData);
        Box::new(System {
            name: self.name,
            run_fn: AtomicRefCell::new(run_fn),
            resources: self.resources.flatten(),
            queries: AtomicRefCell::new(self.queries.flatten()),
            archetypes: if self.access_all_archetypes {
                ArchetypeAccess::All
            } else {
                ArchetypeAccess::Some(BitSet::default())
            },
            access: SystemAccess {
                resources: self.resource_access,
                components: self.component_access,
                tags: Access::default(),
            },
            command_buffer: AtomicRefCell::new(CommandBuffer::default()),
        })
    }

    /// Builds a system which is not `Schedulable`, as it is not thread safe (!Send and !Sync),
    /// but still implements all the calling infrastructure of the `Runnable` trait. This provides
    /// a way for legion consumers to leverage the `System` construction and type-handling of
    /// this build for thread local systems which cannot leave the main initializing thread.
    pub fn build_thread_local<F>(self, run_fn: F) -> Box<dyn Runnable>
    where
        <R as ConsFlatten>::Output: ResourceSet + Send + Sync,
        <Q as ConsFlatten>::Output: QuerySet,
        F: FnMut(
                &mut CommandBuffer,
                &mut SubWorld,
                &mut <<R as ConsFlatten>::Output as ResourceSet>::PreparedResources,
                &mut <<Q as ConsFlatten>::Output as QuerySet>::Queries,
            ) + 'static,
    {
        let run_fn = SystemFnWrapper(run_fn, PhantomData);
        Box::new(System {
            name: self.name,
            run_fn: AtomicRefCell::new(run_fn),
            resources: self.resources.flatten(),
            queries: AtomicRefCell::new(self.queries.flatten()),
            archetypes: if self.access_all_archetypes {
                ArchetypeAccess::All
            } else {
                ArchetypeAccess::Some(BitSet::default())
            },
            access: SystemAccess {
                resources: self.resource_access,
                components: self.component_access,
                tags: Access::default(),
            },
            command_buffer: AtomicRefCell::new(CommandBuffer::default()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    use crate::schedule::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Pos(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Vel(f32, f32, f32);

    #[derive(Default)]
    struct TestResource(pub i32);
    #[derive(Default)]
    struct TestResourceTwo(pub i32);
    #[derive(Default)]
    struct TestResourceThree(pub i32);
    #[derive(Default)]
    struct TestResourceFour(pub i32);

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestComp(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestCompTwo(f32, f32, f32);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct TestCompThree(f32, f32, f32);

    #[test]
    fn builder_schedule_execute() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();
        world.resources.insert(TestResource(123));
        world.resources.insert(TestResourceTwo(123));

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        #[derive(Debug, Eq, PartialEq)]
        pub enum TestSystems {
            TestSystemOne,
            TestSystemTwo,
            TestSystemThree,
            TestSystemFour,
        }

        let runs = Arc::new(Mutex::new(Vec::new()));

        let system_one_runs = runs.clone();
        let system_one = SystemBuilder::<()>::new("TestSystem1")
            .read_resource::<TestResource>()
            .with_query(Read::<Pos>::query())
            .with_query(Write::<Vel>::query())
            .build(move |_commands, _world, _resource, _queries| {
                tracing::trace!("system_one");
                system_one_runs
                    .lock()
                    .unwrap()
                    .push(TestSystems::TestSystemOne);
            });

        let system_two_runs = runs.clone();
        let system_two = SystemBuilder::<()>::new("TestSystem2")
            .write_resource::<TestResourceTwo>()
            .with_query(Read::<Vel>::query())
            .build(move |_commands, _world, _resource, _queries| {
                tracing::trace!("system_two");
                system_two_runs
                    .lock()
                    .unwrap()
                    .push(TestSystems::TestSystemTwo);
            });

        let system_three_runs = runs.clone();
        let system_three = SystemBuilder::<()>::new("TestSystem3")
            .read_resource::<TestResourceTwo>()
            .with_query(Read::<Vel>::query())
            .build(move |_commands, _world, _resource, _queries| {
                tracing::trace!("system_three");
                system_three_runs
                    .lock()
                    .unwrap()
                    .push(TestSystems::TestSystemThree);
            });
        let system_four_runs = runs.clone();
        let system_four = SystemBuilder::<()>::new("TestSystem4")
            .write_resource::<TestResourceTwo>()
            .with_query(Read::<Vel>::query())
            .build(move |_commands, _world, _resource, _queries| {
                tracing::trace!("system_four");
                system_four_runs
                    .lock()
                    .unwrap()
                    .push(TestSystems::TestSystemFour);
            });

        let order = vec![
            TestSystems::TestSystemOne,
            TestSystems::TestSystemTwo,
            TestSystems::TestSystemThree,
            TestSystems::TestSystemFour,
        ];

        let systems = vec![system_one, system_two, system_three, system_four];

        let mut executor = Executor::new(systems);
        executor.execute(&mut world);

        assert_eq!(*(runs.lock().unwrap()), order);
    }

    #[test]
    fn builder_create_and_execute() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();
        world.resources.insert(TestResource(123));

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        let mut system = SystemBuilder::<()>::new("TestSystem")
            .read_resource::<TestResource>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_commands, world, resource, queries| {
                assert_eq!(resource.0, 123);
                let mut count = 0;
                {
                    for (entity, pos) in queries.0.iter_entities(world) {
                        assert_eq!(expected.get(&entity).unwrap().0, *pos);
                        count += 1;
                    }
                }

                assert_eq!(components.len(), count);
            });
        system.prepare(&world);
        system.run(&world);
    }

    #[test]
    fn fnmut_stateful_system_test() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();
        world.resources.insert(TestResource(123));

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        let mut state = 0;
        let mut system = SystemBuilder::<()>::new("TestSystem")
            .read_resource::<TestResource>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, _, _| {
                state += 1;
            });

        system.prepare(&world);
        system.run(&world);
    }

    #[test]
    fn system_mutate_archetype() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        #[derive(Default, Clone, Copy)]
        pub struct Balls(u32);

        let components = vec![
            (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)),
            (Pos(4., 5., 6.), Vel(0.4, 0.5, 0.6)),
        ];

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        let expected_copy = expected.clone();
        let mut system = SystemBuilder::<()>::new("TestSystem")
            .with_query(<(Read<Pos>, Read<Vel>)>::query())
            .build(move |_, world, _, query| {
                let mut count = 0;
                {
                    for (entity, (pos, vel)) in query.iter_entities(world) {
                        assert_eq!(expected_copy.get(&entity).unwrap().0, *pos);
                        assert_eq!(expected_copy.get(&entity).unwrap().1, *vel);
                        count += 1;
                    }
                }

                assert_eq!(components.len(), count);
            });

        system.prepare(&world);
        system.run(&world);

        world
            .add_component(*(expected.keys().nth(0).unwrap()), Balls::default())
            .unwrap();

        system.prepare(&world);
        system.run(&world);
    }

    #[test]
    fn system_mutate_archetype_buffer() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        #[derive(Default, Clone, Copy)]
        pub struct Balls(u32);

        let components = (0..30000)
            .map(|_| (Pos(1., 2., 3.), Vel(0.1, 0.2, 0.3)))
            .collect::<Vec<_>>();

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        let expected_copy = expected.clone();
        let mut system = SystemBuilder::<()>::new("TestSystem")
            .with_query(<(Read<Pos>, Read<Vel>)>::query())
            .build(move |command_buffer, world, _, query| {
                let mut count = 0;
                {
                    for (entity, (pos, vel)) in query.iter_entities(world) {
                        assert_eq!(expected_copy.get(&entity).unwrap().0, *pos);
                        assert_eq!(expected_copy.get(&entity).unwrap().1, *vel);
                        count += 1;

                        command_buffer.add_component(entity, Balls::default());
                    }
                }

                assert_eq!(components.len(), count);
            });

        system.prepare(&world);
        system.run(&world);

        system.command_buffer_mut().write(&mut world);

        system.prepare(&world);
        system.run(&world);
    }

    #[test]
    #[cfg(feature = "par-schedule")]
    fn par_res_write() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let _ = tracing_subscriber::fmt::try_init();

        #[derive(Default)]
        struct AtomicRes(AtomicRefCell<AtomicUsize>);

        let universe = Universe::new();
        let mut world = universe.create_world();
        world.resources.insert(AtomicRes::default());

        let system1 = SystemBuilder::<()>::new("TestSystem1")
            .write_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get_mut().fetch_add(1, Ordering::SeqCst);
            });

        let system2 = SystemBuilder::<()>::new("TestSystem2")
            .write_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get_mut().fetch_add(1, Ordering::SeqCst);
            });

        let system3 = SystemBuilder::<()>::new("TestSystem3")
            .write_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get_mut().fetch_add(1, Ordering::SeqCst);
            });

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap();

        tracing::debug!(
            reads = ?system1.reads(),
            writes = ?system1.writes(),
            "System access"
        );

        let systems = vec![system1, system2, system3];
        let mut executor = Executor::new(systems);
        pool.install(|| {
            for _ in 0..1000 {
                executor.execute(&mut world);
            }
        });

        assert_eq!(
            world
                .resources
                .get::<AtomicRes>()
                .unwrap()
                .0
                .get()
                .load(Ordering::SeqCst),
            3 * 1000,
        );
    }

    #[test]
    #[cfg(feature = "par-schedule")]
    fn par_res_readwrite() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let _ = tracing_subscriber::fmt::try_init();

        #[derive(Default)]
        struct AtomicRes(AtomicRefCell<AtomicUsize>);

        let universe = Universe::new();
        let mut world = universe.create_world();
        world.resources.insert(AtomicRes::default());

        let system1 = SystemBuilder::<()>::new("TestSystem1")
            .read_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get().fetch_add(1, Ordering::SeqCst);
            });

        let system2 = SystemBuilder::<()>::new("TestSystem2")
            .write_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get_mut().fetch_add(1, Ordering::SeqCst);
            });

        let system3 = SystemBuilder::<()>::new("TestSystem3")
            .write_resource::<AtomicRes>()
            .with_query(Read::<Pos>::query())
            .with_query(Read::<Vel>::query())
            .build(move |_, _, resource, _| {
                resource.0.get_mut().fetch_add(1, Ordering::SeqCst);
            });

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap();

        tracing::debug!(
            reads = ?system1.reads(),
            writes = ?system1.writes(),
            "System access"
        );

        let systems = vec![system1, system2, system3];
        let mut executor = Executor::new(systems);
        pool.install(|| {
            for _ in 0..1000 {
                executor.execute(&mut world);
            }
        });
    }

    #[test]
    #[cfg(feature = "par-schedule")]
    #[allow(clippy::float_cmp)]
    fn par_comp_readwrite() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        #[derive(Clone, Copy, Debug, PartialEq)]
        struct Comp1(f32, f32, f32);
        #[derive(Clone, Copy, Debug, PartialEq)]
        struct Comp2(f32, f32, f32);

        let components = vec![
            (Pos(69., 69., 69.), Vel(69., 69., 69.)),
            (Pos(69., 69., 69.), Vel(69., 69., 69.)),
        ];

        let mut expected = HashMap::<Entity, (Pos, Vel)>::new();

        for (i, e) in world.insert((), components.clone()).iter().enumerate() {
            if let Some((pos, rot)) = components.get(i) {
                expected.insert(*e, (*pos, *rot));
            }
        }

        let system1 = SystemBuilder::<()>::new("TestSystem1")
            .with_query(<(Read<Comp1>, Read<Comp2>)>::query())
            .build(move |_, world, _, query| {
                query.iter(world).for_each(|(one, two)| {
                    assert_eq!(one.0, 69.);
                    assert_eq!(one.1, 69.);
                    assert_eq!(one.2, 69.);

                    assert_eq!(two.0, 69.);
                    assert_eq!(two.1, 69.);
                    assert_eq!(two.2, 69.);
                });
            });

        let system2 = SystemBuilder::<()>::new("TestSystem2")
            .with_query(<(Write<Comp1>, Read<Comp2>)>::query())
            .build(move |_, world, _, query| {
                query.iter_mut(world).for_each(|(mut one, two)| {
                    one.0 = 456.;
                    one.1 = 456.;
                    one.2 = 456.;

                    assert_eq!(two.0, 69.);
                    assert_eq!(two.1, 69.);
                    assert_eq!(two.2, 69.);
                });
            });

        let system3 = SystemBuilder::<()>::new("TestSystem3")
            .with_query(<(Write<Comp1>, Write<Comp2>)>::query())
            .build(move |_, world, _, query| {
                query.iter_mut(world).for_each(|(mut one, two)| {
                    assert_eq!(one.0, 456.);
                    assert_eq!(one.1, 456.);
                    assert_eq!(one.2, 456.);

                    assert_eq!(two.0, 69.);
                    assert_eq!(two.1, 69.);
                    assert_eq!(two.2, 69.);

                    one.0 = 789.;
                    one.1 = 789.;
                    one.2 = 789.;

                    one.0 = 789.;
                    one.1 = 789.;
                    one.2 = 789.;
                });
            });

        let system4 = SystemBuilder::<()>::new("TestSystem4")
            .with_query(<(Read<Comp1>, Read<Comp2>)>::query())
            .build(move |_, world, _, query| {
                query.iter(world).for_each(|(one, two)| {
                    assert_eq!(one.0, 789.);
                    assert_eq!(one.1, 789.);
                    assert_eq!(one.2, 789.);

                    assert_eq!(two.0, 789.);
                    assert_eq!(two.1, 789.);
                    assert_eq!(two.2, 789.);
                });
            });

        let system5 = SystemBuilder::<()>::new("TestSystem5")
            .with_query(<(Write<Comp1>, Write<Comp2>)>::query())
            .build(move |_, world, _, query| {
                query.iter_mut(world).for_each(|(mut one, mut two)| {
                    assert_eq!(one.0, 789.);
                    assert_eq!(one.1, 789.);
                    assert_eq!(one.2, 789.);

                    assert_eq!(two.0, 789.);
                    assert_eq!(two.1, 789.);
                    assert_eq!(two.2, 789.);

                    one.0 = 69.;
                    one.1 = 69.;
                    one.2 = 69.;

                    two.0 = 69.;
                    two.1 = 69.;
                    two.2 = 69.;
                });
            });

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap();

        tracing::debug!(
            reads = ?system1.reads(),
            writes = ?system1.writes(),
            "System access"
        );

        let systems = vec![system1, system2, system3, system4, system5];
        let mut executor = Executor::new(systems);
        pool.install(|| {
            for _ in 0..1000 {
                executor.execute(&mut world);
            }
        });
    }
}
