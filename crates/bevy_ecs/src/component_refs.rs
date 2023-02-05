use crate::change_detection::{Mut, Ref, Ticks, TicksMut};
use crate::component::{Component, ComponentStorage, StorageType};
use crate::entity::Entity;
use crate::query::{DebugCheckedUnwrap, ReadFetch, WriteFetch};
use crate::storage::TableRow;
use bevy_ptr::UnsafeCellDeref;
use std::marker::PhantomData;

/// Trait for defining the types returned by &T and &mut T component queries.
pub trait ComponentRefs<T> {
    type Ref<'w>: ComponentRef<'w, T>;
    type MutRef<'w>: ComponentRefMut<'w, T>;

    fn shrink_ref<'wlong: 'wshort, 'wshort>(item: Self::Ref<'wlong>) -> Self::Ref<'wshort>;
    fn shrink_mut<'wlong: 'wshort, 'wshort>(item: Self::MutRef<'wlong>) -> Self::MutRef<'wshort>;
}

pub trait ComponentRef<'w, T> {
    unsafe fn new(fetch: &ReadFetch<'w, T>, entity: Entity, table_row: TableRow) -> Self;
}
pub trait ComponentRefMut<'w, T> {
    unsafe fn new(fetch: &WriteFetch<'w, T>, entity: Entity, table_row: TableRow) -> Self;
}

/// Pass this to your component derives to override the default type returned by &T
/// and &mut T component queries. This can be useful if you don't need change detection for
/// a component and want to avoid the overhead.
///
/// Example:
/// ```rust
///     // TODO
/// ```
///
/// TODO expand on change detection features lost by using this
/// Use this with caution, because Changed queries will not work
///
/// Todo: Is it possible to disable verify for engine components like Transform? (probably not)
pub struct UnwrappedRefs<T: ?Sized> {
    phantom: PhantomData<T>,
}
impl<T> ComponentRefs<T> for UnwrappedRefs<T>
where
    T: Sized + Component,
{
    type Ref<'w> = &'w T;
    type MutRef<'w> = &'w mut T;

    fn shrink_ref<'wlong: 'wshort, 'wshort>(item: Self::Ref<'wlong>) -> Self::Ref<'wshort> {
        item
    }

    fn shrink_mut<'wlong: 'wshort, 'wshort>(item: Self::MutRef<'wlong>) -> Self::MutRef<'wshort> {
        item
    }
}

pub struct ChangeDetectionRefs<T: ?Sized> {
    phantom: PhantomData<T>,
}
impl<T> ComponentRefs<T> for ChangeDetectionRefs<T>
where
    T: Sized + Component,
{
    type Ref<'w> = &'w T;
    // type Ref<'w> = Ref<'w, T>;
    type MutRef<'w> = Mut<'w, T>;

    fn shrink_ref<'wlong: 'wshort, 'wshort>(item: Self::Ref<'wlong>) -> Self::Ref<'wshort> {
        item
    }

    fn shrink_mut<'wlong: 'wshort, 'wshort>(item: Self::MutRef<'wlong>) -> Self::MutRef<'wshort> {
        item
    }
}

impl<'w, T> ComponentRef<'w, T> for &'w T
where
    T: Component,
{
    unsafe fn new(fetch: &ReadFetch<'w, T>, entity: Entity, table_row: TableRow) -> Self {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => fetch
                .table_data
                .debug_checked_unwrap()
                .0
                .get(table_row.index())
                .deref(),
            StorageType::SparseSet => fetch
                .sparse_set
                .debug_checked_unwrap()
                .get(entity)
                .debug_checked_unwrap()
                .deref(),
        }
    }
}
impl<'w, T> ComponentRef<'w, T> for Ref<'w, T>
where
    T: Component,
{
    unsafe fn new(fetch: &ReadFetch<'w, T>, entity: Entity, table_row: TableRow) -> Self {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let (table_components, added_ticks, changed_ticks) =
                    fetch.table_data.debug_checked_unwrap();
                Ref {
                    value: table_components.get(table_row.index()).deref(),
                    ticks: Ticks {
                        added: added_ticks.get(table_row.index()).deref(),
                        changed: changed_ticks.get(table_row.index()).deref(),
                        change_tick: fetch.change_tick,
                        last_change_tick: fetch.last_change_tick,
                    },
                }
            }
            StorageType::SparseSet => {
                let (component, ticks) = fetch
                    .sparse_set
                    .debug_checked_unwrap()
                    .get_with_ticks(entity)
                    .debug_checked_unwrap();
                Ref {
                    value: component.deref(),
                    ticks: Ticks::from_tick_cells(ticks, fetch.last_change_tick, fetch.change_tick),
                }
            }
        }
    }
}
impl<'w, T> ComponentRefMut<'w, T> for &'w mut T
where
    T: Component,
{
    unsafe fn new(fetch: &WriteFetch<'w, T>, entity: Entity, table_row: TableRow) -> Self {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => fetch
                .table_data
                .debug_checked_unwrap()
                .0
                .get(table_row.index())
                .deref_mut(),
            StorageType::SparseSet => fetch
                .sparse_set
                .debug_checked_unwrap()
                .get_with_ticks(entity)
                .debug_checked_unwrap()
                .0
                .assert_unique()
                .deref_mut(),
        }
    }
}

impl<'w, T> ComponentRefMut<'w, T> for Mut<'w, T>
where
    T: Component,
{
    unsafe fn new(fetch: &WriteFetch<'w, T>, entity: Entity, table_row: TableRow) -> Self {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let (table_components, added_ticks, changed_ticks) =
                    fetch.table_data.debug_checked_unwrap();
                Mut {
                    value: table_components.get(table_row.index()).deref_mut(),
                    ticks: TicksMut {
                        added: added_ticks.get(table_row.index()).deref_mut(),
                        changed: changed_ticks.get(table_row.index()).deref_mut(),
                        change_tick: fetch.change_tick,
                        last_change_tick: fetch.last_change_tick,
                    },
                }
            }
            StorageType::SparseSet => {
                let (component, ticks) = fetch
                    .sparse_set
                    .debug_checked_unwrap()
                    .get_with_ticks(entity)
                    .debug_checked_unwrap();
                Mut {
                    value: component.assert_unique().deref_mut(),
                    ticks: TicksMut::from_tick_cells(
                        ticks,
                        fetch.last_change_tick,
                        fetch.change_tick,
                    ),
                }
            }
        }
    }
}
