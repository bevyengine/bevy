use super::{QueryData, QueryFilter, ReadOnlyQueryData};
use crate::{
    archetype::{Archetype, ArchetypeEntity, Archetypes},
    bundle::Bundle,
    component::Tick,
    entity::{ContainsEntity, Entities, Entity, EntityEquivalent, EntitySet, EntitySetIterator},
    query::{ArchetypeFilter, DebugCheckedUnwrap, QueryState, StorageId},
    storage::{Table, TableRow, Tables},
    world::{
        unsafe_world_cell::UnsafeWorldCell, EntityMut, EntityMutExcept, EntityRef, EntityRefExcept,
        FilteredEntityMut, FilteredEntityRef,
    },
};
use alloc::vec::Vec;
use core::{
    cmp::Ordering,
    fmt::{self, Debug, Formatter},
    iter::FusedIterator,
    mem::MaybeUninit,
    ops::Range,
};
use nonmax::NonMaxU32;

/// An [`Iterator`] over query results of a [`Query`](crate::system::Query).
///
/// This struct is created by the [`Query::iter`](crate::system::Query::iter) and
/// [`Query::iter_mut`](crate::system::Query::iter_mut) methods.
pub struct QueryIter<'w, 's, D: QueryData, F: QueryFilter> {
    world: UnsafeWorldCell<'w>,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<D, F>,
    cursor: QueryIterationCursor<'w, 's, D, F>,
}

impl<'w, 's, D: QueryData, F: QueryFilter> QueryIter<'w, 's, D, F> {
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    pub(crate) unsafe fn new(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        QueryIter {
            world,
            query_state,
            // SAFETY: We only access table data that has been registered in `query_state`.
            tables: unsafe { &world.storages().tables },
            archetypes: world.archetypes(),
            // SAFETY: The invariants are upheld by the caller.
            cursor: unsafe { QueryIterationCursor::init(world, query_state, last_run, this_run) },
        }
    }

    /// Creates a new separate iterator yielding the same remaining items of the current one.
    /// Advancing the new iterator will not advance the original one, which will resume at the
    /// point it was left at.
    ///
    /// Differently from [`remaining_mut`](QueryIter::remaining_mut) the new iterator does not
    /// borrow from the original one. However it can only be called from an iterator over read only
    /// items.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct ComponentA;
    ///
    /// fn combinations(query: Query<&ComponentA>) {
    ///     let mut iter = query.iter();
    ///     while let Some(a) = iter.next() {
    ///         for b in iter.remaining() {
    ///             // Check every combination (a, b)
    ///         }
    ///     }
    /// }
    /// ```
    pub fn remaining(&self) -> QueryIter<'w, 's, D, F>
    where
        D: ReadOnlyQueryData,
    {
        QueryIter {
            world: self.world,
            tables: self.tables,
            archetypes: self.archetypes,
            query_state: self.query_state,
            cursor: self.cursor.clone(),
        }
    }

    /// Creates a new separate iterator yielding the same remaining items of the current one.
    /// Advancing the new iterator will not advance the original one, which will resume at the
    /// point it was left at.
    ///
    /// This method can be called on iterators over mutable items. However the original iterator
    /// will be borrowed while the new iterator exists and will thus not be usable in that timespan.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct ComponentA;
    ///
    /// fn combinations(mut query: Query<&mut ComponentA>) {
    ///     let mut iter = query.iter_mut();
    ///     while let Some(a) = iter.next() {
    ///         for b in iter.remaining_mut() {
    ///             // Check every combination (a, b)
    ///         }
    ///     }
    /// }
    /// ```
    pub fn remaining_mut(&mut self) -> QueryIter<'_, 's, D, F> {
        QueryIter {
            world: self.world,
            tables: self.tables,
            archetypes: self.archetypes,
            query_state: self.query_state,
            cursor: self.cursor.reborrow(),
        }
    }

    /// Executes the equivalent of [`Iterator::fold`] over a contiguous segment
    /// from a storage.
    ///
    ///  # Safety
    ///  - `range` must be in `[0, storage::entity_count)` or None.
    #[inline]
    pub(super) unsafe fn fold_over_storage_range<B, Func>(
        &mut self,
        mut accum: B,
        func: &mut Func,
        storage: StorageId,
        range: Option<Range<u32>>,
    ) -> B
    where
        Func: FnMut(B, D::Item<'w, 's>) -> B,
    {
        if self.cursor.is_dense {
            // SAFETY: `self.cursor.is_dense` is true, so storage ids are guaranteed to be table ids.
            let table_id = unsafe { storage.table_id };
            // SAFETY: Matched table IDs are guaranteed to still exist.
            let table = unsafe { self.tables.get(table_id).debug_checked_unwrap() };

            let range = range.unwrap_or(0..table.entity_count());
            accum =
                // SAFETY:
                // - The fetched table matches both D and F
                // - caller ensures `range` is within `[0, table.entity_count)`
                // - The if block ensures that the query iteration is dense
                unsafe { self.fold_over_table_range(accum, func, table, range) };
        } else {
            // SAFETY: `self.cursor.is_dense` is false, so storage ids are guaranteed to be archetype ids.
            let archetype_id = unsafe { storage.archetype_id };
            // SAFETY: Matched archetype IDs are guaranteed to still exist.
            let archetype = unsafe { self.archetypes.get(archetype_id).debug_checked_unwrap() };
            // SAFETY: Matched table IDs are guaranteed to still exist.
            let table = unsafe { self.tables.get(archetype.table_id()).debug_checked_unwrap() };

            let range = range.unwrap_or(0..archetype.len());

            // When an archetype and its table have equal entity counts, dense iteration can be safely used.
            // this leverages cache locality to optimize performance.
            if table.entity_count() == archetype.len() {
                accum =
                // SAFETY:
                // - The fetched archetype matches both D and F
                // - The provided archetype and its' table have the same length.
                // - caller ensures `range` is within `[0, archetype.len)`
                // - The if block ensures that the query iteration is not dense.
                unsafe { self.fold_over_dense_archetype_range(accum, func, archetype,range) };
            } else {
                accum =
                // SAFETY:
                // - The fetched archetype matches both D and F
                // - caller ensures `range` is within `[0, archetype.len)`
                // - The if block ensures that the query iteration is not dense.
                unsafe { self.fold_over_archetype_range(accum, func, archetype,range) };
            }
        }
        accum
    }

    /// Executes the equivalent of [`Iterator::fold`] over a contiguous segment
    /// from a table.
    ///
    /// # Safety
    ///  - all `rows` must be in `[0, table.entity_count)`.
    ///  - `table` must match D and F
    ///  - The query iteration must be dense (i.e. `self.query_state.is_dense` must be true).
    #[inline]
    pub(super) unsafe fn fold_over_table_range<B, Func>(
        &mut self,
        mut accum: B,
        func: &mut Func,
        table: &'w Table,
        rows: Range<u32>,
    ) -> B
    where
        Func: FnMut(B, D::Item<'w, 's>) -> B,
    {
        if table.is_empty() {
            return accum;
        }

        D::set_table(&mut self.cursor.fetch, &self.query_state.fetch_state, table);
        F::set_table(
            &mut self.cursor.filter,
            &self.query_state.filter_state,
            table,
        );

        let entities = table.entities();
        for row in rows {
            // SAFETY: Caller assures `row` in range of the current archetype.
            let entity = unsafe { entities.get_unchecked(row as usize) };
            // SAFETY: This is from an exclusive range, so it can't be max.
            let row = unsafe { TableRow::new(NonMaxU32::new_unchecked(row)) };

            // SAFETY: set_table was called prior.
            // Caller assures `row` in range of the current archetype.
            let fetched = unsafe {
                !F::filter_fetch(
                    &self.query_state.filter_state,
                    &mut self.cursor.filter,
                    *entity,
                    row,
                )
            };
            if fetched {
                continue;
            }

            // SAFETY: set_table was called prior.
            // Caller assures `row` in range of the current archetype.
            let item = D::fetch(
                &self.query_state.fetch_state,
                &mut self.cursor.fetch,
                *entity,
                row,
            );

            accum = func(accum, item);
        }
        accum
    }

    /// Executes the equivalent of [`Iterator::fold`] over a contiguous segment
    /// from an archetype.
    ///
    /// # Safety
    ///  - all `indices` must be in `[0, archetype.len())`.
    ///  - `archetype` must match D and F
    ///  - The query iteration must not be dense (i.e. `self.query_state.is_dense` must be false).
    #[inline]
    pub(super) unsafe fn fold_over_archetype_range<B, Func>(
        &mut self,
        mut accum: B,
        func: &mut Func,
        archetype: &'w Archetype,
        indices: Range<u32>,
    ) -> B
    where
        Func: FnMut(B, D::Item<'w, 's>) -> B,
    {
        if archetype.is_empty() {
            return accum;
        }
        let table = self.tables.get(archetype.table_id()).debug_checked_unwrap();
        D::set_archetype(
            &mut self.cursor.fetch,
            &self.query_state.fetch_state,
            archetype,
            table,
        );
        F::set_archetype(
            &mut self.cursor.filter,
            &self.query_state.filter_state,
            archetype,
            table,
        );

        let entities = archetype.entities();
        for index in indices {
            // SAFETY: Caller assures `index` in range of the current archetype.
            let archetype_entity = unsafe { entities.get_unchecked(index as usize) };

            // SAFETY: set_archetype was called prior.
            // Caller assures `index` in range of the current archetype.
            let fetched = unsafe {
                !F::filter_fetch(
                    &self.query_state.filter_state,
                    &mut self.cursor.filter,
                    archetype_entity.id(),
                    archetype_entity.table_row(),
                )
            };
            if fetched {
                continue;
            }

            // SAFETY: set_archetype was called prior, `index` is an archetype index in range of the current archetype
            // Caller assures `index` in range of the current archetype.
            let item = unsafe {
                D::fetch(
                    &self.query_state.fetch_state,
                    &mut self.cursor.fetch,
                    archetype_entity.id(),
                    archetype_entity.table_row(),
                )
            };

            accum = func(accum, item);
        }
        accum
    }

    /// Executes the equivalent of [`Iterator::fold`] over a contiguous segment
    /// from an archetype which has the same entity count as its table.
    ///
    /// # Safety
    ///  - all `indices` must be in `[0, archetype.len())`.
    ///  - `archetype` must match D and F
    ///  - `archetype` must have the same length as its table.
    ///  - The query iteration must not be dense (i.e. `self.query_state.is_dense` must be false).
    #[inline]
    pub(super) unsafe fn fold_over_dense_archetype_range<B, Func>(
        &mut self,
        mut accum: B,
        func: &mut Func,
        archetype: &'w Archetype,
        rows: Range<u32>,
    ) -> B
    where
        Func: FnMut(B, D::Item<'w, 's>) -> B,
    {
        if archetype.is_empty() {
            return accum;
        }
        let table = self.tables.get(archetype.table_id()).debug_checked_unwrap();
        debug_assert!(
            archetype.len() == table.entity_count(),
            "archetype and its table must have the same length. "
        );

        D::set_archetype(
            &mut self.cursor.fetch,
            &self.query_state.fetch_state,
            archetype,
            table,
        );
        F::set_archetype(
            &mut self.cursor.filter,
            &self.query_state.filter_state,
            archetype,
            table,
        );
        let entities = table.entities();
        for row in rows {
            // SAFETY: Caller assures `row` in range of the current archetype.
            let entity = unsafe { *entities.get_unchecked(row as usize) };
            // SAFETY: This is from an exclusive range, so it can't be max.
            let row = unsafe { TableRow::new(NonMaxU32::new_unchecked(row)) };

            // SAFETY: set_table was called prior.
            // Caller assures `row` in range of the current archetype.
            let filter_matched = unsafe {
                F::filter_fetch(
                    &self.query_state.filter_state,
                    &mut self.cursor.filter,
                    entity,
                    row,
                )
            };
            if !filter_matched {
                continue;
            }

            // SAFETY: set_table was called prior.
            // Caller assures `row` in range of the current archetype.
            let item = D::fetch(
                &self.query_state.fetch_state,
                &mut self.cursor.fetch,
                entity,
                row,
            );

            accum = func(accum, item);
        }
        accum
    }

    /// Sorts all query items into a new iterator, using the query lens as a key.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// This uses [`slice::sort`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// # Panics
    ///
    /// This will panic if `next` has been called on `QueryIter` before, unless the underlying `Query` is empty.
    ///
    /// # Examples
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    /// # use std::{ops::{Deref, DerefMut}, iter::Sum};
    /// #
    /// # #[derive(Component)]
    /// # struct PartMarker;
    /// #
    /// # #[derive(Component, PartialEq, Eq, PartialOrd, Ord)]
    /// # struct PartIndex(usize);
    /// #
    /// # #[derive(Component, Clone, Copy)]
    /// # struct PartValue(f32);
    /// #
    /// # impl Deref for PartValue {
    /// #     type Target = f32;
    /// #
    /// #     fn deref(&self) -> &Self::Target {
    /// #         &self.0
    /// #     }
    /// # }
    /// #
    /// # #[derive(Component)]
    /// # struct ParentValue(f32);
    /// #
    /// # impl Deref for ParentValue {
    /// #     type Target = f32;
    /// #
    /// #     fn deref(&self) -> &Self::Target {
    /// #         &self.0
    /// #     }
    /// # }
    /// #
    /// # impl DerefMut for ParentValue {
    /// #     fn deref_mut(&mut self) -> &mut Self::Target {
    /// #         &mut self.0
    /// #     }
    /// # }
    /// #
    /// # #[derive(Component, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// # struct Length(usize);
    /// #
    /// # #[derive(Component, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// # struct Width(usize);
    /// #
    /// # #[derive(Component, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// # struct Height(usize);
    /// #
    /// # #[derive(Component, PartialEq, Eq, PartialOrd, Ord)]
    /// # struct ParentEntity(Entity);
    /// #
    /// # #[derive(Component, Clone, Copy)]
    /// # struct ChildPartCount(usize);
    /// #
    /// # impl Deref for ChildPartCount {
    /// #     type Target = usize;
    /// #
    /// #     fn deref(&self) -> &Self::Target {
    /// #         &self.0
    /// #     }
    /// # }
    /// # let mut world = World::new();
    /// // We can ensure that a query always returns in the same order.
    /// fn system_1(query: Query<(Entity, &PartIndex)>) {
    ///     let parts: Vec<(Entity, &PartIndex)> = query.iter().sort::<&PartIndex>().collect();
    /// }
    ///
    /// // We can freely rearrange query components in the key.
    /// fn system_2(query: Query<(&Length, &Width, &Height), With<PartMarker>>) {
    ///     for (length, width, height) in query.iter().sort::<(&Height, &Length, &Width)>() {
    ///         println!("height: {height:?}, width: {width:?}, length: {length:?}")
    ///     }
    /// }
    ///
    /// // We can sort by Entity without including it in the original Query.
    /// // Here, we match iteration orders between query iterators.
    /// fn system_3(
    ///     part_query: Query<(&PartValue, &ParentEntity)>,
    ///     mut parent_query: Query<(&ChildPartCount, &mut ParentValue)>,
    /// ) {
    ///     let part_values = &mut part_query
    ///         .into_iter()
    ///         .sort::<&ParentEntity>()
    ///         .map(|(&value, parent_entity)| *value);
    ///
    ///     for (&child_count, mut parent_value) in parent_query.iter_mut().sort::<Entity>() {
    ///         **parent_value = part_values.take(*child_count).sum();
    ///     }
    /// }
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems((system_1, system_2, system_3));
    /// # schedule.run(&mut world);
    /// ```
    pub fn sort<L: ReadOnlyQueryData + 'w>(
        self,
    ) -> QuerySortedIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    >
    where
        for<'lw, 'ls> L::Item<'lw, 'ls>: Ord,
    {
        self.sort_impl::<L>(|keyed_query| keyed_query.sort())
    }

    /// Sorts all query items into a new iterator, using the query lens as a key.
    ///
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// This uses [`slice::sort_unstable`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes]..
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// # Panics
    ///
    /// This will panic if `next` has been called on `QueryIter` before, unless the underlying `Query` is empty.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut world = World::new();
    /// #
    /// # #[derive(Component)]
    /// # struct PartMarker;
    /// #
    /// #[derive(Component, PartialEq, Eq, PartialOrd, Ord)]
    /// enum Flying {
    ///     Enabled,
    ///     Disabled
    /// };
    ///
    /// // We perform an unstable sort by a Component with few values.
    /// fn system_1(query: Query<&Flying, With<PartMarker>>) {
    ///     let part_values: Vec<&Flying> = query.iter().sort_unstable::<&Flying>().collect();
    /// }
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems((system_1));
    /// # schedule.run(&mut world);
    /// ```
    pub fn sort_unstable<L: ReadOnlyQueryData + 'w>(
        self,
    ) -> QuerySortedIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    >
    where
        for<'lw, 'ls> L::Item<'lw, 'ls>: Ord,
    {
        self.sort_impl::<L>(|keyed_query| keyed_query.sort_unstable())
    }

    /// Sorts all query items into a new iterator with a comparator function over the query lens.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// This uses [`slice::sort_by`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// # Panics
    ///
    /// This will panic if `next` has been called on `QueryIter` before, unless the underlying `Query` is empty.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use std::ops::Deref;
    /// #
    /// # impl Deref for PartValue {
    /// #     type Target = f32;
    /// #
    /// #     fn deref(&self) -> &Self::Target {
    /// #         &self.0
    /// #     }
    /// # }
    /// #
    /// # let mut world = World::new();
    /// #
    /// #[derive(Component)]
    /// struct PartValue(f32);
    ///
    /// // We can use a cmp function on components do not implement Ord.
    /// fn system_1(query: Query<&PartValue>) {
    ///     // Sort part values according to `f32::total_comp`.
    ///     let part_values: Vec<&PartValue> = query
    ///         .iter()
    ///         .sort_by::<&PartValue>(|value_1, value_2| value_1.total_cmp(*value_2))
    ///         .collect();
    /// }
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems((system_1));
    /// # schedule.run(&mut world);
    /// ```
    pub fn sort_by<L: ReadOnlyQueryData + 'w>(
        self,
        mut compare: impl FnMut(&L::Item<'_, '_>, &L::Item<'_, '_>) -> Ordering,
    ) -> QuerySortedIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    > {
        self.sort_impl::<L>(move |keyed_query| {
            keyed_query.sort_by(|(key_1, _), (key_2, _)| compare(key_1, key_2));
        })
    }

    /// Sorts all query items into a new iterator with a comparator function over the query lens.
    ///
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// This uses [`slice::sort_unstable_by`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// # Panics
    ///
    /// This will panic if `next` has been called on `QueryIter` before, unless the underlying `Query` is empty.
    pub fn sort_unstable_by<L: ReadOnlyQueryData + 'w>(
        self,
        mut compare: impl FnMut(&L::Item<'_, '_>, &L::Item<'_, '_>) -> Ordering,
    ) -> QuerySortedIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    > {
        self.sort_impl::<L>(move |keyed_query| {
            keyed_query.sort_unstable_by(|(key_1, _), (key_2, _)| compare(key_1, key_2));
        })
    }

    /// Sorts all query items into a new iterator with a key extraction function over the query lens.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// This uses [`slice::sort_by_key`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// # Panics
    ///
    /// This will panic if `next` has been called on `QueryIter` before, unless the underlying `Query` is empty.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use std::ops::Deref;
    /// #
    /// # #[derive(Component)]
    /// # struct PartMarker;
    /// #
    /// # impl Deref for PartValue {
    /// #     type Target = f32;
    /// #
    /// #     fn deref(&self) -> &Self::Target {
    /// #         &self.0
    /// #     }
    /// # }
    /// #
    /// # let mut world = World::new();
    /// #
    /// #[derive(Component)]
    /// struct AvailableMarker;
    ///
    /// #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
    /// enum Rarity {
    ///   Common,
    ///   Rare,
    ///   Epic,
    ///   Legendary
    /// };
    ///
    /// #[derive(Component)]
    /// struct PartValue(f32);
    ///
    /// // We can sort with the internals of components that do not implement Ord.
    /// fn system_1(query: Query<(Entity, &PartValue)>) {
    ///     // Sort by the sines of the part values.
    ///     let parts: Vec<(Entity, &PartValue)> = query
    ///         .iter()
    ///         .sort_by_key::<&PartValue, _>(|value| value.sin() as usize)
    ///         .collect();
    /// }
    ///
    /// // We can define our own custom comparison functions over an EntityRef.
    /// fn system_2(query: Query<EntityRef, With<PartMarker>>) {
    ///     // Sort by whether parts are available and their rarity.
    ///     // We want the available legendaries to come first, so we reverse the iterator.
    ///     let parts: Vec<EntityRef> = query.iter()
    ///         .sort_by_key::<EntityRef, _>(|entity_ref| {
    ///             (
    ///                 entity_ref.contains::<AvailableMarker>(),
    ///                 entity_ref.get::<Rarity>().copied()
    ///             )
    ///         })
    ///         .rev()
    ///         .collect();
    /// }
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems((system_1, system_2));
    /// # schedule.run(&mut world);
    /// ```
    pub fn sort_by_key<L: ReadOnlyQueryData + 'w, K>(
        self,
        mut f: impl FnMut(&L::Item<'_, '_>) -> K,
    ) -> QuerySortedIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    >
    where
        K: Ord,
    {
        self.sort_impl::<L>(move |keyed_query| keyed_query.sort_by_key(|(lens, _)| f(lens)))
    }

    /// Sorts all query items into a new iterator with a key extraction function over the query lens.
    ///
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// This uses [`slice::sort_unstable_by_key`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// # Panics
    ///
    /// This will panic if `next` has been called on `QueryIter` before, unless the underlying `Query` is empty.
    pub fn sort_unstable_by_key<L: ReadOnlyQueryData + 'w, K>(
        self,
        mut f: impl FnMut(&L::Item<'_, '_>) -> K,
    ) -> QuerySortedIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    >
    where
        K: Ord,
    {
        self.sort_impl::<L>(move |keyed_query| {
            keyed_query.sort_unstable_by_key(|(lens, _)| f(lens));
        })
    }

    /// Sort all query items into a new iterator with a key extraction function over the query lens.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// This uses [`slice::sort_by_cached_key`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// # Panics
    ///
    /// This will panic if `next` has been called on `QueryIter` before, unless the underlying `Query` is empty.
    pub fn sort_by_cached_key<L: ReadOnlyQueryData + 'w, K>(
        self,
        mut f: impl FnMut(&L::Item<'_, '_>) -> K,
    ) -> QuerySortedIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    >
    where
        K: Ord,
    {
        self.sort_impl::<L>(move |keyed_query| keyed_query.sort_by_cached_key(|(lens, _)| f(lens)))
    }

    /// Shared implementation for the various `sort` methods.
    /// This uses the lens to collect the items for sorting, but delegates the actual sorting to the provided closure.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// # Panics
    ///
    /// This will panic if `next` has been called on `QueryIter` before, unless the underlying `Query` is empty.
    fn sort_impl<L: ReadOnlyQueryData + 'w>(
        self,
        f: impl FnOnce(&mut Vec<(L::Item<'_, '_>, NeutralOrd<Entity>)>),
    ) -> QuerySortedIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    > {
        // On the first successful iteration of `QueryIterationCursor`, `archetype_entities` or `table_entities`
        // will be set to a non-zero value. The correctness of this method relies on this.
        // I.e. this sort method will execute if and only if `next` on `QueryIterationCursor` of a
        // non-empty `QueryIter` has not yet been called. When empty, this sort method will not panic.
        if !self.cursor.archetype_entities.is_empty() || !self.cursor.table_entities.is_empty() {
            panic!("it is not valid to call sort() after next()")
        }

        let world = self.world;

        let query_lens_state = self.query_state.transmute_filtered::<(L, Entity), F>(world);

        // SAFETY:
        // `self.world` has permission to access the required components.
        // The original query iter has not been iterated on, so no items are aliased from it.
        // `QueryIter::new` ensures `world` is the same one used to initialize `query_state`.
        let query_lens = unsafe { query_lens_state.query_unchecked_manual(world) }.into_iter();
        let mut keyed_query: Vec<_> = query_lens
            .map(|(key, entity)| (key, NeutralOrd(entity)))
            .collect();
        f(&mut keyed_query);
        let entity_iter = keyed_query
            .into_iter()
            .map(|(.., entity)| entity.0)
            .collect::<Vec<_>>()
            .into_iter();
        // SAFETY:
        // `self.world` has permission to access the required components.
        // Each lens query item is dropped before the respective actual query item is accessed.
        unsafe {
            QuerySortedIter::new(
                world,
                self.query_state,
                entity_iter,
                world.last_change_tick(),
                world.change_tick(),
            )
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> Iterator for QueryIter<'w, 's, D, F> {
    type Item = D::Item<'w, 's>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY:
        // `tables` and `archetypes` belong to the same world that the cursor was initialized for.
        // `query_state` is the state that was passed to `QueryIterationCursor::init`.
        unsafe {
            self.cursor
                .next(self.tables, self.archetypes, self.query_state)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let max_size = self.cursor.max_remaining(self.tables, self.archetypes);
        let archetype_query = F::IS_ARCHETYPAL;
        let min_size = if archetype_query { max_size } else { 0 };
        (min_size as usize, Some(max_size as usize))
    }

    #[inline]
    fn fold<B, Func>(mut self, init: B, mut func: Func) -> B
    where
        Func: FnMut(B, Self::Item) -> B,
    {
        let mut accum = init;
        // Empty any remaining uniterated values from the current table/archetype
        while self.cursor.current_row != self.cursor.current_len {
            let Some(item) = self.next() else { break };
            accum = func(accum, item);
        }

        for id in self.cursor.storage_id_iter.clone().copied() {
            // SAFETY:
            // - The range(None) is equivalent to [0, storage.entity_count)
            accum = unsafe { self.fold_over_storage_range(accum, &mut func, id, None) };
        }
        accum
    }
}

// This is correct as [`QueryIter`] always returns `None` once exhausted.
impl<'w, 's, D: QueryData, F: QueryFilter> FusedIterator for QueryIter<'w, 's, D, F> {}

// SAFETY: [`QueryIter`] is guaranteed to return every matching entity once and only once.
unsafe impl<'w, 's, F: QueryFilter> EntitySetIterator for QueryIter<'w, 's, Entity, F> {}

// SAFETY: [`QueryIter`] is guaranteed to return every matching entity once and only once.
unsafe impl<'w, 's, F: QueryFilter> EntitySetIterator for QueryIter<'w, 's, EntityRef<'_>, F> {}

// SAFETY: [`QueryIter`] is guaranteed to return every matching entity once and only once.
unsafe impl<'w, 's, F: QueryFilter> EntitySetIterator for QueryIter<'w, 's, EntityMut<'_>, F> {}

// SAFETY: [`QueryIter`] is guaranteed to return every matching entity once and only once.
unsafe impl<'w, 's, F: QueryFilter> EntitySetIterator
    for QueryIter<'w, 's, FilteredEntityRef<'_>, F>
{
}

// SAFETY: [`QueryIter`] is guaranteed to return every matching entity once and only once.
unsafe impl<'w, 's, F: QueryFilter> EntitySetIterator
    for QueryIter<'w, 's, FilteredEntityMut<'_>, F>
{
}

// SAFETY: [`QueryIter`] is guaranteed to return every matching entity once and only once.
unsafe impl<'w, 's, F: QueryFilter, B: Bundle> EntitySetIterator
    for QueryIter<'w, 's, EntityRefExcept<'_, B>, F>
{
}

// SAFETY: [`QueryIter`] is guaranteed to return every matching entity once and only once.
unsafe impl<'w, 's, F: QueryFilter, B: Bundle> EntitySetIterator
    for QueryIter<'w, 's, EntityMutExcept<'_, B>, F>
{
}

impl<'w, 's, D: QueryData, F: QueryFilter> Debug for QueryIter<'w, 's, D, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryIter").finish()
    }
}

impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter> Clone for QueryIter<'w, 's, D, F> {
    fn clone(&self) -> Self {
        self.remaining()
    }
}

/// An [`Iterator`] over sorted query results of a [`Query`](crate::system::Query).
///
/// This struct is created by the [`QueryIter::sort`], [`QueryIter::sort_unstable`],
/// [`QueryIter::sort_by`], [`QueryIter::sort_unstable_by`], [`QueryIter::sort_by_key`],
/// [`QueryIter::sort_unstable_by_key`], and [`QueryIter::sort_by_cached_key`] methods.
pub struct QuerySortedIter<'w, 's, D: QueryData, F: QueryFilter, I>
where
    I: Iterator<Item = Entity>,
{
    entity_iter: I,
    entities: &'w Entities,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    fetch: D::Fetch<'w>,
    query_state: &'s QueryState<D, F>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator> QuerySortedIter<'w, 's, D, F, I>
where
    I: Iterator<Item = Entity>,
{
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    /// - `entity_list` must only contain unique entities or be empty.
    pub(crate) unsafe fn new<EntityList: IntoIterator<IntoIter = I>>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        entity_list: EntityList,
        last_run: Tick,
        this_run: Tick,
    ) -> QuerySortedIter<'w, 's, D, F, I> {
        let fetch = D::init_fetch(world, &query_state.fetch_state, last_run, this_run);
        QuerySortedIter {
            query_state,
            entities: world.entities(),
            archetypes: world.archetypes(),
            // SAFETY: We only access table data that has been registered in `query_state`.
            // This means `world` has permission to access the data we use.
            tables: &world.storages().tables,
            fetch,
            entity_iter: entity_list.into_iter(),
        }
    }

    /// # Safety
    /// `entity` must stem from `self.entity_iter`, and not have been passed before.
    #[inline(always)]
    unsafe fn fetch_next(&mut self, entity: Entity) -> D::Item<'w, 's> {
        let (location, archetype, table);
        // SAFETY:
        // `tables` and `archetypes` belong to the same world that the [`QueryIter`]
        // was initialized for.
        unsafe {
            location = self.entities.get(entity).debug_checked_unwrap();
            archetype = self
                .archetypes
                .get(location.archetype_id)
                .debug_checked_unwrap();
            table = self.tables.get(location.table_id).debug_checked_unwrap();
        }

        // SAFETY: `archetype` is from the world that `fetch` was created for,
        // `fetch_state` is the state that `fetch` was initialized with
        unsafe {
            D::set_archetype(
                &mut self.fetch,
                &self.query_state.fetch_state,
                archetype,
                table,
            );
        }

        // The entity list has already been filtered by the query lens, so we forego filtering here.
        // SAFETY:
        // - set_archetype was called prior, `location.archetype_row` is an archetype index in range of the current archetype
        // - fetch is only called once for each entity.
        unsafe {
            D::fetch(
                &self.query_state.fetch_state,
                &mut self.fetch,
                entity,
                location.table_row,
            )
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator> Iterator
    for QuerySortedIter<'w, 's, D, F, I>
where
    I: Iterator<Item = Entity>,
{
    type Item = D::Item<'w, 's>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.entity_iter.next()?;
        // SAFETY: `entity` is passed from `entity_iter` the first time.
        unsafe { self.fetch_next(entity).into() }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.entity_iter.size_hint()
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator> DoubleEndedIterator
    for QuerySortedIter<'w, 's, D, F, I>
where
    I: DoubleEndedIterator<Item = Entity>,
{
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        let entity = self.entity_iter.next_back()?;
        // SAFETY: `entity` is passed from `entity_iter` the first time.
        unsafe { self.fetch_next(entity).into() }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator> ExactSizeIterator
    for QuerySortedIter<'w, 's, D, F, I>
where
    I: ExactSizeIterator<Item = Entity>,
{
}

// This is correct as [`QuerySortedIter`] returns `None` once exhausted if `entity_iter` does.
impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator> FusedIterator
    for QuerySortedIter<'w, 's, D, F, I>
where
    I: FusedIterator<Item = Entity>,
{
}

// SAFETY:
// `I` stems from a collected and sorted `EntitySetIterator` ([`QueryIter`]).
// Fetching unique entities maintains uniqueness.
unsafe impl<'w, 's, F: QueryFilter, I: Iterator<Item = Entity>> EntitySetIterator
    for QuerySortedIter<'w, 's, Entity, F, I>
{
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator<Item = Entity>> Debug
    for QuerySortedIter<'w, 's, D, F, I>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuerySortedIter").finish()
    }
}

/// An [`Iterator`] over the query items generated from an iterator of [`Entity`]s.
///
/// Items are returned in the order of the provided iterator.
/// Entities that don't match the query are skipped.
///
/// This struct is created by the [`Query::iter_many`](crate::system::Query::iter_many) and [`Query::iter_many_mut`](crate::system::Query::iter_many_mut) methods.
pub struct QueryManyIter<'w, 's, D: QueryData, F: QueryFilter, I: Iterator<Item: EntityEquivalent>>
{
    world: UnsafeWorldCell<'w>,
    entity_iter: I,
    entities: &'w Entities,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    fetch: D::Fetch<'w>,
    filter: F::Fetch<'w>,
    query_state: &'s QueryState<D, F>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator<Item: EntityEquivalent>>
    QueryManyIter<'w, 's, D, F, I>
{
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    pub(crate) unsafe fn new<EntityList: IntoIterator<IntoIter = I>>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        entity_list: EntityList,
        last_run: Tick,
        this_run: Tick,
    ) -> QueryManyIter<'w, 's, D, F, I> {
        let fetch = D::init_fetch(world, &query_state.fetch_state, last_run, this_run);
        let filter = F::init_fetch(world, &query_state.filter_state, last_run, this_run);
        QueryManyIter {
            world,
            query_state,
            entities: world.entities(),
            archetypes: world.archetypes(),
            // SAFETY: We only access table data that has been registered in `query_state`.
            // This means `world` has permission to access the data we use.
            tables: &world.storages().tables,
            fetch,
            filter,
            entity_iter: entity_list.into_iter(),
        }
    }

    /// # Safety
    /// All arguments must stem from the same valid `QueryManyIter`.
    ///
    /// The lifetime here is not restrictive enough for Fetch with &mut access,
    /// as calling `fetch_next_aliased_unchecked` multiple times can produce multiple
    /// references to the same component, leading to unique reference aliasing.
    ///
    /// It is always safe for shared access.
    #[inline(always)]
    unsafe fn fetch_next_aliased_unchecked(
        entity_iter: impl Iterator<Item: EntityEquivalent>,
        entities: &'w Entities,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        fetch: &mut D::Fetch<'w>,
        filter: &mut F::Fetch<'w>,
        query_state: &'s QueryState<D, F>,
    ) -> Option<D::Item<'w, 's>> {
        for entity_borrow in entity_iter {
            let entity = entity_borrow.entity();
            let Some(location) = entities.get(entity) else {
                continue;
            };

            if !query_state
                .matched_archetypes
                .contains(location.archetype_id.index())
            {
                continue;
            }

            let archetype = archetypes.get(location.archetype_id).debug_checked_unwrap();
            let table = tables.get(location.table_id).debug_checked_unwrap();

            // SAFETY: `archetype` is from the world that `fetch/filter` were created for,
            // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
            unsafe {
                D::set_archetype(fetch, &query_state.fetch_state, archetype, table);
            }
            // SAFETY: `table` is from the world that `fetch/filter` were created for,
            // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
            unsafe {
                F::set_archetype(filter, &query_state.filter_state, archetype, table);
            }

            // SAFETY: set_archetype was called prior.
            // `location.archetype_row` is an archetype index row in range of the current archetype, because if it was not, the match above would have `continue`d
            if unsafe {
                F::filter_fetch(
                    &query_state.filter_state,
                    filter,
                    entity,
                    location.table_row,
                )
            } {
                // SAFETY:
                // - set_archetype was called prior, `location.archetype_row` is an archetype index in range of the current archetype
                // - fetch is only called once for each entity.
                return Some(unsafe {
                    D::fetch(&query_state.fetch_state, fetch, entity, location.table_row)
                });
            }
        }
        None
    }

    /// Get next result from the query
    #[inline(always)]
    pub fn fetch_next(&mut self) -> Option<D::Item<'_, 's>> {
        // SAFETY:
        // All arguments stem from self.
        // We are limiting the returned reference to self,
        // making sure this method cannot be called multiple times without getting rid
        // of any previously returned unique references first, thus preventing aliasing.
        unsafe {
            Self::fetch_next_aliased_unchecked(
                &mut self.entity_iter,
                self.entities,
                self.tables,
                self.archetypes,
                &mut self.fetch,
                &mut self.filter,
                self.query_state,
            )
            .map(D::shrink)
        }
    }

    /// Sorts all query items into a new iterator, using the query lens as a key.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// This uses [`slice::sort`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// Unlike the sort methods on [`QueryIter`], this does NOT panic if `next`/`fetch_next` has been
    /// called on [`QueryManyIter`] before.
    ///
    /// # Examples
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    /// # use std::{ops::{Deref, DerefMut}, iter::Sum};
    /// #
    /// # #[derive(Component)]
    /// # struct PartMarker;
    /// #
    /// # #[derive(Component, PartialEq, Eq, PartialOrd, Ord)]
    /// # struct PartIndex(usize);
    /// #
    /// # #[derive(Component, Clone, Copy)]
    /// # struct PartValue(usize);
    /// #
    /// # impl Deref for PartValue {
    /// #     type Target = usize;
    /// #
    /// #     fn deref(&self) -> &Self::Target {
    /// #         &self.0
    /// #     }
    /// # }
    /// #
    /// # impl DerefMut for PartValue {
    /// #     fn deref_mut(&mut self) -> &mut Self::Target {
    /// #         &mut self.0
    /// #     }
    /// # }
    /// #
    /// # #[derive(Component, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// # struct Length(usize);
    /// #
    /// # #[derive(Component, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// # struct Width(usize);
    /// #
    /// # #[derive(Component, Debug, PartialEq, Eq, PartialOrd, Ord)]
    /// # struct Height(usize);
    /// #
    /// # #[derive(Component, PartialEq, Eq, PartialOrd, Ord)]
    /// # struct ParentEntity(Entity);
    /// #
    /// # let mut world = World::new();
    /// // We can ensure that a query always returns in the same order.
    /// fn system_1(query: Query<(Entity, &PartIndex)>) {
    /// #   let entity_list: Vec<Entity> = Vec::new();
    ///     let parts: Vec<(Entity, &PartIndex)> = query.iter_many(entity_list).sort::<&PartIndex>().collect();
    /// }
    ///
    /// // We can freely rearrange query components in the key.
    /// fn system_2(query: Query<(&Length, &Width, &Height), With<PartMarker>>) {
    /// #   let entity_list: Vec<Entity> = Vec::new();
    ///     for (length, width, height) in query.iter_many(entity_list).sort::<(&Height, &Length, &Width)>() {
    ///         println!("height: {height:?}, width: {width:?}, length: {length:?}")
    ///     }
    /// }
    ///
    /// // You can use `fetch_next_back` to obtain mutable references in reverse order.
    /// fn system_3(
    ///     mut query: Query<&mut PartValue>,
    /// ) {
    /// #   let entity_list: Vec<Entity> = Vec::new();
    ///     // We need to collect the internal iterator before iterating mutably
    ///     let mut parent_query_iter = query.iter_many_mut(entity_list)
    ///         .sort::<Entity>();
    ///
    ///     let mut scratch_value = 0;
    ///     while let Some(mut part_value) = parent_query_iter.fetch_next_back()
    ///     {
    ///         // some order-dependent operation, here bitwise XOR
    ///         **part_value ^= scratch_value;
    ///         scratch_value = **part_value;
    ///     }
    /// }
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems((system_1, system_2, system_3));
    /// # schedule.run(&mut world);
    /// ```
    pub fn sort<L: ReadOnlyQueryData + 'w>(
        self,
    ) -> QuerySortedManyIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    >
    where
        for<'lw, 'ls> L::Item<'lw, 'ls>: Ord,
    {
        self.sort_impl::<L>(|keyed_query| keyed_query.sort())
    }

    /// Sorts all query items into a new iterator, using the query lens as a key.
    ///
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// This uses [`slice::sort_unstable`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes]..
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// Unlike the sort methods on [`QueryIter`], this does NOT panic if `next`/`fetch_next` has been
    /// called on [`QueryManyIter`] before.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut world = World::new();
    /// #
    /// # #[derive(Component)]
    /// # struct PartMarker;
    /// #
    /// # let entity_list: Vec<Entity> = Vec::new();
    /// #[derive(Component, PartialEq, Eq, PartialOrd, Ord)]
    /// enum Flying {
    ///     Enabled,
    ///     Disabled
    /// };
    ///
    /// // We perform an unstable sort by a Component with few values.
    /// fn system_1(query: Query<&Flying, With<PartMarker>>) {
    /// #   let entity_list: Vec<Entity> = Vec::new();
    ///     let part_values: Vec<&Flying> = query.iter_many(entity_list).sort_unstable::<&Flying>().collect();
    /// }
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems((system_1));
    /// # schedule.run(&mut world);
    /// ```
    pub fn sort_unstable<L: ReadOnlyQueryData + 'w>(
        self,
    ) -> QuerySortedManyIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    >
    where
        for<'lw, 'ls> L::Item<'lw, 'ls>: Ord,
    {
        self.sort_impl::<L>(|keyed_query| keyed_query.sort_unstable())
    }

    /// Sorts all query items into a new iterator with a comparator function over the query lens.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// This uses [`slice::sort_by`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// Unlike the sort methods on [`QueryIter`], this does NOT panic if `next`/`fetch_next` has been
    /// called on [`QueryManyIter`] before.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use std::ops::Deref;
    /// #
    /// # impl Deref for PartValue {
    /// #     type Target = f32;
    /// #
    /// #     fn deref(&self) -> &Self::Target {
    /// #         &self.0
    /// #     }
    /// # }
    /// #
    /// # let mut world = World::new();
    /// # let entity_list: Vec<Entity> = Vec::new();
    /// #
    /// #[derive(Component)]
    /// struct PartValue(f32);
    ///
    /// // We can use a cmp function on components do not implement Ord.
    /// fn system_1(query: Query<&PartValue>) {
    /// #   let entity_list: Vec<Entity> = Vec::new();
    ///     // Sort part values according to `f32::total_comp`.
    ///     let part_values: Vec<&PartValue> = query
    ///         .iter_many(entity_list)
    ///         .sort_by::<&PartValue>(|value_1, value_2| value_1.total_cmp(*value_2))
    ///         .collect();
    /// }
    /// #
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems((system_1));
    /// # schedule.run(&mut world);
    /// ```
    pub fn sort_by<L: ReadOnlyQueryData + 'w>(
        self,
        mut compare: impl FnMut(&L::Item<'_, '_>, &L::Item<'_, '_>) -> Ordering,
    ) -> QuerySortedManyIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    > {
        self.sort_impl::<L>(move |keyed_query| {
            keyed_query.sort_by(|(key_1, _), (key_2, _)| compare(key_1, key_2));
        })
    }

    /// Sorts all query items into a new iterator with a comparator function over the query lens.
    ///
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// This uses [`slice::sort_unstable_by`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// Unlike the sort methods on [`QueryIter`], this does NOT panic if `next`/`fetch_next` has been
    /// called on [`QueryManyIter`] before.
    pub fn sort_unstable_by<L: ReadOnlyQueryData + 'w>(
        self,
        mut compare: impl FnMut(&L::Item<'_, '_>, &L::Item<'_, '_>) -> Ordering,
    ) -> QuerySortedManyIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    > {
        self.sort_impl::<L>(move |keyed_query| {
            keyed_query.sort_unstable_by(|(key_1, _), (key_2, _)| compare(key_1, key_2));
        })
    }

    /// Sorts all query items into a new iterator with a key extraction function over the query lens.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// This uses [`slice::sort_by_key`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// Unlike the sort methods on [`QueryIter`], this does NOT panic if `next`/`fetch_next` has been
    /// called on [`QueryManyIter`] before.
    ///
    /// # Example
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use std::ops::Deref;
    /// #
    /// # #[derive(Component)]
    /// # struct PartMarker;
    /// #
    /// # impl Deref for PartValue {
    /// #     type Target = f32;
    /// #
    /// #     fn deref(&self) -> &Self::Target {
    /// #         &self.0
    /// #     }
    /// # }
    /// #
    /// # let mut world = World::new();
    /// # let entity_list: Vec<Entity> = Vec::new();
    /// #
    /// #[derive(Component)]
    /// struct AvailableMarker;
    ///
    /// #[derive(Component, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
    /// enum Rarity {
    ///   Common,
    ///   Rare,
    ///   Epic,
    ///   Legendary
    /// };
    ///
    /// #[derive(Component)]
    /// struct PartValue(f32);
    ///
    /// // We can sort with the internals of components that do not implement Ord.
    /// fn system_1(query: Query<(Entity, &PartValue)>) {
    /// #   let entity_list: Vec<Entity> = Vec::new();
    ///     // Sort by the sines of the part values.
    ///     let parts: Vec<(Entity, &PartValue)> = query
    ///         .iter_many(entity_list)
    ///         .sort_by_key::<&PartValue, _>(|value| value.sin() as usize)
    ///         .collect();
    /// }
    ///
    /// // We can define our own custom comparison functions over an EntityRef.
    /// fn system_2(query: Query<EntityRef, With<PartMarker>>) {
    /// #   let entity_list: Vec<Entity> = Vec::new();
    ///     // Sort by whether parts are available and their rarity.
    ///     // We want the available legendaries to come first, so we reverse the iterator.
    ///     let parts: Vec<EntityRef> = query.iter_many(entity_list)
    ///         .sort_by_key::<EntityRef, _>(|entity_ref| {
    ///             (
    ///                 entity_ref.contains::<AvailableMarker>(),
    //                  entity_ref.get::<Rarity>().copied()
    ///             )
    ///         })
    ///         .rev()
    ///         .collect();
    /// }
    /// # let mut schedule = Schedule::default();
    /// # schedule.add_systems((system_1, system_2));
    /// # schedule.run(&mut world);
    /// ```
    pub fn sort_by_key<L: ReadOnlyQueryData + 'w, K>(
        self,
        mut f: impl FnMut(&L::Item<'_, '_>) -> K,
    ) -> QuerySortedManyIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    >
    where
        K: Ord,
    {
        self.sort_impl::<L>(move |keyed_query| keyed_query.sort_by_key(|(lens, _)| f(lens)))
    }

    /// Sorts all query items into a new iterator with a key extraction function over the query lens.
    ///
    /// This sort is unstable (i.e., may reorder equal elements).
    ///
    /// This uses [`slice::sort_unstable_by_key`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// Unlike the sort methods on [`QueryIter`], this does NOT panic if `next`/`fetch_next` has been
    /// called on [`QueryManyIter`] before.
    pub fn sort_unstable_by_key<L: ReadOnlyQueryData + 'w, K>(
        self,
        mut f: impl FnMut(&L::Item<'_, '_>) -> K,
    ) -> QuerySortedManyIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    >
    where
        K: Ord,
    {
        self.sort_impl::<L>(move |keyed_query| {
            keyed_query.sort_unstable_by_key(|(lens, _)| f(lens));
        })
    }

    /// Sort all query items into a new iterator with a key extraction function over the query lens.
    ///
    /// This sort is stable (i.e., does not reorder equal elements).
    ///
    /// This uses [`slice::sort_by_cached_key`] internally.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// Unlike the sort methods on [`QueryIter`], this does NOT panic if `next`/`fetch_next` has been
    /// called on [`QueryManyIter`] before.
    pub fn sort_by_cached_key<L: ReadOnlyQueryData + 'w, K>(
        self,
        mut f: impl FnMut(&L::Item<'_, '_>) -> K,
    ) -> QuerySortedManyIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    >
    where
        K: Ord,
    {
        self.sort_impl::<L>(move |keyed_query| keyed_query.sort_by_cached_key(|(lens, _)| f(lens)))
    }

    /// Shared implementation for the various `sort` methods.
    /// This uses the lens to collect the items for sorting, but delegates the actual sorting to the provided closure.
    ///
    /// Defining the lens works like [`transmute_lens`](crate::system::Query::transmute_lens).
    /// This includes the allowed parameter type changes listed under [allowed transmutes].
    /// However, the lens uses the filter of the original query when present.
    ///
    /// The sort is not cached across system runs.
    ///
    /// [allowed transmutes]: crate::system::Query#allowed-transmutes
    ///
    /// Unlike the sort methods on [`QueryIter`], this does NOT panic if `next`/`fetch_next` has been
    /// called on [`QueryManyIter`] before.
    fn sort_impl<L: ReadOnlyQueryData + 'w>(
        self,
        f: impl FnOnce(&mut Vec<(L::Item<'_, '_>, NeutralOrd<Entity>)>),
    ) -> QuerySortedManyIter<
        'w,
        's,
        D,
        F,
        impl ExactSizeIterator<Item = Entity> + DoubleEndedIterator + FusedIterator + 'w,
    > {
        let world = self.world;

        let query_lens_state = self.query_state.transmute_filtered::<(L, Entity), F>(world);

        // SAFETY:
        // `self.world` has permission to access the required components.
        // The original query iter has not been iterated on, so no items are aliased from it.
        // `QueryIter::new` ensures `world` is the same one used to initialize `query_state`.
        let query_lens = unsafe { query_lens_state.query_unchecked_manual(world) }
            .iter_many_inner(self.entity_iter);
        let mut keyed_query: Vec<_> = query_lens
            .map(|(key, entity)| (key, NeutralOrd(entity)))
            .collect();
        f(&mut keyed_query);
        // Re-collect into a `Vec` to eagerly drop the lens items.
        // They must be dropped before `fetch_next` is called since they may alias
        // with other data items if there are duplicate entities in `entity_iter`.
        let entity_iter = keyed_query
            .into_iter()
            .map(|(.., entity)| entity.0)
            .collect::<Vec<_>>()
            .into_iter();
        // SAFETY:
        // `self.world` has permission to access the required components.
        // Each lens query item is dropped before the respective actual query item is accessed.
        unsafe {
            QuerySortedManyIter::new(
                world,
                self.query_state,
                entity_iter,
                world.last_change_tick(),
                world.change_tick(),
            )
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: DoubleEndedIterator<Item: EntityEquivalent>>
    QueryManyIter<'w, 's, D, F, I>
{
    /// Get next result from the back of the query
    #[inline(always)]
    pub fn fetch_next_back(&mut self) -> Option<D::Item<'_, 's>> {
        // SAFETY:
        // All arguments stem from self.
        // We are limiting the returned reference to self,
        // making sure this method cannot be called multiple times without getting rid
        // of any previously returned unique references first, thus preventing aliasing.
        unsafe {
            Self::fetch_next_aliased_unchecked(
                self.entity_iter.by_ref().rev(),
                self.entities,
                self.tables,
                self.archetypes,
                &mut self.fetch,
                &mut self.filter,
                self.query_state,
            )
            .map(D::shrink)
        }
    }
}

impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, I: Iterator<Item: EntityEquivalent>> Iterator
    for QueryManyIter<'w, 's, D, F, I>
{
    type Item = D::Item<'w, 's>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY:
        // All arguments stem from self.
        // It is safe to alias for ReadOnlyWorldQuery.
        unsafe {
            Self::fetch_next_aliased_unchecked(
                &mut self.entity_iter,
                self.entities,
                self.tables,
                self.archetypes,
                &mut self.fetch,
                &mut self.filter,
                self.query_state,
            )
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, max_size) = self.entity_iter.size_hint();
        (0, max_size)
    }
}

impl<
        'w,
        's,
        D: ReadOnlyQueryData,
        F: QueryFilter,
        I: DoubleEndedIterator<Item: EntityEquivalent>,
    > DoubleEndedIterator for QueryManyIter<'w, 's, D, F, I>
{
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        // SAFETY:
        // All arguments stem from self.
        // It is safe to alias for ReadOnlyWorldQuery.
        unsafe {
            Self::fetch_next_aliased_unchecked(
                self.entity_iter.by_ref().rev(),
                self.entities,
                self.tables,
                self.archetypes,
                &mut self.fetch,
                &mut self.filter,
                self.query_state,
            )
        }
    }
}

// This is correct as [`QueryManyIter`] always returns `None` once exhausted.
impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, I: Iterator<Item: EntityEquivalent>>
    FusedIterator for QueryManyIter<'w, 's, D, F, I>
{
}

// SAFETY: Fetching unique entities maintains uniqueness.
unsafe impl<'w, 's, F: QueryFilter, I: EntitySetIterator> EntitySetIterator
    for QueryManyIter<'w, 's, Entity, F, I>
{
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator<Item: EntityEquivalent>> Debug
    for QueryManyIter<'w, 's, D, F, I>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryManyIter").finish()
    }
}

/// An [`Iterator`] over the query items generated from an iterator of unique [`Entity`]s.
///
/// Items are returned in the order of the provided iterator.
/// Entities that don't match the query are skipped.
///
/// In contrast with [`QueryManyIter`], this allows for mutable iteration without a [`fetch_next`] method.
///
/// This struct is created by the [`iter_many_unique`] and [`iter_many_unique_mut`] methods on [`Query`].
///
/// [`fetch_next`]: QueryManyIter::fetch_next
/// [`iter_many_unique`]: crate::system::Query::iter_many
/// [`iter_many_unique_mut`]: crate::system::Query::iter_many_mut
/// [`Query`]: crate::system::Query
pub struct QueryManyUniqueIter<'w, 's, D: QueryData, F: QueryFilter, I: EntitySetIterator>(
    QueryManyIter<'w, 's, D, F, I>,
);

impl<'w, 's, D: QueryData, F: QueryFilter, I: EntitySetIterator>
    QueryManyUniqueIter<'w, 's, D, F, I>
{
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    pub(crate) unsafe fn new<EntityList: EntitySet<IntoIter = I>>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        entity_list: EntityList,
        last_run: Tick,
        this_run: Tick,
    ) -> QueryManyUniqueIter<'w, 's, D, F, I> {
        QueryManyUniqueIter(QueryManyIter::new(
            world,
            query_state,
            entity_list,
            last_run,
            this_run,
        ))
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: EntitySetIterator> Iterator
    for QueryManyUniqueIter<'w, 's, D, F, I>
{
    type Item = D::Item<'w, 's>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: Entities are guaranteed to be unique, thus do not alias.
        unsafe {
            QueryManyIter::<'w, 's, D, F, I>::fetch_next_aliased_unchecked(
                &mut self.0.entity_iter,
                self.0.entities,
                self.0.tables,
                self.0.archetypes,
                &mut self.0.fetch,
                &mut self.0.filter,
                self.0.query_state,
            )
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, max_size) = self.0.entity_iter.size_hint();
        (0, max_size)
    }
}

// This is correct as [`QueryManyIter`] always returns `None` once exhausted.
impl<'w, 's, D: QueryData, F: QueryFilter, I: EntitySetIterator> FusedIterator
    for QueryManyUniqueIter<'w, 's, D, F, I>
{
}

// SAFETY: Fetching unique entities maintains uniqueness.
unsafe impl<'w, 's, F: QueryFilter, I: EntitySetIterator> EntitySetIterator
    for QueryManyUniqueIter<'w, 's, Entity, F, I>
{
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: EntitySetIterator> Debug
    for QueryManyUniqueIter<'w, 's, D, F, I>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryManyUniqueIter").finish()
    }
}

/// An [`Iterator`] over sorted query results of a [`QueryManyIter`].
///
/// This struct is created by the [`sort`](QueryManyIter), [`sort_unstable`](QueryManyIter),
/// [`sort_by`](QueryManyIter), [`sort_unstable_by`](QueryManyIter), [`sort_by_key`](QueryManyIter),
/// [`sort_unstable_by_key`](QueryManyIter), and [`sort_by_cached_key`](QueryManyIter) methods of [`QueryManyIter`].
pub struct QuerySortedManyIter<'w, 's, D: QueryData, F: QueryFilter, I: Iterator<Item = Entity>> {
    entity_iter: I,
    entities: &'w Entities,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    fetch: D::Fetch<'w>,
    query_state: &'s QueryState<D, F>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator<Item = Entity>>
    QuerySortedManyIter<'w, 's, D, F, I>
{
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    /// - `entity_list` must only contain unique entities or be empty.
    pub(crate) unsafe fn new<EntityList: IntoIterator<IntoIter = I>>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        entity_list: EntityList,
        last_run: Tick,
        this_run: Tick,
    ) -> QuerySortedManyIter<'w, 's, D, F, I> {
        let fetch = D::init_fetch(world, &query_state.fetch_state, last_run, this_run);
        QuerySortedManyIter {
            query_state,
            entities: world.entities(),
            archetypes: world.archetypes(),
            // SAFETY: We only access table data that has been registered in `query_state`.
            // This means `world` has permission to access the data we use.
            tables: &world.storages().tables,
            fetch,
            entity_iter: entity_list.into_iter(),
        }
    }

    /// # Safety
    /// The lifetime here is not restrictive enough for Fetch with &mut access,
    /// as calling `fetch_next_aliased_unchecked` multiple times can produce multiple
    /// references to the same component, leading to unique reference aliasing.
    ///
    /// It is always safe for shared access.
    /// `entity` must stem from `self.entity_iter`, and not have been passed before.
    #[inline(always)]
    unsafe fn fetch_next_aliased_unchecked(&mut self, entity: Entity) -> D::Item<'w, 's> {
        let (location, archetype, table);
        // SAFETY:
        // `tables` and `archetypes` belong to the same world that the [`QueryIter`]
        // was initialized for.
        unsafe {
            location = self.entities.get(entity).debug_checked_unwrap();
            archetype = self
                .archetypes
                .get(location.archetype_id)
                .debug_checked_unwrap();
            table = self.tables.get(location.table_id).debug_checked_unwrap();
        }

        // SAFETY: `archetype` is from the world that `fetch` was created for,
        // `fetch_state` is the state that `fetch` was initialized with
        unsafe {
            D::set_archetype(
                &mut self.fetch,
                &self.query_state.fetch_state,
                archetype,
                table,
            );
        }

        // The entity list has already been filtered by the query lens, so we forego filtering here.
        // SAFETY:
        // - set_archetype was called prior, `location.archetype_row` is an archetype index in range of the current archetype
        // - fetch is only called once for each entity.
        unsafe {
            D::fetch(
                &self.query_state.fetch_state,
                &mut self.fetch,
                entity,
                location.table_row,
            )
        }
    }

    /// Get next result from the query
    #[inline(always)]
    pub fn fetch_next(&mut self) -> Option<D::Item<'_, 's>> {
        let entity = self.entity_iter.next()?;

        // SAFETY:
        // We have collected the entity_iter once to drop all internal lens query item
        // references.
        // We are limiting the returned reference to self,
        // making sure this method cannot be called multiple times without getting rid
        // of any previously returned unique references first, thus preventing aliasing.
        // `entity` is passed from `entity_iter` the first time.
        unsafe { D::shrink(self.fetch_next_aliased_unchecked(entity)).into() }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: DoubleEndedIterator<Item = Entity>>
    QuerySortedManyIter<'w, 's, D, F, I>
{
    /// Get next result from the query
    #[inline(always)]
    pub fn fetch_next_back(&mut self) -> Option<D::Item<'_, 's>> {
        let entity = self.entity_iter.next_back()?;

        // SAFETY:
        // We have collected the entity_iter once to drop all internal lens query item
        // references.
        // We are limiting the returned reference to self,
        // making sure this method cannot be called multiple times without getting rid
        // of any previously returned unique references first, thus preventing aliasing.
        // `entity` is passed from `entity_iter` the first time.
        unsafe { D::shrink(self.fetch_next_aliased_unchecked(entity)).into() }
    }
}

impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, I: Iterator<Item = Entity>> Iterator
    for QuerySortedManyIter<'w, 's, D, F, I>
{
    type Item = D::Item<'w, 's>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.entity_iter.next()?;
        // SAFETY:
        // It is safe to alias for ReadOnlyWorldQuery.
        // `entity` is passed from `entity_iter` the first time.
        unsafe { self.fetch_next_aliased_unchecked(entity).into() }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.entity_iter.size_hint()
    }
}

impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, I: DoubleEndedIterator<Item = Entity>>
    DoubleEndedIterator for QuerySortedManyIter<'w, 's, D, F, I>
{
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        let entity = self.entity_iter.next_back()?;
        // SAFETY:
        // It is safe to alias for ReadOnlyWorldQuery.
        // `entity` is passed from `entity_iter` the first time.
        unsafe { self.fetch_next_aliased_unchecked(entity).into() }
    }
}

impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, I: ExactSizeIterator<Item = Entity>>
    ExactSizeIterator for QuerySortedManyIter<'w, 's, D, F, I>
{
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator<Item = Entity>> Debug
    for QuerySortedManyIter<'w, 's, D, F, I>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuerySortedManyIter").finish()
    }
}

/// An iterator over `K`-sized combinations of query items without repetition.
///
/// A combination is an arrangement of a collection of items where order does not matter.
///
/// `K` is the number of items that make up each subset, and the number of items returned by the iterator.
/// `N` is the number of total entities output by the query.
///
/// For example, given the list [1, 2, 3, 4], where `K` is 2, the combinations without repeats are
/// [1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4].
/// And in this case, `N` would be defined as 4 since the size of the input list is 4.
///
/// The number of combinations depend on how `K` relates to the number of entities matching the [`Query`]:
/// - if `K = N`, only one combination exists,
/// - if `K < N`, there are <sub>N</sub>C<sub>K</sub> combinations (see the [performance section] of `Query`),
/// - if `K > N`, there are no combinations.
///
/// The output combination is not guaranteed to have any order of iteration.
///
/// # Usage
///
/// This type is returned by calling [`Query::iter_combinations`] or [`Query::iter_combinations_mut`].
///
/// It implements [`Iterator`] only if it iterates over read-only query items ([learn more]).
///
/// In the case of mutable query items, it can be iterated by calling [`fetch_next`] in a `while let` loop.
///
/// # Examples
///
/// The following example shows how to traverse the iterator when the query items are read-only.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// fn some_system(query: Query<&ComponentA>) {
///     for [a1, a2] in query.iter_combinations() {
///         // ...
///     }
/// }
/// ```
///
/// The following example shows how `fetch_next` should be called with a `while let` loop to traverse the iterator when the query items are mutable.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// fn some_system(mut query: Query<&mut ComponentA>) {
///     let mut combinations = query.iter_combinations_mut();
///     while let Some([a1, a2]) = combinations.fetch_next() {
///         // ...
///     }
/// }
/// ```
///
/// [`fetch_next`]: Self::fetch_next
/// [learn more]: Self#impl-Iterator
/// [performance section]: crate::system::Query#performance
/// [`Query`]: crate::system::Query
/// [`Query::iter_combinations`]: crate::system::Query::iter_combinations
/// [`Query::iter_combinations_mut`]: crate::system::Query::iter_combinations_mut
pub struct QueryCombinationIter<'w, 's, D: QueryData, F: QueryFilter, const K: usize> {
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<D, F>,
    cursors: [QueryIterationCursor<'w, 's, D, F>; K],
}

impl<'w, 's, D: QueryData, F: QueryFilter, const K: usize> QueryCombinationIter<'w, 's, D, F, K> {
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    pub(crate) unsafe fn new(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        assert!(K != 0, "K should not equal to zero");
        // Initialize array with cursors.
        // There is no FromIterator on arrays, so instead initialize it manually with MaybeUninit

        let mut array: MaybeUninit<[QueryIterationCursor<'w, 's, D, F>; K]> = MaybeUninit::uninit();
        let ptr = array
            .as_mut_ptr()
            .cast::<QueryIterationCursor<'w, 's, D, F>>();
        ptr.write(QueryIterationCursor::init(
            world,
            query_state,
            last_run,
            this_run,
        ));
        for slot in (1..K).map(|offset| ptr.add(offset)) {
            slot.write(QueryIterationCursor::init_empty(
                world,
                query_state,
                last_run,
                this_run,
            ));
        }

        QueryCombinationIter {
            query_state,
            // SAFETY: We only access table data that has been registered in `query_state`.
            tables: unsafe { &world.storages().tables },
            archetypes: world.archetypes(),
            cursors: array.assume_init(),
        }
    }

    /// # Safety
    /// The lifetime here is not restrictive enough for Fetch with &mut access,
    /// as calling `fetch_next_aliased_unchecked` multiple times can produce multiple
    /// references to the same component, leading to unique reference aliasing.
    /// .
    /// It is always safe for shared access.
    #[inline]
    unsafe fn fetch_next_aliased_unchecked(&mut self) -> Option<[D::Item<'w, 's>; K]> {
        // PERF: can speed up the following code using `cursor.remaining()` instead of `next_item.is_none()`
        // when D::IS_ARCHETYPAL && F::IS_ARCHETYPAL
        //
        // let `i` be the index of `c`, the last cursor in `self.cursors` that
        // returns `K-i` or more elements.
        // Make cursor in index `j` for all `j` in `[i, K)` a copy of `c` advanced `j-i+1` times.
        // If no such `c` exists, return `None`
        'outer: for i in (0..K).rev() {
            match self.cursors[i].next(self.tables, self.archetypes, self.query_state) {
                Some(_) => {
                    for j in (i + 1)..K {
                        self.cursors[j] = self.cursors[j - 1].clone();
                        match self.cursors[j].next(self.tables, self.archetypes, self.query_state) {
                            Some(_) => {}
                            None if i > 0 => continue 'outer,
                            None => return None,
                        }
                    }
                    break;
                }
                None if i > 0 => continue,
                None => return None,
            }
        }

        let mut values = MaybeUninit::<[D::Item<'w, 's>; K]>::uninit();

        let ptr = values.as_mut_ptr().cast::<D::Item<'w, 's>>();
        for (offset, cursor) in self.cursors.iter_mut().enumerate() {
            ptr.add(offset)
                .write(cursor.peek_last(self.query_state).unwrap());
        }

        Some(values.assume_init())
    }

    /// Get next combination of queried components
    #[inline]
    pub fn fetch_next(&mut self) -> Option<[D::Item<'_, 's>; K]> {
        // SAFETY: we are limiting the returned reference to self,
        // making sure this method cannot be called multiple times without getting rid
        // of any previously returned unique references first, thus preventing aliasing.
        unsafe {
            self.fetch_next_aliased_unchecked()
                .map(|array| array.map(D::shrink))
        }
    }
}

// Iterator type is intentionally implemented only for read-only access.
// Doing so for mutable references would be unsound, because calling `next`
// multiple times would allow multiple owned references to the same data to exist.
impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, const K: usize> Iterator
    for QueryCombinationIter<'w, 's, D, F, K>
{
    type Item = [D::Item<'w, 's>; K];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Safety: it is safe to alias for ReadOnlyWorldQuery
        unsafe { QueryCombinationIter::fetch_next_aliased_unchecked(self) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // binomial coefficient: (n ; k) = n! / k!(n-k)! = (n*n-1*...*n-k+1) / k!
        // See https://en.wikipedia.org/wiki/Binomial_coefficient
        // See https://blog.plover.com/math/choose.html for implementation
        // It was chosen to reduce overflow potential.
        fn choose(n: usize, k: usize) -> Option<usize> {
            if k > n || n == 0 {
                return Some(0);
            }
            let k = k.min(n - k);
            let ks = 1..=k;
            let ns = (n - k + 1..=n).rev();
            ks.zip(ns)
                .try_fold(1_usize, |acc, (k, n)| Some(acc.checked_mul(n)? / k))
        }
        // sum_i=0..k choose(cursors[i].remaining, k-i)
        let max_combinations = self
            .cursors
            .iter()
            .enumerate()
            .try_fold(0, |acc, (i, cursor)| {
                let n = cursor.max_remaining(self.tables, self.archetypes);
                Some(acc + choose(n as usize, K - i)?)
            });

        let archetype_query = F::IS_ARCHETYPAL;
        let known_max = max_combinations.unwrap_or(usize::MAX);
        let min_combinations = if archetype_query { known_max } else { 0 };
        (min_combinations, max_combinations)
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> ExactSizeIterator for QueryIter<'w, 's, D, F>
where
    F: ArchetypeFilter,
{
    fn len(&self) -> usize {
        self.size_hint().0
    }
}

// This is correct as [`QueryCombinationIter`] always returns `None` once exhausted.
impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, const K: usize> FusedIterator
    for QueryCombinationIter<'w, 's, D, F, K>
{
}

impl<'w, 's, D: QueryData, F: QueryFilter, const K: usize> Debug
    for QueryCombinationIter<'w, 's, D, F, K>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryCombinationIter").finish()
    }
}

struct QueryIterationCursor<'w, 's, D: QueryData, F: QueryFilter> {
    // whether the query iteration is dense or not. Mirrors QueryState's `is_dense` field.
    is_dense: bool,
    storage_id_iter: core::slice::Iter<'s, StorageId>,
    table_entities: &'w [Entity],
    archetype_entities: &'w [ArchetypeEntity],
    fetch: D::Fetch<'w>,
    filter: F::Fetch<'w>,
    // length of the table or length of the archetype, depending on whether both `D`'s and `F`'s fetches are dense
    current_len: u32,
    // either table row or archetype index, depending on whether both `D`'s and `F`'s fetches are dense
    current_row: u32,
}

impl<D: QueryData, F: QueryFilter> Clone for QueryIterationCursor<'_, '_, D, F> {
    fn clone(&self) -> Self {
        Self {
            is_dense: self.is_dense,
            storage_id_iter: self.storage_id_iter.clone(),
            table_entities: self.table_entities,
            archetype_entities: self.archetype_entities,
            fetch: self.fetch.clone(),
            filter: self.filter.clone(),
            current_len: self.current_len,
            current_row: self.current_row,
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> QueryIterationCursor<'w, 's, D, F> {
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    unsafe fn init_empty(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        QueryIterationCursor {
            storage_id_iter: [].iter(),
            ..Self::init(world, query_state, last_run, this_run)
        }
    }

    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    unsafe fn init(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        let fetch = D::init_fetch(world, &query_state.fetch_state, last_run, this_run);
        let filter = F::init_fetch(world, &query_state.filter_state, last_run, this_run);
        QueryIterationCursor {
            fetch,
            filter,
            table_entities: &[],
            archetype_entities: &[],
            storage_id_iter: query_state.matched_storage_ids.iter(),
            is_dense: query_state.is_dense,
            current_len: 0,
            current_row: 0,
        }
    }

    fn reborrow(&mut self) -> QueryIterationCursor<'_, 's, D, F> {
        QueryIterationCursor {
            is_dense: self.is_dense,
            fetch: D::shrink_fetch(self.fetch.clone()),
            filter: F::shrink_fetch(self.filter.clone()),
            table_entities: self.table_entities,
            archetype_entities: self.archetype_entities,
            storage_id_iter: self.storage_id_iter.clone(),
            current_len: self.current_len,
            current_row: self.current_row,
        }
    }

    /// Retrieve item returned from most recent `next` call again.
    ///
    /// # Safety
    /// The result of `next` and any previous calls to `peek_last` with this row must have been
    /// dropped to prevent aliasing mutable references.
    #[inline]
    unsafe fn peek_last(&mut self, query_state: &'s QueryState<D, F>) -> Option<D::Item<'w, 's>> {
        if self.current_row > 0 {
            let index = self.current_row - 1;
            if self.is_dense {
                // SAFETY: This must have been called previously in `next` as `current_row > 0`
                let entity = unsafe { self.table_entities.get_unchecked(index as usize) };
                // SAFETY:
                //  - `set_table` must have been called previously either in `next` or before it.
                //  - `*entity` and `index` are in the current table.
                unsafe {
                    Some(D::fetch(
                        &query_state.fetch_state,
                        &mut self.fetch,
                        *entity,
                        // SAFETY: This is from an exclusive range, so it can't be max.
                        TableRow::new(NonMaxU32::new_unchecked(index)),
                    ))
                }
            } else {
                // SAFETY: This must have been called previously in `next` as `current_row > 0`
                let archetype_entity =
                    unsafe { self.archetype_entities.get_unchecked(index as usize) };
                // SAFETY:
                //  - `set_archetype` must have been called previously either in `next` or before it.
                //  - `archetype_entity.id()` and `archetype_entity.table_row()` are in the current archetype.
                unsafe {
                    Some(D::fetch(
                        &query_state.fetch_state,
                        &mut self.fetch,
                        archetype_entity.id(),
                        archetype_entity.table_row(),
                    ))
                }
            }
        } else {
            None
        }
    }

    /// How many values will this cursor return at most?
    ///
    /// Note that if `F::IS_ARCHETYPAL`, the return value
    /// will be **the exact count of remaining values**.
    fn max_remaining(&self, tables: &'w Tables, archetypes: &'w Archetypes) -> u32 {
        let ids = self.storage_id_iter.clone();
        let remaining_matched: u32 = if self.is_dense {
            // SAFETY: The if check ensures that storage_id_iter stores TableIds
            unsafe { ids.map(|id| tables[id.table_id].entity_count()).sum() }
        } else {
            // SAFETY: The if check ensures that storage_id_iter stores ArchetypeIds
            unsafe { ids.map(|id| archetypes[id.archetype_id].len()).sum() }
        };
        remaining_matched + self.current_len - self.current_row
    }

    // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
    // QueryIter, QueryIterationCursor, QuerySortedIter, QueryManyIter, QuerySortedManyIter, QueryCombinationIter,
    // QueryState::par_fold_init_unchecked_manual, QueryState::par_many_fold_init_unchecked_manual,
    // QueryState::par_many_unique_fold_init_unchecked_manual
    /// # Safety
    /// `tables` and `archetypes` must belong to the same world that the [`QueryIterationCursor`]
    /// was initialized for.
    /// `query_state` must be the same [`QueryState`] that was passed to `init` or `init_empty`.
    #[inline(always)]
    unsafe fn next(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s QueryState<D, F>,
    ) -> Option<D::Item<'w, 's>> {
        if self.is_dense {
            loop {
                // we are on the beginning of the query, or finished processing a table, so skip to the next
                if self.current_row == self.current_len {
                    let table_id = self.storage_id_iter.next()?.table_id;
                    let table = tables.get(table_id).debug_checked_unwrap();
                    if table.is_empty() {
                        continue;
                    }
                    // SAFETY: `table` is from the world that `fetch/filter` were created for,
                    // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                    unsafe {
                        D::set_table(&mut self.fetch, &query_state.fetch_state, table);
                        F::set_table(&mut self.filter, &query_state.filter_state, table);
                    }
                    self.table_entities = table.entities();
                    self.current_len = table.entity_count();
                    self.current_row = 0;
                }

                // SAFETY: set_table was called prior.
                // `current_row` is a table row in range of the current table, because if it was not, then the above would have been executed.
                let entity =
                    unsafe { self.table_entities.get_unchecked(self.current_row as usize) };
                // SAFETY: The row is less than the u32 len, so it must not be max.
                let row = unsafe { TableRow::new(NonMaxU32::new_unchecked(self.current_row)) };
                if !F::filter_fetch(&query_state.filter_state, &mut self.filter, *entity, row) {
                    self.current_row += 1;
                    continue;
                }

                // SAFETY:
                // - set_table was called prior.
                // - `current_row` must be a table row in range of the current table,
                //   because if it was not, then the above would have been executed.
                // - fetch is only called once for each `entity`.
                let item =
                    unsafe { D::fetch(&query_state.fetch_state, &mut self.fetch, *entity, row) };

                self.current_row += 1;
                return Some(item);
            }
        } else {
            loop {
                if self.current_row == self.current_len {
                    let archetype_id = self.storage_id_iter.next()?.archetype_id;
                    let archetype = archetypes.get(archetype_id).debug_checked_unwrap();
                    if archetype.is_empty() {
                        continue;
                    }
                    let table = tables.get(archetype.table_id()).debug_checked_unwrap();
                    // SAFETY: `archetype` and `tables` are from the world that `fetch/filter` were created for,
                    // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                    unsafe {
                        D::set_archetype(
                            &mut self.fetch,
                            &query_state.fetch_state,
                            archetype,
                            table,
                        );
                        F::set_archetype(
                            &mut self.filter,
                            &query_state.filter_state,
                            archetype,
                            table,
                        );
                    }
                    self.archetype_entities = archetype.entities();
                    self.current_len = archetype.len();
                    self.current_row = 0;
                }

                // SAFETY: set_archetype was called prior.
                // `current_row` is an archetype index row in range of the current archetype, because if it was not, then the if above would have been executed.
                let archetype_entity = unsafe {
                    self.archetype_entities
                        .get_unchecked(self.current_row as usize)
                };
                if !F::filter_fetch(
                    &query_state.filter_state,
                    &mut self.filter,
                    archetype_entity.id(),
                    archetype_entity.table_row(),
                ) {
                    self.current_row += 1;
                    continue;
                }

                // SAFETY:
                // - set_archetype was called prior.
                // - `current_row` must be an archetype index row in range of the current archetype,
                //   because if it was not, then the if above would have been executed.
                // - fetch is only called once for each `archetype_entity`.
                let item = unsafe {
                    D::fetch(
                        &query_state.fetch_state,
                        &mut self.fetch,
                        archetype_entity.id(),
                        archetype_entity.table_row(),
                    )
                };
                self.current_row += 1;
                return Some(item);
            }
        }
    }
}

// A wrapper struct that gives its data a neutral ordering.
#[derive(Copy, Clone)]
struct NeutralOrd<T>(T);

impl<T> PartialEq for NeutralOrd<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<T> Eq for NeutralOrd<T> {}

#[expect(
    clippy::non_canonical_partial_ord_impl,
    reason = "`PartialOrd` and `Ord` on this struct must only ever return `Ordering::Equal`, so we prefer clarity"
)]
impl<T> PartialOrd for NeutralOrd<T> {
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        Some(Ordering::Equal)
    }
}

impl<T> Ord for NeutralOrd<T> {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

#[cfg(test)]
#[expect(clippy::print_stdout, reason = "Allowed in tests.")]
mod tests {
    use alloc::vec::Vec;
    use std::println;

    use crate::component::Component;
    use crate::entity::Entity;
    use crate::prelude::World;

    #[derive(Component, Debug, PartialEq, PartialOrd, Clone, Copy)]
    struct A(f32);
    #[derive(Component, Debug, Eq, PartialEq, Clone, Copy)]
    #[component(storage = "SparseSet")]
    struct Sparse(usize);

    #[test]
    fn query_iter_sorts() {
        let mut world = World::new();
        for i in 0..100 {
            world.spawn(A(i as f32));
            world.spawn((A(i as f32), Sparse(i)));
            world.spawn(Sparse(i));
        }

        let mut query = world.query::<Entity>();

        let sort = query.iter(&world).sort::<Entity>().collect::<Vec<_>>();
        assert_eq!(sort.len(), 300);

        let sort_unstable = query
            .iter(&world)
            .sort_unstable::<Entity>()
            .collect::<Vec<_>>();

        let sort_by = query
            .iter(&world)
            .sort_by::<Entity>(Ord::cmp)
            .collect::<Vec<_>>();

        let sort_unstable_by = query
            .iter(&world)
            .sort_unstable_by::<Entity>(Ord::cmp)
            .collect::<Vec<_>>();

        let sort_by_key = query
            .iter(&world)
            .sort_by_key::<Entity, _>(|&e| e)
            .collect::<Vec<_>>();

        let sort_unstable_by_key = query
            .iter(&world)
            .sort_unstable_by_key::<Entity, _>(|&e| e)
            .collect::<Vec<_>>();

        let sort_by_cached_key = query
            .iter(&world)
            .sort_by_cached_key::<Entity, _>(|&e| e)
            .collect::<Vec<_>>();

        let mut sort_v2 = query.iter(&world).collect::<Vec<_>>();
        sort_v2.sort();

        let mut sort_unstable_v2 = query.iter(&world).collect::<Vec<_>>();
        sort_unstable_v2.sort_unstable();

        let mut sort_by_v2 = query.iter(&world).collect::<Vec<_>>();
        sort_by_v2.sort_by(Ord::cmp);

        let mut sort_unstable_by_v2 = query.iter(&world).collect::<Vec<_>>();
        sort_unstable_by_v2.sort_unstable_by(Ord::cmp);

        let mut sort_by_key_v2 = query.iter(&world).collect::<Vec<_>>();
        sort_by_key_v2.sort_by_key(|&e| e);

        let mut sort_unstable_by_key_v2 = query.iter(&world).collect::<Vec<_>>();
        sort_unstable_by_key_v2.sort_unstable_by_key(|&e| e);

        let mut sort_by_cached_key_v2 = query.iter(&world).collect::<Vec<_>>();
        sort_by_cached_key_v2.sort_by_cached_key(|&e| e);

        assert_eq!(sort, sort_v2);
        assert_eq!(sort_unstable, sort_unstable_v2);
        assert_eq!(sort_by, sort_by_v2);
        assert_eq!(sort_unstable_by, sort_unstable_by_v2);
        assert_eq!(sort_by_key, sort_by_key_v2);
        assert_eq!(sort_unstable_by_key, sort_unstable_by_key_v2);
        assert_eq!(sort_by_cached_key, sort_by_cached_key_v2);
    }

    #[test]
    #[should_panic]
    fn query_iter_sort_after_next() {
        let mut world = World::new();
        world.spawn((A(0.),));
        world.spawn((A(1.1),));
        world.spawn((A(2.22),));

        {
            let mut query = world.query::<&A>();
            let mut iter = query.iter(&world);
            println!(
                "archetype_entities: {} table_entities: {} current_len: {} current_row: {}",
                iter.cursor.archetype_entities.len(),
                iter.cursor.table_entities.len(),
                iter.cursor.current_len,
                iter.cursor.current_row
            );
            _ = iter.next();
            println!(
                "archetype_entities: {} table_entities: {} current_len: {} current_row: {}",
                iter.cursor.archetype_entities.len(),
                iter.cursor.table_entities.len(),
                iter.cursor.current_len,
                iter.cursor.current_row
            );
            println!("{}", iter.sort::<Entity>().len());
        }
    }

    #[test]
    #[should_panic]
    fn query_iter_sort_after_next_dense() {
        let mut world = World::new();
        world.spawn((Sparse(11),));
        world.spawn((Sparse(22),));
        world.spawn((Sparse(33),));

        {
            let mut query = world.query::<&Sparse>();
            let mut iter = query.iter(&world);
            println!(
                "before_next_call: archetype_entities: {} table_entities: {} current_len: {} current_row: {}",
                iter.cursor.archetype_entities.len(),
                iter.cursor.table_entities.len(),
                iter.cursor.current_len,
                iter.cursor.current_row
            );
            _ = iter.next();
            println!(
                "after_next_call: archetype_entities: {} table_entities: {} current_len: {} current_row: {}",
                iter.cursor.archetype_entities.len(),
                iter.cursor.table_entities.len(),
                iter.cursor.current_len,
                iter.cursor.current_row
            );
            println!("{}", iter.sort::<Entity>().len());
        }
    }

    #[test]
    fn empty_query_iter_sort_after_next_does_not_panic() {
        let mut world = World::new();
        {
            let mut query = world.query::<(&A, &Sparse)>();
            let mut iter = query.iter(&world);
            println!(
                "before_next_call: archetype_entities: {} table_entities: {} current_len: {} current_row: {}",
                iter.cursor.archetype_entities.len(),
                iter.cursor.table_entities.len(),
                iter.cursor.current_len,
                iter.cursor.current_row
            );
            _ = iter.next();
            println!(
                "after_next_call: archetype_entities: {} table_entities: {} current_len: {} current_row: {}",
                iter.cursor.archetype_entities.len(),
                iter.cursor.table_entities.len(),
                iter.cursor.current_len,
                iter.cursor.current_row
            );
            println!("{}", iter.sort::<Entity>().len());
        }
    }

    #[test]
    fn query_iter_cursor_state_non_empty_after_next() {
        let mut world = World::new();
        world.spawn((A(0.), Sparse(11)));
        world.spawn((A(1.1), Sparse(22)));
        world.spawn((A(2.22), Sparse(33)));
        {
            let mut query = world.query::<(&A, &Sparse)>();
            let mut iter = query.iter(&world);
            println!(
                "before_next_call: archetype_entities: {} table_entities: {} current_len: {} current_row: {}",
                iter.cursor.archetype_entities.len(),
                iter.cursor.table_entities.len(),
                iter.cursor.current_len,
                iter.cursor.current_row
            );
            assert!(iter.cursor.table_entities.len() | iter.cursor.archetype_entities.len() == 0);
            _ = iter.next();
            println!(
                "after_next_call: archetype_entities: {} table_entities: {} current_len: {} current_row: {}",
                iter.cursor.archetype_entities.len(),
                iter.cursor.table_entities.len(),
                iter.cursor.current_len,
                iter.cursor.current_row
            );
            assert!(
                (
                    iter.cursor.table_entities.len(),
                    iter.cursor.archetype_entities.len()
                ) != (0, 0)
            );
        }
    }

    #[test]
    fn query_iter_many_sorts() {
        let mut world = World::new();

        let entity_list: &Vec<_> = &world
            .spawn_batch([A(0.), A(1.), A(2.), A(3.), A(4.)])
            .collect();

        let mut query = world.query::<Entity>();

        let sort = query
            .iter_many(&world, entity_list)
            .sort::<Entity>()
            .collect::<Vec<_>>();

        let sort_unstable = query
            .iter_many(&world, entity_list)
            .sort_unstable::<Entity>()
            .collect::<Vec<_>>();

        let sort_by = query
            .iter_many(&world, entity_list)
            .sort_by::<Entity>(Ord::cmp)
            .collect::<Vec<_>>();

        let sort_unstable_by = query
            .iter_many(&world, entity_list)
            .sort_unstable_by::<Entity>(Ord::cmp)
            .collect::<Vec<_>>();

        let sort_by_key = query
            .iter_many(&world, entity_list)
            .sort_by_key::<Entity, _>(|&e| e)
            .collect::<Vec<_>>();

        let sort_unstable_by_key = query
            .iter_many(&world, entity_list)
            .sort_unstable_by_key::<Entity, _>(|&e| e)
            .collect::<Vec<_>>();

        let sort_by_cached_key = query
            .iter_many(&world, entity_list)
            .sort_by_cached_key::<Entity, _>(|&e| e)
            .collect::<Vec<_>>();

        let mut sort_v2 = query.iter_many(&world, entity_list).collect::<Vec<_>>();
        sort_v2.sort();

        let mut sort_unstable_v2 = query.iter_many(&world, entity_list).collect::<Vec<_>>();
        sort_unstable_v2.sort_unstable();

        let mut sort_by_v2 = query.iter_many(&world, entity_list).collect::<Vec<_>>();
        sort_by_v2.sort_by(Ord::cmp);

        let mut sort_unstable_by_v2 = query.iter_many(&world, entity_list).collect::<Vec<_>>();
        sort_unstable_by_v2.sort_unstable_by(Ord::cmp);

        let mut sort_by_key_v2 = query.iter_many(&world, entity_list).collect::<Vec<_>>();
        sort_by_key_v2.sort_by_key(|&e| e);

        let mut sort_unstable_by_key_v2 = query.iter_many(&world, entity_list).collect::<Vec<_>>();
        sort_unstable_by_key_v2.sort_unstable_by_key(|&e| e);

        let mut sort_by_cached_key_v2 = query.iter_many(&world, entity_list).collect::<Vec<_>>();
        sort_by_cached_key_v2.sort_by_cached_key(|&e| e);

        assert_eq!(sort, sort_v2);
        assert_eq!(sort_unstable, sort_unstable_v2);
        assert_eq!(sort_by, sort_by_v2);
        assert_eq!(sort_unstable_by, sort_unstable_by_v2);
        assert_eq!(sort_by_key, sort_by_key_v2);
        assert_eq!(sort_unstable_by_key, sort_unstable_by_key_v2);
        assert_eq!(sort_by_cached_key, sort_by_cached_key_v2);
    }

    #[test]
    fn query_iter_many_sort_doesnt_panic_after_next() {
        let mut world = World::new();

        let entity_list: &Vec<_> = &world
            .spawn_batch([A(0.), A(1.), A(2.), A(3.), A(4.)])
            .collect();

        let mut query = world.query::<Entity>();
        let mut iter = query.iter_many(&world, entity_list);

        _ = iter.next();

        iter.sort::<Entity>();

        let mut query_2 = world.query::<&mut A>();
        let mut iter_2 = query_2.iter_many_mut(&mut world, entity_list);

        _ = iter_2.fetch_next();

        iter_2.sort::<Entity>();
    }

    // This test should be run with miri to check for UB caused by aliasing.
    // The lens items created during the sort must not be live at the same time as the mutable references returned from the iterator.
    #[test]
    fn query_iter_many_sorts_duplicate_entities_no_ub() {
        #[derive(Component, Ord, PartialOrd, Eq, PartialEq)]
        struct C(usize);

        let mut world = World::new();
        let id = world.spawn(C(10)).id();
        let mut query_state = world.query::<&mut C>();

        {
            let mut query = query_state.iter_many_mut(&mut world, [id, id]).sort::<&C>();
            while query.fetch_next().is_some() {}
        }
        {
            let mut query = query_state
                .iter_many_mut(&mut world, [id, id])
                .sort_unstable::<&C>();
            while query.fetch_next().is_some() {}
        }
        {
            let mut query = query_state
                .iter_many_mut(&mut world, [id, id])
                .sort_by::<&C>(|l, r| Ord::cmp(l, r));
            while query.fetch_next().is_some() {}
        }
        {
            let mut query = query_state
                .iter_many_mut(&mut world, [id, id])
                .sort_unstable_by::<&C>(|l, r| Ord::cmp(l, r));
            while query.fetch_next().is_some() {}
        }
        {
            let mut query = query_state
                .iter_many_mut(&mut world, [id, id])
                .sort_by_key::<&C, _>(|d| d.0);
            while query.fetch_next().is_some() {}
        }
        {
            let mut query = query_state
                .iter_many_mut(&mut world, [id, id])
                .sort_unstable_by_key::<&C, _>(|d| d.0);
            while query.fetch_next().is_some() {}
        }
        {
            let mut query = query_state
                .iter_many_mut(&mut world, [id, id])
                .sort_by_cached_key::<&C, _>(|d| d.0);
            while query.fetch_next().is_some() {}
        }
    }
}
