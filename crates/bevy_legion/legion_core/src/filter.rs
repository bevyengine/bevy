use crate::index::ArchetypeIndex;
use crate::index::ChunkIndex;
use crate::index::SetIndex;
use crate::iterator::FissileZip;
use crate::iterator::SliceVecIter;
use crate::storage::ArchetypeData;
use crate::storage::ArchetypeId;
use crate::storage::Component;
use crate::storage::ComponentStorage;
use crate::storage::ComponentTypeId;
use crate::storage::ComponentTypes;
use crate::storage::Storage;
use crate::storage::Tag;
use crate::storage::TagTypeId;
use crate::storage::TagTypes;
use std::iter::Enumerate;
use std::iter::Repeat;
use std::iter::Take;
use std::marker::PhantomData;
use std::slice::Iter;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

pub mod filter_fns {
    ///! Contains functions for constructing filters.
    use super::*;

    pub fn passthrough() -> EntityFilterTuple<Passthrough, Passthrough, Passthrough> {
        EntityFilterTuple::new(Passthrough, Passthrough, Passthrough)
    }

    pub fn any() -> EntityFilterTuple<Any, Any, Any> { EntityFilterTuple::new(Any, Any, Any) }

    /// Creates an entity data filter which includes chunks that contain
    /// entity data components of type `T`.
    pub fn component<T: Component>(
    ) -> EntityFilterTuple<ComponentFilter<T>, Passthrough, Passthrough> {
        EntityFilterTuple::new(ComponentFilter::new(), Passthrough, Passthrough)
    }

    /// Creates a shared data filter which includes chunks that contain
    /// shared data components of type `T`.
    pub fn tag<T: Tag>() -> EntityFilterTuple<TagFilter<T>, Passthrough, Passthrough> {
        EntityFilterTuple::new(TagFilter::new(), Passthrough, Passthrough)
    }

    /// Creates a shared data filter which includes chunks that contain
    /// specific shared data values.
    pub fn tag_value<'a, T: Tag>(
        data: &'a T,
    ) -> EntityFilterTuple<TagFilter<T>, TagValueFilter<'a, T>, Passthrough> {
        EntityFilterTuple::new(TagFilter::new(), TagValueFilter::new(data), Passthrough)
    }

    /// Creates a filter which includes chunks for which entity data components
    /// of type `T` have changed since the filter was last executed.
    pub fn changed<T: Component>(
    ) -> EntityFilterTuple<ComponentFilter<T>, Passthrough, ComponentChangedFilter<T>> {
        EntityFilterTuple::new(
            ComponentFilter::new(),
            Passthrough,
            ComponentChangedFilter::new(),
        )
    }
}

pub(crate) trait FilterResult {
    fn coalesce_and(self, other: Self) -> Self;
    fn coalesce_or(self, other: Self) -> Self;
    fn is_pass(&self) -> bool;
}

impl FilterResult for Option<bool> {
    #[inline]
    fn coalesce_and(self, other: Self) -> Self {
        match self {
            Some(x) => other.map(|y| x && y).or(Some(x)),
            None => other,
        }
    }

    #[inline]
    fn coalesce_or(self, other: Self) -> Self {
        match self {
            Some(x) => other.map(|y| x || y).or(Some(x)),
            None => other,
        }
    }

    #[inline]
    fn is_pass(&self) -> bool { self.unwrap_or(true) }
}

/// A streaming iterator of bools.
pub trait Filter<T: Copy>: Send + Sync + Sized {
    type Iter: Iterator + Send + Sync;

    // Called when a query is about to begin execution.
    fn init(&self) {}

    /// Pulls iterator data out of the source.
    fn collect(&self, source: T) -> Self::Iter;

    /// Determines if an element of `Self::Iter` matches the filter conditions.
    fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool>;

    /// Creates an iterator which yields bools for each element in the source
    /// which indicate if the element matches the filter.
    fn matches(&mut self, source: T) -> FilterIter<Self, T> {
        FilterIter {
            elements: self.collect(source),
            filter: self,
            _phantom: PhantomData,
        }
    }
}

/// An iterator over the elements matching a filter.
pub struct FilterIter<'a, F: Filter<T>, T: Copy> {
    elements: <F as Filter<T>>::Iter,
    filter: &'a mut F,
    _phantom: PhantomData<T>,
}

impl<'a, F: Filter<T>, T: Copy> Iterator for FilterIter<'a, F, T> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        self.elements
            .next()
            .map(|x| self.filter.is_match(&x).is_pass())
    }
}

impl<'a, F: Filter<T>, T: Copy + 'a> FilterIter<'a, F, T> {
    /// Finds the indices of all elements matching the filter.
    pub fn matching_indices(self) -> impl Iterator<Item = usize> + 'a {
        self.enumerate().filter(|(_, x)| *x).map(|(i, _)| i)
    }
}

/// Input data for archetype filters.
#[derive(Copy, Clone)]
pub struct ArchetypeFilterData<'a> {
    /// The component types in each archetype.
    pub component_types: &'a ComponentTypes,
    /// The tag types in each archetype.
    pub tag_types: &'a TagTypes,
}

/// Input data for chunkset filters.
#[derive(Copy, Clone)]
pub struct ChunksetFilterData<'a> {
    /// The component data in an archetype.
    pub archetype_data: &'a ArchetypeData,
}

/// Input data for chunk filters.
#[derive(Copy, Clone)]
pub struct ChunkFilterData<'a> {
    // The components in a set
    pub chunks: &'a [ComponentStorage],
}

/// A marker trait for filters that are not no-ops.
pub trait ActiveFilter {}

/// A type which combines both an archetype and a chunk filter.
pub trait EntityFilter: Send + Clone {
    type ArchetypeFilter: for<'a> Filter<ArchetypeFilterData<'a>> + Clone;
    type ChunksetFilter: for<'a> Filter<ChunksetFilterData<'a>> + Clone;
    type ChunkFilter: for<'a> Filter<ChunkFilterData<'a>> + Clone;

    /// Initializes the entity filter for iteration.
    fn init(&self);

    /// Gets mutable references to both inner filters.
    fn filters(
        &self,
    ) -> (
        &Self::ArchetypeFilter,
        &Self::ChunksetFilter,
        &Self::ChunkFilter,
    );

    /// Converts self into both inner filters.
    fn into_filters(
        self,
    ) -> (
        Self::ArchetypeFilter,
        Self::ChunksetFilter,
        Self::ChunkFilter,
    );

    /// Gets an iterator over all matching archetype indexes.
    fn iter_archetype_indexes<'a, 'b>(
        &'a self,
        storage: &'b Storage,
    ) -> FilterArchIter<'b, 'a, Self::ArchetypeFilter>;

    /// Gets an iterator over all matching chunkset indexes.
    fn iter_chunkset_indexes<'a, 'b>(
        &'a self,
        archetype: &'b ArchetypeData,
    ) -> FilterChunkIter<'b, 'a, Self::ChunksetFilter>;

    /// Gets an iterator over all matching archetypes and chunksets.
    fn iter<'a, 'b>(
        &'a self,
        storage: &'b Storage,
    ) -> FilterEntityIter<'b, 'a, Self::ArchetypeFilter, Self::ChunksetFilter>;
}

/// An EntityFilter which combined both an archetype filter and a chunk filter.
#[derive(Debug, Clone)]
pub struct EntityFilterTuple<A, S, C> {
    pub arch_filter: A,
    pub chunkset_filter: S,
    pub chunk_filter: C,
}

impl<A, S, C> EntityFilterTuple<A, S, C>
where
    A: for<'a> Filter<ArchetypeFilterData<'a>>,
    S: for<'a> Filter<ChunksetFilterData<'a>>,
    C: for<'a> Filter<ChunkFilterData<'a>>,
{
    /// Creates a new entity filter.
    pub fn new(arch_filter: A, chunkset_filter: S, chunk_filter: C) -> Self {
        Self {
            arch_filter,
            chunkset_filter,
            chunk_filter,
        }
    }
}

impl<A, S, C> EntityFilter for EntityFilterTuple<A, S, C>
where
    A: for<'a> Filter<ArchetypeFilterData<'a>> + Clone,
    S: for<'a> Filter<ChunksetFilterData<'a>> + Clone,
    C: for<'a> Filter<ChunkFilterData<'a>> + Clone,
{
    type ArchetypeFilter = A;
    type ChunksetFilter = S;
    type ChunkFilter = C;

    fn init(&self) {
        self.arch_filter.init();
        self.chunkset_filter.init();
        self.chunk_filter.init();
    }

    fn filters(
        &self,
    ) -> (
        &Self::ArchetypeFilter,
        &Self::ChunksetFilter,
        &Self::ChunkFilter,
    ) {
        (&self.arch_filter, &self.chunkset_filter, &self.chunk_filter)
    }

    fn into_filters(
        self,
    ) -> (
        Self::ArchetypeFilter,
        Self::ChunksetFilter,
        Self::ChunkFilter,
    ) {
        (self.arch_filter, self.chunkset_filter, self.chunk_filter)
    }

    fn iter_archetype_indexes<'a, 'b>(&'a self, storage: &'b Storage) -> FilterArchIter<'b, 'a, A> {
        let data = ArchetypeFilterData {
            component_types: storage.component_types(),
            tag_types: storage.tag_types(),
        };

        let iter = self.arch_filter.collect(data);
        FilterArchIter {
            archetypes: iter.enumerate(),
            filter: &self.arch_filter,
        }
    }

    fn iter_chunkset_indexes<'a, 'b>(
        &'a self,
        archetype: &'b ArchetypeData,
    ) -> FilterChunkIter<'b, 'a, S> {
        let data = ChunksetFilterData {
            archetype_data: archetype,
        };

        let iter = self.chunkset_filter.collect(data);
        FilterChunkIter {
            chunks: iter.enumerate(),
            filter: &self.chunkset_filter,
        }
    }

    fn iter<'a, 'b>(&'a self, storage: &'b Storage) -> FilterEntityIter<'b, 'a, A, S> {
        let data = ArchetypeFilterData {
            component_types: storage.component_types(),
            tag_types: storage.tag_types(),
        };

        let iter = self.arch_filter.collect(data).enumerate();
        FilterEntityIter {
            storage,
            arch_filter: &self.arch_filter,
            chunk_filter: &self.chunkset_filter,
            archetypes: iter,
            chunks: None,
        }
    }
}

impl<A, S, C> std::ops::Not for EntityFilterTuple<A, S, C>
where
    A: std::ops::Not,
    S: std::ops::Not,
    C: std::ops::Not,
{
    type Output = EntityFilterTuple<A::Output, S::Output, C::Output>;

    #[inline]
    fn not(self) -> Self::Output {
        EntityFilterTuple {
            arch_filter: !self.arch_filter,
            chunkset_filter: !self.chunkset_filter,
            chunk_filter: !self.chunk_filter,
        }
    }
}

impl<'a, A1, S1, C1, A2, S2, C2> std::ops::BitAnd<EntityFilterTuple<A2, S2, C2>>
    for EntityFilterTuple<A1, S1, C1>
where
    A1: std::ops::BitAnd<A2>,
    S1: std::ops::BitAnd<S2>,
    C1: std::ops::BitAnd<C2>,
{
    type Output = EntityFilterTuple<A1::Output, S1::Output, C1::Output>;

    #[inline]
    fn bitand(self, rhs: EntityFilterTuple<A2, S2, C2>) -> Self::Output {
        EntityFilterTuple {
            arch_filter: self.arch_filter & rhs.arch_filter,
            chunkset_filter: self.chunkset_filter & rhs.chunkset_filter,
            chunk_filter: self.chunk_filter & rhs.chunk_filter,
        }
    }
}

impl<'a, A1, S1, C1, A2, S2, C2> std::ops::BitOr<EntityFilterTuple<A2, S2, C2>>
    for EntityFilterTuple<A1, S1, C1>
where
    A1: std::ops::BitOr<A2>,
    S1: std::ops::BitOr<S2>,
    C1: std::ops::BitOr<C2>,
{
    type Output = EntityFilterTuple<A1::Output, S1::Output, C1::Output>;

    #[inline]
    fn bitor(self, rhs: EntityFilterTuple<A2, S2, C2>) -> Self::Output {
        EntityFilterTuple {
            arch_filter: self.arch_filter | rhs.arch_filter,
            chunkset_filter: self.chunkset_filter | rhs.chunkset_filter,
            chunk_filter: self.chunk_filter | rhs.chunk_filter,
        }
    }
}

/// An iterator which yields the indexes of archetypes that match a filter.
pub struct FilterArchIter<'a, 'b, F: Filter<ArchetypeFilterData<'a>>> {
    filter: &'b F,
    archetypes: Enumerate<F::Iter>,
}

impl<'a, 'b, F: Filter<ArchetypeFilterData<'a>>> Iterator for FilterArchIter<'a, 'b, F> {
    type Item = ArchetypeIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((i, data)) = self.archetypes.next() {
            if self.filter.is_match(&data).is_pass() {
                return Some(ArchetypeIndex(i));
            }
        }

        None
    }
}

/// An iterator which yields the index of chuinks that match a filter.
pub struct FilterChunkIter<'a, 'b, F: Filter<ChunksetFilterData<'a>>> {
    filter: &'b F,
    chunks: Enumerate<F::Iter>,
}

impl<'a, 'b, F: Filter<ChunksetFilterData<'a>>> Iterator for FilterChunkIter<'a, 'b, F> {
    type Item = SetIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((i, data)) = self.chunks.next() {
            if self.filter.is_match(&data).is_pass() {
                return Some(SetIndex(i));
            }
        }

        None
    }
}

/// An iterator which yields the IDs of chunks that match an entity filter.
pub struct FilterEntityIter<
    'a,
    'b,
    Arch: Filter<ArchetypeFilterData<'a>>,
    Chunk: Filter<ChunksetFilterData<'a>>,
> {
    storage: &'a Storage,
    arch_filter: &'b Arch,
    chunk_filter: &'b Chunk,
    archetypes: Enumerate<Arch::Iter>,
    chunks: Option<(ArchetypeId, Enumerate<Chunk::Iter>)>,
}

impl<'a, 'b, Arch: Filter<ArchetypeFilterData<'a>>, Chunk: Filter<ChunksetFilterData<'a>>> Iterator
    for FilterEntityIter<'a, 'b, Arch, Chunk>
{
    type Item = (ArchetypeId, ChunkIndex);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((arch_id, ref mut chunks)) = self.chunks {
                for (chunk_index, chunk_data) in chunks {
                    if self.chunk_filter.is_match(&chunk_data).is_pass() {
                        return Some((arch_id, ChunkIndex(chunk_index)));
                    }
                }
            }
            loop {
                match self.archetypes.next() {
                    Some((arch_index, arch_data)) => {
                        if self.arch_filter.is_match(&arch_data).is_pass() {
                            self.chunks = {
                                let archetype =
                                    unsafe { self.storage.archetypes().get_unchecked(arch_index) };
                                let data = ChunksetFilterData {
                                    archetype_data: archetype,
                                };

                                Some((archetype.id(), self.chunk_filter.collect(data).enumerate()))
                            };
                            break;
                        }
                    }
                    None => return None,
                }
            }
        }
    }
}

/// A passthrough filter which allows through all elements.
#[derive(Debug, Clone)]
pub struct Passthrough;

impl<'a> Filter<ArchetypeFilterData<'a>> for Passthrough {
    type Iter = Take<Repeat<()>>;

    #[inline]
    fn init(&self) {}

    #[inline]
    fn collect(&self, arch: ArchetypeFilterData<'a>) -> Self::Iter {
        std::iter::repeat(()).take(arch.component_types.len())
    }

    #[inline]
    fn is_match(&self, _: &<Self::Iter as Iterator>::Item) -> Option<bool> { None }
}

impl<'a> Filter<ChunksetFilterData<'a>> for Passthrough {
    type Iter = Take<Repeat<()>>;

    #[inline]
    fn init(&self) {}

    #[inline]
    fn collect(&self, sets: ChunksetFilterData<'a>) -> Self::Iter {
        std::iter::repeat(()).take(sets.archetype_data.len())
    }

    #[inline]
    fn is_match(&self, _: &<Self::Iter as Iterator>::Item) -> Option<bool> { None }
}

impl<'a> Filter<ChunkFilterData<'a>> for Passthrough {
    type Iter = Take<Repeat<()>>;

    #[inline]
    fn init(&self) {}

    #[inline]
    fn collect(&self, chunk: ChunkFilterData<'a>) -> Self::Iter {
        std::iter::repeat(()).take(chunk.chunks.len())
    }

    #[inline]
    fn is_match(&self, _: &<Self::Iter as Iterator>::Item) -> Option<bool> { None }
}

impl std::ops::Not for Passthrough {
    type Output = Passthrough;

    #[inline]
    fn not(self) -> Self::Output { self }
}

impl<'a, Rhs> std::ops::BitAnd<Rhs> for Passthrough {
    type Output = Rhs;

    #[inline]
    fn bitand(self, rhs: Rhs) -> Self::Output { rhs }
}

impl<'a, Rhs> std::ops::BitOr<Rhs> for Passthrough {
    type Output = Rhs;

    #[inline]
    fn bitor(self, rhs: Rhs) -> Self::Output { rhs }
}

#[derive(Debug, Clone)]
pub struct Any;

impl ActiveFilter for Any {}

impl<'a> Filter<ArchetypeFilterData<'a>> for Any {
    type Iter = Take<Repeat<()>>;

    #[inline]
    fn init(&self) {}

    #[inline]
    fn collect(&self, arch: ArchetypeFilterData<'a>) -> Self::Iter {
        std::iter::repeat(()).take(arch.component_types.len())
    }

    #[inline]
    fn is_match(&self, _: &<Self::Iter as Iterator>::Item) -> Option<bool> { Some(true) }
}

impl<'a> Filter<ChunksetFilterData<'a>> for Any {
    type Iter = Take<Repeat<()>>;

    #[inline]
    fn init(&self) {}

    #[inline]
    fn collect(&self, sets: ChunksetFilterData<'a>) -> Self::Iter {
        std::iter::repeat(()).take(sets.archetype_data.len())
    }

    #[inline]
    fn is_match(&self, _: &<Self::Iter as Iterator>::Item) -> Option<bool> { Some(true) }
}

impl<'a> Filter<ChunkFilterData<'a>> for Any {
    type Iter = Take<Repeat<()>>;

    #[inline]
    fn init(&self) {}

    #[inline]
    fn collect(&self, chunk: ChunkFilterData<'a>) -> Self::Iter {
        std::iter::repeat(()).take(chunk.chunks.len())
    }

    #[inline]
    fn is_match(&self, _: &<Self::Iter as Iterator>::Item) -> Option<bool> { Some(true) }
}

impl<Rhs: ActiveFilter> std::ops::BitAnd<Rhs> for Any {
    type Output = Rhs;

    #[inline]
    fn bitand(self, rhs: Rhs) -> Self::Output { rhs }
}

impl std::ops::BitAnd<Passthrough> for Any {
    type Output = Self;

    #[inline]
    fn bitand(self, _: Passthrough) -> Self::Output { self }
}

impl<Rhs: ActiveFilter> std::ops::BitOr<Rhs> for Any {
    type Output = Self;

    #[inline]
    fn bitor(self, _: Rhs) -> Self::Output { self }
}

impl std::ops::BitOr<Passthrough> for Any {
    type Output = Self;

    #[inline]
    fn bitor(self, _: Passthrough) -> Self::Output { self }
}

/// A filter which negates `F`.
#[derive(Debug, Clone)]
pub struct Not<F> {
    pub filter: F,
}

impl<F> ActiveFilter for Not<F> {}

impl<'a, T: Copy, F: Filter<T>> Filter<T> for Not<F> {
    type Iter = F::Iter;

    #[inline]
    fn init(&self) { self.filter.init(); }

    #[inline]
    fn collect(&self, source: T) -> Self::Iter { self.filter.collect(source) }

    #[inline]
    fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
        self.filter.is_match(item).map(|x| !x)
    }
}

impl<'a, F, Rhs: ActiveFilter> std::ops::BitAnd<Rhs> for Not<F> {
    type Output = And<(Self, Rhs)>;

    #[inline]
    fn bitand(self, rhs: Rhs) -> Self::Output {
        And {
            filters: (self, rhs),
        }
    }
}

impl<'a, F> std::ops::BitAnd<Passthrough> for Not<F> {
    type Output = Self;

    #[inline]
    fn bitand(self, _: Passthrough) -> Self::Output { self }
}

impl<'a, F, Rhs: ActiveFilter> std::ops::BitOr<Rhs> for Not<F> {
    type Output = Or<(Self, Rhs)>;

    #[inline]
    fn bitor(self, rhs: Rhs) -> Self::Output {
        Or {
            filters: (self, rhs),
        }
    }
}

impl<'a, F> std::ops::BitOr<Passthrough> for Not<F> {
    type Output = Self;

    #[inline]
    fn bitor(self, _: Passthrough) -> Self::Output { self }
}

/// A filter which requires all filters within `T` match.
#[derive(Debug, Clone)]
pub struct And<T> {
    pub filters: T,
}

impl<T> ActiveFilter for And<(T,)> {}

impl<'a, T: Copy, F: Filter<T>> Filter<T> for And<(F,)> {
    type Iter = F::Iter;

    #[inline]
    fn init(&self) { self.filters.0.init(); }

    #[inline]
    fn collect(&self, source: T) -> Self::Iter { self.filters.0.collect(source) }

    #[inline]
    fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
        self.filters.0.is_match(item)
    }
}

impl<T> std::ops::Not for And<(T,)> {
    type Output = Not<Self>;

    #[inline]
    fn not(self) -> Self::Output { Not { filter: self } }
}

impl<T, Rhs: ActiveFilter> std::ops::BitAnd<Rhs> for And<(T,)> {
    type Output = And<(T, Rhs)>;

    #[inline]
    fn bitand(self, rhs: Rhs) -> Self::Output {
        And {
            filters: (self.filters.0, rhs),
        }
    }
}

impl<T> std::ops::BitAnd<Passthrough> for And<(T,)> {
    type Output = Self;

    #[inline]
    fn bitand(self, _: Passthrough) -> Self::Output { self }
}

impl<T, Rhs: ActiveFilter> std::ops::BitOr<Rhs> for And<(T,)> {
    type Output = Or<(Self, Rhs)>;

    #[inline]
    fn bitor(self, rhs: Rhs) -> Self::Output {
        Or {
            filters: (self, rhs),
        }
    }
}

impl<T> std::ops::BitOr<Passthrough> for And<(T,)> {
    type Output = Self;

    #[inline]
    fn bitor(self, _: Passthrough) -> Self::Output { self }
}

macro_rules! recursive_zip {
    (@value $first:expr, $($rest:expr),*) => { FissileZip::new($first, recursive_zip!(@value $($rest),*)) };
    (@value $last:expr) => { $last };
    (@type $first:ty, $($rest:ty),*) => { FissileZip<$first, recursive_zip!(@type $($rest),*)> };
    (@type $last:ty) => { $last };
    (@unzip $first:ident, $($rest:ident),*) => { ($first, recursive_zip!(@unzip $($rest),*)) };
    (@unzip $last:ident) => { $last };
}

macro_rules! impl_and_filter {
    ( $( $ty: ident => $ty2: ident ),* ) => {
        impl<$( $ty ),*> ActiveFilter for And<($( $ty, )*)> {}

        impl<'a, T: Copy, $( $ty: Filter<T> ),*> Filter<T> for And<($( $ty, )*)> {
            // type Iter = crate::zip::Zip<( $( $ty::Iter ),* )>;
            type Iter = recursive_zip!(@type $($ty::Iter),*);

            #[inline]
            fn init(&self) {
                #![allow(non_snake_case)]
                let ($( $ty, )*) = &self.filters;
                $( $ty.init(); )*
            }

            fn collect(&self, source: T) -> Self::Iter {
                #![allow(non_snake_case)]
                let ($( $ty, )*) = &self.filters;
                // let iters = (
                //     $( $ty.collect(source) ),*
                // );
                // crate::zip::multizip(iters)
                recursive_zip!(@value $($ty.collect(source)),*)
            }

            #[inline]
            fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
                #![allow(non_snake_case)]
                let ($( $ty, )*) = &self.filters;
                // let ($( $ty2, )*) = item;
                let recursive_zip!(@unzip $($ty2),*) = item;
                let mut result: Option<bool> = None;
                $( result = result.coalesce_and($ty.is_match($ty2)); )*
                result
            }
        }

        impl<$( $ty ),*> std::ops::Not for And<($( $ty, )*)> {
            type Output = Not<Self>;

            #[inline]
            fn not(self) -> Self::Output {
                Not { filter: self }
            }
        }

        impl<$( $ty ),*, Rhs: ActiveFilter> std::ops::BitAnd<Rhs> for And<($( $ty, )*)> {
            type Output = And<($( $ty, )* Rhs)>;

            #[inline]
            fn bitand(self, rhs: Rhs) -> Self::Output {
                #![allow(non_snake_case)]
                let ($( $ty, )*) = self.filters;
                And {
                    filters: ($( $ty, )* rhs),
                }
            }
        }

        impl<$( $ty ),*> std::ops::BitAnd<Passthrough> for And<($( $ty, )*)> {
            type Output = Self;

            #[inline]
            fn bitand(self, _: Passthrough) -> Self::Output {
                self
            }
        }

        impl<$( $ty ),*, Rhs: ActiveFilter> std::ops::BitOr<Rhs> for And<($( $ty, )*)> {
            type Output = Or<(Self, Rhs)>;

            #[inline]
            fn bitor(self, rhs: Rhs) -> Self::Output {
                Or {
                    filters: (self, rhs),
                }
            }
        }

        impl<$( $ty ),*> std::ops::BitOr<Passthrough> for And<($( $ty, )*)> {
            type Output = Self;

            #[inline]
            fn bitor(self, _: Passthrough) -> Self::Output {
                self
            }
        }
    }
}

impl_and_filter!(A => a, B => b);
impl_and_filter!(A => a, B => b, C => c);
impl_and_filter!(A => a, B => b, C => c, D => d);
impl_and_filter!(A => a, B => b, C => c, D => d, E => e);
impl_and_filter!(A => a, B => b, C => c, D => d, E => e, F => f);
impl_and_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g);
impl_and_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h);
impl_and_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i);
impl_and_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j);
impl_and_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k);
impl_and_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l);

/// A filter which requires that any filter within `T` match.
#[derive(Debug, Clone)]
pub struct Or<T> {
    pub filters: T,
}

macro_rules! impl_or_filter {
    ( $( $ty: ident => $ty2: ident ),* ) => {
        impl<$( $ty ),*> ActiveFilter for Or<($( $ty, )*)> {}

        impl<'a, T: Copy, $( $ty: Filter<T> ),*> Filter<T> for Or<($( $ty, )*)> {
            // type Iter = crate::zip::Zip<( $( $ty::Iter ),* )>;
            type Iter = recursive_zip!(@type $($ty::Iter),*);

            #[inline]
            fn init(&self) {
                #![allow(non_snake_case)]
                let ($( $ty, )*) = &self.filters;
                $( $ty.init(); )*
            }

            fn collect(&self, source: T) -> Self::Iter {
                #![allow(non_snake_case)]
                let ($( $ty, )*) = &self.filters;
                // let iters = (
                //     $( $ty.collect(source) ),*
                // );
                // crate::zip::multizip(iters)
                recursive_zip!(@value $($ty.collect(source)),*)
            }

            #[inline]
            fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
                #![allow(non_snake_case)]
                let ($( $ty, )*) = &self.filters;
                // let ($( $ty2, )*) = item;
                let recursive_zip!(@unzip $($ty2),*) = item;
                let mut result: Option<bool> = None;
                $( result = result.coalesce_or($ty.is_match($ty2)); )*
                result
            }
        }

        impl<$( $ty ),*> std::ops::Not for Or<($( $ty, )*)> {
            type Output = Not<Self>;

            #[inline]
            fn not(self) -> Self::Output {
                Not { filter: self }
            }
        }

        impl<$( $ty ),*, Rhs: ActiveFilter> std::ops::BitAnd<Rhs> for Or<($( $ty, )*)> {
            type Output = And<(Self, Rhs)>;

            #[inline]
            fn bitand(self, rhs: Rhs) -> Self::Output {
                And {
                    filters: (self, rhs),
                }
            }
        }

        impl<$( $ty ),*> std::ops::BitAnd<Passthrough> for Or<($( $ty, )*)> {
            type Output = Self;

            #[inline]
            fn bitand(self, _: Passthrough) -> Self::Output {
                self
            }
        }

        impl<$( $ty ),*, Rhs: ActiveFilter> std::ops::BitOr<Rhs> for Or<($( $ty, )*)> {
            type Output = Or<($( $ty, )* Rhs)>;

            #[inline]
            fn bitor(self, rhs: Rhs) -> Self::Output {
                #![allow(non_snake_case)]
                let ($( $ty, )*) = self.filters;
                Or {
                    filters: ($( $ty, )* rhs),
                }
            }
        }

        impl<$( $ty ),*> std::ops::BitOr<Passthrough> for Or<($( $ty, )*)> {
            type Output = Self;

            #[inline]
            fn bitor(self, _: Passthrough) -> Self::Output {
                self
            }
        }
    }
}

impl_or_filter!(A => a, B => b);
impl_or_filter!(A => a, B => b, C => c);
impl_or_filter!(A => a, B => b, C => c, D => d);
impl_or_filter!(A => a, B => b, C => c, D => d, E => e);
impl_or_filter!(A => a, B => b, C => c, D => d, E => e, F => f);
impl_or_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g);
impl_or_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h);
impl_or_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i);
impl_or_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j);
impl_or_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k);
impl_or_filter!(A => a, B => b, C => c, D => d, E => e, F => f, G => g, H => h, I => i, J => j, K => k, L => l);

/// A filter qhich requires that all chunks contain entity data components of type `T`.
#[derive(Debug)]
pub struct ComponentFilter<T>(PhantomData<T>);

impl<T: Component> ComponentFilter<T> {
    fn new() -> Self { ComponentFilter(PhantomData) }
}

impl<T> ActiveFilter for ComponentFilter<T> {}

impl<T> Copy for ComponentFilter<T> {}
impl<T> Clone for ComponentFilter<T> {
    fn clone(&self) -> Self { *self }
}

impl<'a, T: Component> Filter<ArchetypeFilterData<'a>> for ComponentFilter<T> {
    type Iter = SliceVecIter<'a, ComponentTypeId>;

    #[inline]
    fn init(&self) {}

    #[inline]
    fn collect(&self, source: ArchetypeFilterData<'a>) -> Self::Iter {
        source.component_types.iter()
    }

    #[inline]
    fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
        Some(item.contains(&ComponentTypeId::of::<T>()))
    }
}

impl<T> std::ops::Not for ComponentFilter<T> {
    type Output = Not<Self>;

    #[inline]
    fn not(self) -> Self::Output { Not { filter: self } }
}

impl<'a, T, Rhs: ActiveFilter> std::ops::BitAnd<Rhs> for ComponentFilter<T> {
    type Output = And<(Self, Rhs)>;

    #[inline]
    fn bitand(self, rhs: Rhs) -> Self::Output {
        And {
            filters: (self, rhs),
        }
    }
}

impl<'a, T> std::ops::BitAnd<Passthrough> for ComponentFilter<T> {
    type Output = Self;

    #[inline]
    fn bitand(self, _: Passthrough) -> Self::Output { self }
}

impl<'a, T, Rhs: ActiveFilter> std::ops::BitOr<Rhs> for ComponentFilter<T> {
    type Output = Or<(Self, Rhs)>;

    #[inline]
    fn bitor(self, rhs: Rhs) -> Self::Output {
        Or {
            filters: (self, rhs),
        }
    }
}

impl<'a, T> std::ops::BitOr<Passthrough> for ComponentFilter<T> {
    type Output = Self;

    #[inline]
    fn bitor(self, _: Passthrough) -> Self::Output { self }
}

/// A filter which requires that all chunks contain shared tag data of type `T`.
#[derive(Debug)]
pub struct TagFilter<T>(PhantomData<T>);

impl<T: Tag> TagFilter<T> {
    fn new() -> Self { TagFilter(PhantomData) }
}

impl<T> ActiveFilter for TagFilter<T> {}

impl<T> Copy for TagFilter<T> {}
impl<T> Clone for TagFilter<T> {
    fn clone(&self) -> Self { *self }
}

impl<'a, T: Tag> Filter<ArchetypeFilterData<'a>> for TagFilter<T> {
    type Iter = SliceVecIter<'a, TagTypeId>;

    #[inline]
    fn init(&self) {}

    #[inline]
    fn collect(&self, source: ArchetypeFilterData<'a>) -> Self::Iter { source.tag_types.iter() }

    #[inline]
    fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
        Some(item.contains(&TagTypeId::of::<T>()))
    }
}

impl<T> std::ops::Not for TagFilter<T> {
    type Output = Not<Self>;

    #[inline]
    fn not(self) -> Self::Output { Not { filter: self } }
}

impl<'a, T, Rhs: ActiveFilter> std::ops::BitAnd<Rhs> for TagFilter<T> {
    type Output = And<(Self, Rhs)>;

    #[inline]
    fn bitand(self, rhs: Rhs) -> Self::Output {
        And {
            filters: (self, rhs),
        }
    }
}

impl<'a, T> std::ops::BitAnd<Passthrough> for TagFilter<T> {
    type Output = Self;

    #[inline]
    fn bitand(self, _: Passthrough) -> Self::Output { self }
}

impl<'a, T, Rhs: ActiveFilter> std::ops::BitOr<Rhs> for TagFilter<T> {
    type Output = Or<(Self, Rhs)>;

    #[inline]
    fn bitor(self, rhs: Rhs) -> Self::Output {
        Or {
            filters: (self, rhs),
        }
    }
}

impl<'a, T> std::ops::BitOr<Passthrough> for TagFilter<T> {
    type Output = Self;

    #[inline]
    fn bitor(self, _: Passthrough) -> Self::Output { self }
}

/// A filter which requires that all chunks contain a specific tag value.
#[derive(Debug)]
pub struct TagValueFilter<'a, T> {
    value: &'a T,
}

impl<'a, T: Tag> TagValueFilter<'a, T> {
    fn new(value: &'a T) -> Self { TagValueFilter { value } }
}

impl<'a, T> ActiveFilter for TagValueFilter<'a, T> {}

impl<'a, T> Copy for TagValueFilter<'a, T> {}
impl<'a, T> Clone for TagValueFilter<'a, T> {
    fn clone(&self) -> Self { *self }
}

impl<'a, 'b, T: Tag> Filter<ChunksetFilterData<'a>> for TagValueFilter<'b, T> {
    type Iter = Iter<'a, T>;

    #[inline]
    fn init(&self) {}

    fn collect(&self, source: ChunksetFilterData<'a>) -> Self::Iter {
        unsafe {
            source
                .archetype_data
                .tags()
                .get(TagTypeId::of::<T>())
                .unwrap()
                .data_slice::<T>()
                .iter()
        }
    }

    #[inline]
    fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
        Some(**item == *self.value)
    }
}

impl<'a, T> std::ops::Not for TagValueFilter<'a, T> {
    type Output = Not<Self>;

    #[inline]
    fn not(self) -> Self::Output { Not { filter: self } }
}

impl<'a, T, Rhs: ActiveFilter> std::ops::BitAnd<Rhs> for TagValueFilter<'a, T> {
    type Output = And<(Self, Rhs)>;

    #[inline]
    fn bitand(self, rhs: Rhs) -> Self::Output {
        And {
            filters: (self, rhs),
        }
    }
}

impl<'a, T> std::ops::BitAnd<Passthrough> for TagValueFilter<'a, T> {
    type Output = Self;

    #[inline]
    fn bitand(self, _: Passthrough) -> Self::Output { self }
}

impl<'a, T, Rhs: ActiveFilter> std::ops::BitOr<Rhs> for TagValueFilter<'a, T> {
    type Output = Or<(Self, Rhs)>;

    #[inline]
    fn bitor(self, rhs: Rhs) -> Self::Output {
        Or {
            filters: (self, rhs),
        }
    }
}

impl<'a, T> std::ops::BitOr<Passthrough> for TagValueFilter<'a, T> {
    type Output = Self;

    #[inline]
    fn bitor(self, _: Passthrough) -> Self::Output { self }
}

/// A filter which requires that entity data of type `T` has changed within the
/// chunk since the last time the filter was executed.
#[derive(Debug)]
pub struct ComponentChangedFilter<T: Component> {
    high_water_mark: AtomicU64,
    version_threshold: AtomicU64,
    phantom: PhantomData<T>,
}

impl<T: Component> ComponentChangedFilter<T> {
    fn new() -> ComponentChangedFilter<T> {
        ComponentChangedFilter {
            high_water_mark: AtomicU64::new(0),
            version_threshold: AtomicU64::new(0),
            phantom: PhantomData,
        }
    }
}

impl<T: Component> ActiveFilter for ComponentChangedFilter<T> {}

impl<T: Component> Clone for ComponentChangedFilter<T> {
    fn clone(&self) -> Self {
        Self {
            high_water_mark: AtomicU64::new(self.high_water_mark.load(Ordering::Relaxed)),
            version_threshold: AtomicU64::new(self.version_threshold.load(Ordering::Relaxed)),
            phantom: PhantomData,
        }
    }
}

impl<'a, T: Component> Filter<ChunkFilterData<'a>> for ComponentChangedFilter<T> {
    type Iter = ComponentChangedState<'a, ComponentStorage>;

    #[inline]
    fn init(&self) {
        let version = self.high_water_mark.load(Ordering::Relaxed);
        let mut threshold = self.version_threshold.load(Ordering::Relaxed);
        if threshold < version {
            loop {
                match self.version_threshold.compare_exchange_weak(
                    threshold,
                    version,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(stored_last_read) => {
                        threshold = stored_last_read;
                        if threshold >= version {
                            // matched version is already considered visited, update no longer needed
                            break;
                        }
                    }
                }
            }
        }
    }

    fn collect(&self, source: ChunkFilterData<'a>) -> Self::Iter {
        let compare_version = self.version_threshold.load(Ordering::Relaxed);
        ComponentChangedState {
            iter: source.chunks.iter(),
            version_threshold: compare_version,
        }
    }

    #[inline]
    fn is_match(&self, item: &<Self::Iter as Iterator>::Item) -> Option<bool> {
        let (version_threshold, storage) = item;

        let components = storage.components(ComponentTypeId::of::<T>());
        if components.is_none() {
            return Some(false);
        }

        let version = components.unwrap().version();
        let mut last_read = self.high_water_mark.load(Ordering::Relaxed);
        if last_read < version {
            loop {
                match self.high_water_mark.compare_exchange_weak(
                    last_read,
                    version,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(stored_last_read) => {
                        last_read = stored_last_read;
                        if last_read >= version {
                            // matched version is already considered visited, update no longer needed
                            break;
                        }
                    }
                }
            }
        }

        if version > *version_threshold {
            Some(true)
        } else {
            Some(false)
        }
    }
}

pub struct ComponentChangedState<'a, T: Component> {
    iter: Iter<'a, T>,
    version_threshold: u64,
}

impl<'a, T: Component> Iterator for ComponentChangedState<'a, T> {
    type Item = (u64, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|c| (self.version_threshold, c))
    }
}

impl<'a, T: Component> std::ops::Not for ComponentChangedFilter<T> {
    type Output = Not<Self>;

    #[inline]
    fn not(self) -> Self::Output { Not { filter: self } }
}

impl<'a, T: Component, Rhs: ActiveFilter> std::ops::BitAnd<Rhs> for ComponentChangedFilter<T> {
    type Output = And<(Self, Rhs)>;

    #[inline]
    fn bitand(self, rhs: Rhs) -> Self::Output {
        And {
            filters: (self, rhs),
        }
    }
}

impl<'a, T: Component> std::ops::BitAnd<Passthrough> for ComponentChangedFilter<T> {
    type Output = Self;

    #[inline]
    fn bitand(self, _: Passthrough) -> Self::Output { self }
}

impl<'a, T: Component, Rhs: ActiveFilter> std::ops::BitOr<Rhs> for ComponentChangedFilter<T> {
    type Output = Or<(Self, Rhs)>;

    #[inline]
    fn bitor(self, rhs: Rhs) -> Self::Output {
        Or {
            filters: (self, rhs),
        }
    }
}

impl<'a, T: Component> std::ops::BitOr<Passthrough> for ComponentChangedFilter<T> {
    type Output = Self;

    #[inline]
    fn bitor(self, _: Passthrough) -> Self::Output { self }
}

#[cfg(test)]
mod test {
    use super::filter_fns::*;
    use crate::prelude::*;

    #[test]
    pub fn create() {
        let _ = tracing_subscriber::fmt::try_init();

        let filter = component::<usize>() | tag_value(&5isize);
        tracing::trace!(?filter);
    }

    #[test]
    fn component_changed_filter() {
        let _ = tracing_subscriber::fmt::try_init();

        let universe = Universe::new();
        let mut world = universe.create_world();

        let entity1 = world.insert((), vec![(1usize,)])[0];
        let entity2 = world.insert((), vec![(2usize, false)])[0];

        let query = <Read<usize>>::query().filter(changed::<usize>());

        assert_eq!(2, query.iter_chunks(&world).collect::<Vec<_>>().len());

        *world.get_component_mut::<usize>(entity1).unwrap() = 3usize;

        assert_eq!(1, query.iter_chunks(&world).collect::<Vec<_>>().len());

        *world.get_component_mut::<usize>(entity1).unwrap() = 4usize;
        *world.get_component_mut::<usize>(entity2).unwrap() = 5usize;

        assert_eq!(2, query.iter_chunks(&world).collect::<Vec<_>>().len());

        *world.get_component_mut::<usize>(entity1).unwrap() = 6usize;
        *world.get_component_mut::<usize>(entity1).unwrap() = 7usize;
        *world.get_component_mut::<usize>(entity2).unwrap() = 8usize;

        assert_eq!(2, query.iter_chunks(&world).collect::<Vec<_>>().len());

        *world.get_component_mut::<usize>(entity2).unwrap() = 6usize;
        *world.get_component_mut::<usize>(entity2).unwrap() = 7usize;
        *world.get_component_mut::<usize>(entity1).unwrap() = 8usize;

        assert_eq!(2, query.iter_chunks(&world).collect::<Vec<_>>().len());
    }
}
