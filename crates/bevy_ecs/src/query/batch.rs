use crate::{
    change_detection::{MutBatch, TicksBatch},
    component::{Component, Tick},
    entity::Entity,
    ptr::Batch,
    query::{AnyOf, ChangeTrackers, DebugCheckedUnwrap, WorldQuery},
    storage::TableRow,
};

use bevy_ecs_macros::all_tuples;

use core::marker::PhantomData;

/// The item type returned when a [`WorldQuery`] is iterated over in a batched fashion
pub type QueryBatch<'w, Q, const N: usize> = <Q as WorldQueryBatch<N>>::BatchItem<'w>;

/// The read-only variant of the item type returned when a [`WorldQuery`] is iterated over in a batched fashion
pub type ROQueryBatch<'w, Q, const N: usize> = QueryBatch<'w, <Q as WorldQuery>::ReadOnly, N>;

/// An extension of [`WorldQuery`] for batched queries.
pub trait WorldQueryBatch<const N: usize>: WorldQuery {
    type BatchItem<'w>;

    /// Retrieve a batch of size `N` from the current table.
    /// # Safety
    ///
    /// `table_row_start` is a valid table row index for the current table
    /// `table_row_start` + `N` is a valid table row index for the current table
    /// `table_row_start` is a multiple of `N`
    ///
    /// Must always be called _after_ [`WorldQuery::set_table`].
    unsafe fn fetch_batched<'w>(
        fetch: &mut <Self as WorldQuery>::Fetch<'w>,
        entity_batch: &'w Batch<Entity, N>,
        table_row_start: TableRow,
        len: usize,
    ) -> Self::BatchItem<'w>;
}

impl<const N: usize> WorldQueryBatch<N> for Entity {
    type BatchItem<'w> = &'w Batch<Entity, N>;

    #[inline]
    unsafe fn fetch_batched<'w>(
        _fetch: &mut <Self as WorldQuery>::Fetch<'w>,
        entity_batch: &'w Batch<Entity, N>,
        _table_row_start: TableRow,
        _len: usize,
    ) -> Self::BatchItem<'w> {
        entity_batch
    }
}

impl<T: Component, const N: usize> WorldQueryBatch<N> for &T {
    type BatchItem<'w> = &'w Batch<T, N>;

    #[inline]
    unsafe fn fetch_batched<'w>(
        fetch: &mut <Self as WorldQuery>::Fetch<'w>,
        _entity_batch: &'w Batch<Entity, N>,
        table_row_start: TableRow,
        len: usize,
    ) -> Self::BatchItem<'w> {
        //TODO: when generalized const expresions are stable, want the following:
        //gcd::euclid_usize(ptr::MAX_SIMD_ALIGNMENT, N * core::mem::size_of::<T>());

        let components = fetch.table_components.debug_checked_unwrap();

        components.get_batch_deref::<N>(table_row_start.index(), len)
    }
}

impl<'__w, T: Component, const N: usize> WorldQueryBatch<N> for &'__w mut T {
    type BatchItem<'w> = MutBatch<'w, T, N>;

    #[inline]
    unsafe fn fetch_batched<'w>(
        fetch: &mut <Self as WorldQuery>::Fetch<'w>,
        _entity_batch: &'w Batch<Entity, N>,
        table_row_start: TableRow,
        len: usize,
    ) -> Self::BatchItem<'w> {
        let (table_components, added_ticks, changed_ticks) =
            fetch.table_data.debug_checked_unwrap();

        MutBatch::<T, N> {
            value: table_components.get_batch_deref_mut::<N>(table_row_start.index(), len),
            ticks: TicksBatch {
                // SAFETY: [table_row_start..+batch.len()] is in range
                added_ticks: added_ticks.get_batch_deref_mut::<N>(table_row_start.index(), len),
                changed_ticks: changed_ticks.get_batch_deref_mut::<N>(table_row_start.index(), len),
                change_tick: fetch.change_tick,
                last_change_tick: fetch.last_change_tick,
            },
            _marker: PhantomData,
        }
    }
}

impl<T: WorldQueryBatch<N>, const N: usize> WorldQueryBatch<N> for Option<T> {
    type BatchItem<'w> = Option<QueryBatch<'w, T, N>>;

    #[inline]
    unsafe fn fetch_batched<'w>(
        fetch: &mut <Self as WorldQuery>::Fetch<'w>,
        entity_batch: &'w Batch<Entity, N>,
        table_row_start: TableRow,
        len: usize,
    ) -> Self::BatchItem<'w> {
        if fetch.matches {
            Some(T::fetch_batched(
                &mut fetch.fetch,
                entity_batch,
                table_row_start,
                len,
            ))
        } else {
            None
        }
    }
}

/// A batch of [`ChangeTrackers`].  This is used when performing queries with Change Trackers using the
/// [`Query::for_each_mut_batched`](crate::system::Query::for_each_mut_batched) and [`Query::for_each_batched`](crate::system::Query::for_each_batched) functions.
#[derive(Clone)]
pub struct ChangeTrackersBatch<'a, T, const N: usize> {
    pub(crate) added_ticks: &'a Batch<Tick, N>,
    pub(crate) changed_ticks: &'a Batch<Tick, N>,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
    marker: PhantomData<T>,
}

impl<'a, T: Component, const N: usize> ChangeTrackersBatch<'a, T, N> {
    /// Returns true if this component has been added since the last execution of this system.
    #[inline]
    pub fn is_added(&self) -> bool {
        self.added_ticks
            .iter()
            .any(|x| x.is_older_than(self.last_change_tick, self.change_tick))
    }

    /// Returns true if this component has been changed since the last execution of this system.
    #[inline]
    pub fn is_changed(&self) -> bool {
        self.changed_ticks
            .iter()
            .any(|x| x.is_older_than(self.last_change_tick, self.change_tick))
    }
}

impl<T: Component, const N: usize> WorldQueryBatch<N> for ChangeTrackers<T> {
    type BatchItem<'w> = ChangeTrackersBatch<'w, T, N>;

    #[inline]
    unsafe fn fetch_batched<'w>(
        fetch: &mut <Self as WorldQuery>::Fetch<'w>,
        _entity_batch: &'w Batch<Entity, N>,
        table_row_start: TableRow,
        len: usize,
    ) -> Self::BatchItem<'w> {
        ChangeTrackersBatch {
            added_ticks: {
                let table_ticks = fetch.table_added.debug_checked_unwrap();

                table_ticks.get_batch_deref::<N>(table_row_start.index(), len)
            },
            changed_ticks: {
                let table_ticks = fetch.table_changed.debug_checked_unwrap();

                table_ticks.get_batch_deref::<N>(table_row_start.index(), len)
            },
            marker: PhantomData,
            last_change_tick: fetch.last_change_tick,
            change_tick: fetch.change_tick,
        }
    }
}

macro_rules! impl_tuple_fetch_batched {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        impl<const N: usize, $($name: WorldQueryBatch<N>),*> WorldQueryBatch<N> for ($($name,)*)
        {
            type BatchItem<'w> = ($($name::BatchItem<'w>,)*);

            #[inline]
            unsafe fn fetch_batched<'w>(
                _fetch: &mut <Self as WorldQuery>::Fetch<'w>,
                _entity_batch: &'w Batch<Entity, N>,
                _table_row_start: TableRow,
                _len: usize,
            ) -> Self::BatchItem<'w>
            {
                let ($($name,)*) = _fetch;
                    ($($name::fetch_batched($name, _entity_batch, _table_row_start, _len),)*)
            }
        }
    };
}

macro_rules! impl_anytuple_fetch_batched {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        impl<const N: usize, $($name: WorldQueryBatch<N>),*> WorldQueryBatch<N> for AnyOf<($($name,)*)>
        {
            type BatchItem<'w> = ($(Option<$name::BatchItem<'w>>,)*);

            #[inline]
            unsafe fn fetch_batched<'w>(
                _fetch: &mut <Self as WorldQuery>::Fetch<'w>,
                _entity_batch: &'w Batch<Entity, N>,
                _table_row_start: TableRow,
                _len: usize,
            ) -> <Self as WorldQueryBatch<N>>::BatchItem<'w>
            {
                let ($($name,)*) = _fetch;

                ($(
                    $name.1.then(|| $name::fetch_batched(&mut $name.0, _entity_batch, _table_row_start, _len)),
                )*)

            }
        }

    };
}

all_tuples!(impl_tuple_fetch_batched, 0, 15, F, S);
all_tuples!(impl_anytuple_fetch_batched, 0, 15, F, S);
