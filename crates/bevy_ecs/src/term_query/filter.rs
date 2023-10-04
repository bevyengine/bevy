use bevy_ptr::{ThinSlicePtr, UnsafeCellDeref};
use bevy_utils::all_tuples;

use crate::{
    archetype::Archetype,
    component::{ComponentId, ComponentStorage, StorageType, Tick},
    entity::Entity,
    prelude::{Added, Changed, Component, Or, With, Without, World},
    query::DebugCheckedUnwrap,
    storage::{Table, TableRow},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{
    ComponentPtr, ComponentState, QueryFetch, QueryFetchGroup, Term, TermFilter, TermState,
};

impl<T: Component> QueryFetch for With<T> {
    type Item<'w> = ();
    type ReadOnly = Self;
    const DENSE: bool = true;

    fn init_term(world: &mut World, term: Term) -> Term {
        let component = world.init_component::<T>();
        term.with_id(component)
    }

    unsafe fn init_term_state<'w>(_world: UnsafeWorldCell<'w>, _term: &Term) -> TermState<'w> {
        TermState::empty()
    }

    #[inline(always)]
    unsafe fn filter_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        _entity: Entity,
        _row: TableRow,
    ) -> bool {
        true
    }

    unsafe fn matches_component_set(
        state: &TermState<'_>,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(state.component.as_ref().debug_checked_unwrap().id)
    }

    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        _entity: Entity,
        _row: TableRow,
    ) -> Self::Item<'w> {
    }
}

impl<T: Component> QueryFetch for Without<T> {
    type Item<'w> = ();
    type ReadOnly = Self;
    const DENSE: bool = true;

    fn init_term(world: &mut World, term: Term) -> Term {
        let component = world.init_component::<T>();
        term.with_id(component).with_filter(TermFilter::Without)
    }

    unsafe fn init_term_state<'w>(_world: UnsafeWorldCell<'w>, _term: &Term) -> TermState<'w> {
        TermState::empty()
    }

    #[inline(always)]
    unsafe fn filter_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        _entity: Entity,
        _row: TableRow,
    ) -> bool {
        true
    }

    unsafe fn matches_component_set(
        state: &TermState<'_>,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        !set_contains_id(state.component.as_ref().debug_checked_unwrap().id)
    }

    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        _entity: Entity,
        _row: TableRow,
    ) -> Self::Item<'w> {
    }
}

impl<T: Component> QueryFetch for Added<T> {
    type Item<'w> = bool;
    type ReadOnly = Self;
    const DENSE: bool = match T::Storage::STORAGE_TYPE {
        StorageType::Table => true,
        StorageType::SparseSet => false,
    };

    fn init_term(world: &mut World, term: Term) -> Term {
        let component = world.init_component::<T>();
        term.with_id(component).with_filter(TermFilter::Added)
    }

    unsafe fn init_term_state<'w>(world: UnsafeWorldCell<'w>, term: &Term) -> TermState<'w> {
        let component = term.component.debug_checked_unwrap();
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            TermState::new(ComponentState {
                ptr: None,
                id: component,
                size: 0,
                set: None,
            })
        } else {
            let set = world
                .storages()
                .sparse_sets
                .get(component)
                .debug_checked_unwrap();
            TermState::new(ComponentState {
                ptr: None,
                id: component,
                size: 0,
                set: Some(set),
            })
        }
    }

    /// Adjusts internal state to account for the next [`Table`].
    ///
    /// # Safety
    ///
    /// - `table` must contain the components accessed by `state`
    unsafe fn set_term_table<'w>(state: &mut TermState<'w>, table: &'w Table) {
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            let component = state.component.as_mut().debug_checked_unwrap();
            let column = table.get_column(component.id).debug_checked_unwrap();
            component.ptr = Some(ComponentPtr {
                component: column.get_data_ptr(),
                added: Some(column.get_added_ticks_slice().into()),
                changed: Some(column.get_changed_ticks_slice().into()),
            });
        }
    }

    #[inline(always)]
    unsafe fn filter_term<'w>(
        _world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        row: TableRow,
    ) -> bool {
        let component = state.component.as_ref().debug_checked_unwrap();
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            let ptr = component.ptr.as_ref().debug_checked_unwrap();
            let added = ptr.added.debug_checked_unwrap();
            added
                .get(row.index())
                .deref()
                .is_newer_than(last_run, this_run)
        } else {
            let set = component.set.debug_checked_unwrap();
            let added = set.get_added_tick(entity).debug_checked_unwrap();
            added.deref().is_newer_than(last_run, this_run)
        }
    }

    unsafe fn fetch_term<'w>(
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        row: TableRow,
    ) -> Self::Item<'w> {
        Self::filter_term(world, last_run, this_run, state, entity, row)
    }

    unsafe fn matches_component_set(
        state: &TermState<'_>,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(state.component.as_ref().debug_checked_unwrap().id)
    }
}

impl<T: Component> QueryFetch for Changed<T> {
    type Item<'w> = bool;
    type ReadOnly = Self;
    const DENSE: bool = match T::Storage::STORAGE_TYPE {
        StorageType::Table => true,
        StorageType::SparseSet => false,
    };

    fn init_term(world: &mut World, term: Term) -> Term {
        let component = world.init_component::<T>();
        term.with_id(component).with_filter(TermFilter::Changed)
    }

    unsafe fn init_term_state<'w>(world: UnsafeWorldCell<'w>, term: &Term) -> TermState<'w> {
        let component = term.component.debug_checked_unwrap();
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            TermState::new(ComponentState {
                ptr: None,
                id: component,
                size: 0,
                set: None,
            })
        } else {
            let set = world
                .storages()
                .sparse_sets
                .get(component)
                .debug_checked_unwrap();
            TermState::new(ComponentState {
                ptr: None,
                id: component,
                size: 0,
                set: Some(set),
            })
        }
    }

    /// Adjusts internal state to account for the next [`Table`].
    ///
    /// # Safety
    ///
    /// - `table` must contain the components accessed by `state`
    unsafe fn set_term_table<'w>(state: &mut TermState<'w>, table: &'w Table) {
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            let component = state.component.as_mut().debug_checked_unwrap();
            let column = table.get_column(component.id).debug_checked_unwrap();
            component.ptr = Some(ComponentPtr {
                component: column.get_data_ptr(),
                added: Some(column.get_added_ticks_slice().into()),
                changed: Some(column.get_changed_ticks_slice().into()),
            });
        }
    }

    #[inline(always)]
    unsafe fn filter_term<'w>(
        _world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        row: TableRow,
    ) -> bool {
        let component = state.component.as_ref().debug_checked_unwrap();
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            let ptr = component.ptr.as_ref().debug_checked_unwrap();
            let changed = ptr.changed.debug_checked_unwrap().get(row.index());
            changed.deref().is_newer_than(last_run, this_run)
        } else {
            let set = component.set.debug_checked_unwrap();
            let changed = set.get_changed_tick(entity).debug_checked_unwrap();
            changed.deref().is_newer_than(last_run, this_run)
        }
    }

    unsafe fn fetch_term<'w>(
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        row: TableRow,
    ) -> Self::Item<'w> {
        Self::filter_term(world, last_run, this_run, state, entity, row)
    }

    unsafe fn matches_component_set(
        state: &TermState<'_>,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(state.component.as_ref().debug_checked_unwrap().id)
    }
}

// Blanket implementatinon of [`QueryFetch`] for all tuples of [`QueryFetch`]
macro_rules! or_term_tuple {
    ($($term: ident),*) => {
        impl<$($term: QueryFetchGroup),*> QueryFetchGroup for Or<($($term,)*)> {
            type Item<'w> = bool;
            type ReadOnly = Self;
            const DENSE: bool = true $(&& $term::DENSE)*;
            const SIZE: usize = 0 $(+ $term::SIZE)*;

            #[inline]
            fn init_terms(_world: &mut World, _terms: &mut Vec<Term>, _depth: u8) {
                $(
                    $term::init_terms(_world, _terms, _depth);
                    if let Some(last) = _terms.last_mut() {
                        last.or = true;
                    }
                )*
                if let Some(last) = _terms.last_mut() {
                    last.or = false;
                }
            }

            #[inline]
            unsafe fn init_term_states<'w>(
                _world: UnsafeWorldCell<'w>,
                mut _term: ThinSlicePtr<'_, Term>,
                _state: &mut Vec<TermState<'w>>
            ) {
                $(
                    $term::init_term_states(_world, _term, _state);
                    _term.add($term::SIZE);
                )*
            }

            #[inline]
            unsafe fn set_term_tables<'w>(
                mut _state: ThinSlicePtr<'_, TermState<'w>>,
                _table: &'w Table,
            ) {
                $(
                    if $term::set_matches_states(_state, &|id| _table.has_column(id)) {
                        $term::set_term_tables(_state, _table);
                    }
                    _state.add($term::SIZE);
                )*
            }

            #[inline]
            unsafe fn set_term_archetypes<'w>(
                mut _state: ThinSlicePtr<'_, TermState<'w>>,
                _archetype: &'w Archetype,
                _table: &'w Table,
            ) {
                $(
                    if $term::set_matches_states(_state, &|id| _archetype.contains(id)) {
                        $term::set_term_archetypes(_state, _archetype, _table);
                    }
                    _state.add($term::SIZE);
                )*
            }

            #[allow(clippy::unused_unit)]
            #[inline(always)]
            unsafe fn filter_terms<'w>(
                _world: UnsafeWorldCell<'w>,
                _last_run: Tick,
                _this_run: Tick,
                mut _state: ThinSlicePtr<'_, TermState<'w>>,
                _entity: Entity,
                _row: TableRow
            ) -> bool {
                false $(||
                    {
                        let item = $term::filter_terms(_world, _last_run, _this_run, _state, _entity, _row);
                        _state.add($term::SIZE);
                        item
                    }
                )*
            }

            #[allow(clippy::unused_unit)]
            #[inline(always)]
            unsafe fn fetch_terms<'w>(
                world: UnsafeWorldCell<'w>,
                last_run: Tick,
                this_run: Tick,
                state: ThinSlicePtr<'_, TermState<'w>>,
                entity: Entity,
                row: TableRow
            ) -> bool {
                Self::filter_terms(world, last_run, this_run, state, entity, row)
            }

            #[allow(clippy::unused_unit)]
            #[inline(always)]
            unsafe fn fetch_terms_checked<'w>(
                world: UnsafeWorldCell<'w>,
                last_run: Tick,
                this_run: Tick,
                state: ThinSlicePtr<'_, TermState<'w>>,
                entity: Entity,
                row: TableRow
            ) -> Option<bool> {
                Some(Self::filter_terms(world, last_run, this_run, state, entity, row))
            }

            unsafe fn set_matches_states(
                mut _state: ThinSlicePtr<'_, TermState<'_>>,
                _set_contains_id: &impl Fn(ComponentId) -> bool,
            ) -> bool {
                false $(|| {
                    let item = $term::set_matches_states(_state, _set_contains_id);
                    _state.add($term::SIZE);
                    item
                })*
            }
        }
    };
}

all_tuples!(or_term_tuple, 0, 15, T);
