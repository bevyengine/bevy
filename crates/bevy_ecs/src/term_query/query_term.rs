use bevy_ptr::{Ptr, PtrMut, UnsafeCellDeref};
use bevy_utils::all_tuples;

use crate::{
    archetype::Archetype,
    change_detection::{Mut, MutUntyped, Ticks, TicksMut},
    component::{ComponentStorage, StorageType, Tick},
    entity::Entity,
    prelude::{
        Added, AnyOf, Changed, Component, EntityMut, EntityRef, Has, Or, Ref, With, Without, World,
    },
    query::DebugCheckedUnwrap,
    storage::{Table, TableRow},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{ComponentPtr, TablePtr, Term, TermAccess, TermOperator, TermState};

/// Types that can be fetched from a [`World`] using a [`TermQuery`](crate::prelude::TermQuery).
///
/// This is implemented for all the same types as [`WorldQuery`](crate::query::WorldQuery) as well
/// as additional types that for dynamic queries.
///
/// Theses additional types are [`Ptr`] and [`PtrMut`] which are equivalent to
/// &T and &mut T respectively but their component id is set at runtime.
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ptr::Ptr;
///
/// #[derive(Component)]
/// struct MyComponent;
///
/// let mut world = World::new();
/// world.spawn(MyComponent);
///
/// let component_id = world.init_component::<MyComponent>();
///
/// let mut query = unsafe {
///     QueryBuilder::<(Entity, Ptr)>::new(&mut world)
///         .term_at(1)
///         .set_dynamic_by_id(component_id)
///         .build()
/// };
///
/// let (entity, component): (Entity, Ptr) = query.single(&world);
/// let component_ref: &MyComponent = unsafe { component.deref::<MyComponent>() };
/// ```
///
/// # Safety
///
/// Component access of `Self::ReadOnly` must be a subset of `Self`
/// and `Self::ReadOnly` must match exactly the same archetypes/tables as `Self`
///
/// Implementor must ensure that [`Self::from_fetch`] is safe to call on a [`FetchedTerm`]
/// resolved from the value returned by [`Self::init_term`]
pub trait QueryTerm {
    /// The item returned by this [`QueryTerm`]
    type Item<'w>;
    /// The read-only variant of this [`QueryTerm`]
    type ReadOnly: QueryTerm;

    /// True if all components accessed by this are [`StorageType::Table`]
    const DENSE: bool;

    /// Creates a new [`Term`] instance satisfying the requirements for [`Self::from_fetch`]
    fn init_term(world: &mut World) -> Term;

    /// Adjusts internal state to account for the next [`Table`].
    ///
    /// # Safety
    ///
    /// - `table` must contain the components accessed by `state`
    #[inline(always)]
    unsafe fn set_term_table<'w>(_state: &mut TermState<'w>, _table: &'w Table) {}

    /// Adjusts internal state to account for the next [`Archetype`].
    ///
    /// # Safety
    ///
    /// - `table` and `archetype` must contain the components accessed by `state`
    #[inline(always)]
    unsafe fn set_term_archetype<'w>(
        _state: &mut TermState<'w>,
        _archetype: &'w Archetype,
        _table: &'w Table,
    ) {
    }

    /// Fetch the [`Self::Item`] for the given entity at the given table row
    ///
    /// # Safety
    ///
    /// - `state` must be fetchable to `Self::Item`
    unsafe fn fetch_term<'w>(
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w>;
}

/// A trait representing a group of types implementing [`QueryTerm`].
///
/// This is most commonly tuples of terms or operators like [`Or`] and [`AnyOf`].
pub trait QueryTermGroup {
    /// The item returned by this [`QueryTermGroup`]
    type Item<'w>;
    /// The read-only variant of this [`QueryTermGroup`]
    type ReadOnly: QueryTermGroup;
    /// The optional variant of this [`QueryTermGroup`]
    type Optional: QueryTermGroup;

    /// True if all components accessed by this are [`StorageType::Table`]
    const DENSE: bool;

    /// Writes new [`Term`] instances to `terms`, satisfying the requirements for [`Self::from_fetches`]
    fn init_terms(world: &mut World, terms: &mut Vec<Term>);

    /// Adjusts internal state to account for the next [`Table`].
    ///
    /// # Safety
    ///
    /// - `table` must contain the components accessed by `state`
    unsafe fn set_tables<'w: 'f, 'f>(
        state: &mut impl Iterator<Item = &'f mut TermState<'w>>,
        table: &'w Table,
    );

    /// Adjusts internal state to account for the next [`Archetype`].
    ///
    /// # Safety
    ///
    /// - `table` and `archetype` must contain the components accessed by `state`
    unsafe fn set_archetypes<'w: 'f, 'f>(
        state: &mut impl Iterator<Item = &'f mut TermState<'w>>,
        archetype: &'w Archetype,
        table: &'w Table,
    );

    /// Fetch the [`Self::Item`] for the given entity at the given table row
    ///
    /// # Safety
    ///
    /// - `state` must be fetchable to `Self::Item`
    unsafe fn fetch_terms<'w: 'f, 'f>(
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &mut impl Iterator<Item = &'f TermState<'w>>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w>;
}

// Blanket implementation [`QueryTermGroup`] for [`QueryTerm`]
// Pushes a single term to the list of terms and resolves a single term from the iterator
impl<T: QueryTerm> QueryTermGroup for T {
    type Item<'w> = T::Item<'w>;
    type ReadOnly = T::ReadOnly;
    type Optional = Option<T>;

    const DENSE: bool = T::DENSE;

    fn init_terms(world: &mut World, terms: &mut Vec<Term>) {
        terms.push(T::init_term(world));
    }

    #[inline(always)]
    unsafe fn set_tables<'w: 'f, 'f>(
        state: &mut impl Iterator<Item = &'f mut TermState<'w>>,
        table: &'w Table,
    ) {
        T::set_term_table(state.next().debug_checked_unwrap(), table);
    }

    #[inline(always)]
    unsafe fn set_archetypes<'w: 'f, 'f>(
        state: &mut impl Iterator<Item = &'f mut TermState<'w>>,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        T::set_term_archetype(state.next().debug_checked_unwrap(), archetype, table);
    }

    #[inline(always)]
    unsafe fn fetch_terms<'w: 'f, 'f>(
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &mut impl Iterator<Item = &'f TermState<'w>>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        T::fetch_term(
            world,
            last_run,
            this_run,
            state.next().debug_checked_unwrap(),
            entity,
            table_row,
        )
    }
}

// Blanket implementatinon of [`QueryTermGroup`] for all tuples of [`QueryTermGroup`]
macro_rules! impl_query_term_tuple {
    ($($term: ident),*) => {
        impl<$($term: QueryTermGroup),*> QueryTermGroup for ($($term,)*) {
            type Item<'w> = ($($term::Item<'w>,)*);
            type ReadOnly = ($($term::ReadOnly,)*);
            type Optional = ($($term::Optional,)*);

            const DENSE: bool = true $(&& $term::DENSE)*;

            fn init_terms(_world: &mut World, _terms: &mut Vec<Term>) {
                $(
                    $term::init_terms(_world, _terms);
                )*
            }

            #[inline]
            unsafe fn set_tables<'w: 'f, 'f>(
                _state: &mut impl Iterator<Item = &'f mut TermState<'w>>,
                _table: &'w Table,
            ) {
                $(
                    $term::set_tables(_state, _table);
                )*
            }

            #[inline]
            unsafe fn set_archetypes<'w: 'f, 'f>(
                _state: &mut impl Iterator<Item = &'f mut TermState<'w>>,
                _archetype: &'w Archetype,
                _table: &'w Table,
            ) {
                $(
                    $term::set_archetypes(_state, _archetype, _table);
                )*
            }

            #[allow(clippy::unused_unit)]
            #[inline(always)]
            unsafe fn fetch_terms<'w: 'f, 'f>(
                _world: UnsafeWorldCell<'w>,
                _last_run: Tick,
                _this_run: Tick,
                _state: &mut impl Iterator<Item = &'f TermState<'w>>,
                _entity: Entity,
                _table_row: TableRow,
            ) -> Self::Item<'w> {
                ($(
                    $term::fetch_terms(_world, _last_run, _this_run, _state, _entity, _table_row),
                )*)
            }
        }
    };
}

all_tuples!(impl_query_term_tuple, 0, 15, T);

impl QueryTerm for Entity {
    type Item<'w> = Self;
    type ReadOnly = Self;

    const DENSE: bool = true;

    fn init_term(_world: &mut World) -> Term {
        Term::default()
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        entity
    }
}

impl QueryTerm for EntityRef<'_> {
    type Item<'w> = EntityRef<'w>;
    type ReadOnly = Self;

    const DENSE: bool = true;

    fn init_term(_world: &mut World) -> Term {
        Term::default().set_access(TermAccess::Read)
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        EntityRef::new(world.get_entity(entity).debug_checked_unwrap())
    }
}

impl<'r> QueryTerm for EntityMut<'r> {
    type Item<'w> = EntityMut<'w>;
    type ReadOnly = EntityRef<'r>;

    const DENSE: bool = true;

    fn init_term(_world: &mut World) -> Term {
        Term::default().set_access(TermAccess::Write)
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        EntityMut::new(world.get_entity(entity).debug_checked_unwrap())
    }
}

impl<T: Component> QueryTerm for With<T> {
    type Item<'w> = ();
    type ReadOnly = Self;

    const DENSE: bool = true;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::with_id(component)
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
    }
}

impl<T: Component> QueryTerm for Without<T> {
    type Item<'w> = ();
    type ReadOnly = Self;

    const DENSE: bool = true;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::without_id(component)
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
    }
}

impl<T: Component> QueryTerm for Has<T> {
    type Item<'w> = bool;
    type ReadOnly = Self;

    const DENSE: bool = true;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::with_id(component).set_operator(TermOperator::Optional)
    }

    #[inline(always)]
    unsafe fn set_term_table<'w>(state: &mut TermState<'w>, table: &'w Table) {
        let component = state.component.as_ref().debug_checked_unwrap();
        state.matches = table.has_column(component.id);
    }

    #[inline(always)]
    unsafe fn set_term_archetype<'w>(
        state: &mut TermState<'w>,
        archetype: &'w Archetype,
        _table: &'w Table,
    ) {
        let component = state.component.as_ref().debug_checked_unwrap();
        state.matches = archetype.contains(component.id);
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        state: &TermState<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        state.matches
    }
}

impl<T: Component> QueryTerm for Added<T> {
    type Item<'w> = ();
    type ReadOnly = Self;

    const DENSE: bool = match T::Storage::STORAGE_TYPE {
        StorageType::Table => true,
        StorageType::SparseSet => false,
    };

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::added_id(component)
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
    }
}

impl<T: Component> QueryTerm for Changed<T> {
    type Item<'w> = ();
    type ReadOnly = Self;

    const DENSE: bool = match T::Storage::STORAGE_TYPE {
        StorageType::Table => true,
        StorageType::SparseSet => false,
    };

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::changed_id(component)
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        _state: &TermState<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
    }
}

impl<T: Component> QueryTerm for &T {
    type Item<'w> = &'w T;
    type ReadOnly = Self;

    const DENSE: bool = match T::Storage::STORAGE_TYPE {
        StorageType::Table => true,
        StorageType::SparseSet => false,
    };

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::read_id(component)
    }

    #[inline(always)]
    unsafe fn set_term_table<'w>(state: &mut TermState<'w>, table: &'w Table) {
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            let component = state.component.as_mut().debug_checked_unwrap();
            let column = table.get_column(component.id).debug_checked_unwrap();
            let ptr = component.ptr.table_mut().debug_checked_unwrap();
            *ptr = Some(TablePtr {
                component: column.get_data_ptr(),
                added: None,
                changed: None,
            });
        }
    }

    #[inline(always)]
    unsafe fn set_term_archetype<'w>(
        state: &mut TermState<'w>,
        _archetype: &'w Archetype,
        table: &'w Table,
    ) {
        Self::set_term_table(state, table);
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        let component = state.component.as_ref().debug_checked_unwrap();
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let table = component.ptr.table().debug_checked_unwrap();
                table
                    .component
                    .byte_add(component.size * table_row.index())
                    .deref()
            }
            StorageType::SparseSet => {
                let set = component.ptr.sparse_set().debug_checked_unwrap();
                set.get(entity).debug_checked_unwrap().deref()
            }
        }
    }
}

impl<T: Component> QueryTerm for Ref<'_, T> {
    type Item<'w> = Ref<'w, T>;
    type ReadOnly = Self;

    const DENSE: bool = match T::Storage::STORAGE_TYPE {
        StorageType::Table => true,
        StorageType::SparseSet => false,
    };

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::read_id(component).with_change_detection()
    }

    #[inline(always)]
    unsafe fn set_term_table<'w>(state: &mut TermState<'w>, table: &'w Table) {
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            let component = state.component.as_mut().debug_checked_unwrap();
            let column = table.get_column(component.id).debug_checked_unwrap();
            let ptr = component.ptr.table_mut().debug_checked_unwrap();
            *ptr = Some(TablePtr {
                component: column.get_data_ptr(),
                added: Some(column.get_added_ticks_slice().get_unchecked(0).into()),
                changed: Some(column.get_changed_ticks_slice().get_unchecked(0).into()),
            });
        }
    }

    #[inline(always)]
    unsafe fn set_term_archetype<'w>(
        state: &mut TermState<'w>,
        _archetype: &'w Archetype,
        table: &'w Table,
    ) {
        Self::set_term_table(state, table);
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        let component = state.component.as_ref().debug_checked_unwrap();
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            let table = component.ptr.table().debug_checked_unwrap();
            let index = table_row.index();
            let tick_index = std::mem::size_of::<Tick>() * index;
            Ref {
                value: table.component.byte_add(component.size * index).deref(),
                ticks: Ticks {
                    added: table
                        .added
                        .debug_checked_unwrap()
                        .byte_add(tick_index)
                        .deref(),
                    changed: table
                        .changed
                        .debug_checked_unwrap()
                        .byte_add(tick_index)
                        .deref(),
                    last_run,
                    this_run,
                },
            }
        } else {
            let set = component.ptr.sparse_set().debug_checked_unwrap();
            let (component, ticks) = set.get_with_ticks(entity).debug_checked_unwrap();
            Ref {
                value: component.deref(),
                ticks: Ticks {
                    added: ticks.added.deref(),
                    changed: ticks.changed.deref(),
                    last_run,
                    this_run,
                },
            }
        }
    }
}

impl QueryTerm for Ptr<'_> {
    type Item<'w> = Ptr<'w>;
    type ReadOnly = Self;

    const DENSE: bool = false;

    fn init_term(_world: &mut World) -> Term {
        Term::read()
    }

    #[inline(always)]
    unsafe fn set_term_table<'w>(state: &mut TermState<'w>, table: &'w Table) {
        let component = state.component.as_mut().debug_checked_unwrap();
        if let Some(column) = table.get_column(component.id) {
            let ptr = component.ptr.table_mut().debug_checked_unwrap();
            *ptr = Some(TablePtr {
                component: column.get_data_ptr(),
                added: None,
                changed: None,
            });
        }
    }

    #[inline(always)]
    unsafe fn set_term_archetype<'w>(
        state: &mut TermState<'w>,
        _archetype: &'w Archetype,
        table: &'w Table,
    ) {
        Self::set_term_table(state, table);
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        _last_run: Tick,
        _this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        let component = state.component.as_ref().debug_checked_unwrap();
        match &component.ptr {
            ComponentPtr::Table(table) => {
                let table = table.as_ref().debug_checked_unwrap();
                table.component.byte_add(component.size * table_row.index())
            }
            ComponentPtr::SparseSet(set) => set.get(entity).debug_checked_unwrap(),
        }
    }
}

impl<'r, T: Component> QueryTerm for &'r mut T {
    type Item<'w> = Mut<'w, T>;
    type ReadOnly = &'r T;

    const DENSE: bool = match T::Storage::STORAGE_TYPE {
        StorageType::Table => true,
        StorageType::SparseSet => false,
    };

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::write_id(component).with_change_detection()
    }

    #[inline(always)]
    unsafe fn set_term_table<'w>(state: &mut TermState<'w>, table: &'w Table) {
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            let component = &mut state.component.as_mut().debug_checked_unwrap();
            let column = table.get_column(component.id).debug_checked_unwrap();
            let ptr = component.ptr.table_mut().debug_checked_unwrap();
            *ptr = Some(TablePtr {
                component: column.get_data_ptr(),
                added: Some(column.get_added_ticks_slice().get_unchecked(0).into()),
                changed: Some(column.get_changed_ticks_slice().get_unchecked(0).into()),
            });
        }
    }

    #[inline(always)]
    unsafe fn set_term_archetype<'w>(
        state: &mut TermState<'w>,
        _archetype: &'w Archetype,
        table: &'w Table,
    ) {
        Self::set_term_table(state, table);
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        let component = state.component.as_ref().debug_checked_unwrap();
        if T::Storage::STORAGE_TYPE == StorageType::Table {
            let table = component
                .ptr
                .table()
                .debug_checked_unwrap()
                .get_row(component.size, table_row.index());
            Mut {
                value: table.component.assert_unique().deref_mut(),
                ticks: TicksMut {
                    added: table
                        .added
                        .debug_checked_unwrap()
                        .assert_unique()
                        .deref_mut(),
                    changed: table
                        .changed
                        .debug_checked_unwrap()
                        .assert_unique()
                        .deref_mut(),
                    last_run,
                    this_run,
                },
            }
        } else {
            let set = component.ptr.sparse_set().debug_checked_unwrap();
            let (component, ticks) = set.get_with_ticks(entity).debug_checked_unwrap();
            Mut {
                value: component.assert_unique().deref_mut(),
                ticks: TicksMut {
                    added: ticks.added.deref_mut(),
                    changed: ticks.changed.deref_mut(),
                    last_run,
                    this_run,
                },
            }
        }
    }
}

impl<'r> QueryTerm for PtrMut<'r> {
    type Item<'w> = MutUntyped<'w>;
    type ReadOnly = Ptr<'r>;

    const DENSE: bool = false;

    fn init_term(_world: &mut World) -> Term {
        Term::write().with_change_detection()
    }

    #[inline(always)]
    unsafe fn set_term_table<'w>(state: &mut TermState<'w>, table: &'w Table) {
        let component = state.component.as_mut().debug_checked_unwrap();
        if let Some(column) = table.get_column(component.id) {
            let ptr = component.ptr.table_mut().debug_checked_unwrap();
            *ptr = Some(TablePtr {
                component: column.get_data_ptr(),
                added: Some(column.get_added_ticks_slice().get_unchecked(0).into()),
                changed: Some(column.get_changed_ticks_slice().get_unchecked(0).into()),
            });
        }
    }

    #[inline(always)]
    unsafe fn set_term_archetype<'w>(
        state: &mut TermState<'w>,
        _archetype: &'w Archetype,
        table: &'w Table,
    ) {
        Self::set_term_table(state, table);
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        _world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        let component = state.component.as_ref().debug_checked_unwrap();
        match &component.ptr {
            ComponentPtr::Table(table) => {
                let table = table
                    .as_ref()
                    .debug_checked_unwrap()
                    .get_row(component.size, table_row.index());
                MutUntyped {
                    value: table.component.assert_unique(),
                    ticks: TicksMut {
                        added: table
                            .added
                            .debug_checked_unwrap()
                            .assert_unique()
                            .deref_mut(),
                        changed: table
                            .changed
                            .debug_checked_unwrap()
                            .assert_unique()
                            .deref_mut(),
                        last_run,
                        this_run,
                    },
                }
            }
            ComponentPtr::SparseSet(set) => {
                let (component, ticks) = set.get_with_ticks(entity).debug_checked_unwrap();
                MutUntyped {
                    value: component.assert_unique(),
                    ticks: TicksMut {
                        added: ticks.added.deref_mut(),
                        changed: ticks.changed.deref_mut(),
                        last_run,
                        this_run,
                    },
                }
            }
        }
    }
}

impl<C: QueryTerm> QueryTerm for Option<C> {
    type Item<'w> = Option<C::Item<'w>>;
    type ReadOnly = Option<C::ReadOnly>;

    const DENSE: bool = C::DENSE;

    fn init_term(world: &mut World) -> Term {
        C::init_term(world).set_operator(TermOperator::Optional)
    }

    #[inline(always)]
    unsafe fn set_term_table<'w>(state: &mut TermState<'w>, table: &'w Table) {
        if let Some(component) = &state.component {
            state.matches = table.has_column(component.id);
        }
        if state.matches {
            C::set_term_table(state, table);
        }
    }

    #[inline(always)]
    unsafe fn set_term_archetype<'w>(
        state: &mut TermState<'w>,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        if let Some(component) = &state.component {
            state.matches = archetype.contains(component.id);
        }
        if state.matches {
            C::set_term_archetype(state, archetype, table);
        }
    }

    #[inline(always)]
    unsafe fn fetch_term<'w>(
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &TermState<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        if state.matches {
            Some(C::fetch_term(
                world, last_run, this_run, state, entity, table_row,
            ))
        } else {
            None
        }
    }
}

impl<Q: QueryTermGroup> QueryTermGroup for Or<Q> {
    type Item<'w> = ();
    type ReadOnly = Self;
    type Optional = ();

    const DENSE: bool = Q::DENSE;

    fn init_terms(world: &mut World, terms: &mut Vec<Term>) {
        let start = terms.len();
        Q::init_terms(world, terms);
        for i in start..terms.len() - 1 {
            terms[i].or = true;
        }
    }

    #[inline(always)]
    unsafe fn set_tables<'w: 'f, 'f>(
        state: &mut impl Iterator<Item = &'f mut TermState<'w>>,
        table: &'w Table,
    ) {
        Q::set_tables(state, table);
    }

    #[inline(always)]
    unsafe fn set_archetypes<'w: 'f, 'f>(
        state: &mut impl Iterator<Item = &'f mut TermState<'w>>,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        Q::set_archetypes(state, archetype, table);
    }

    #[inline(always)]
    unsafe fn fetch_terms<'w: 'f, 'f>(
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &mut impl Iterator<Item = &'f TermState<'w>>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        Q::Optional::fetch_terms(world, last_run, this_run, state, entity, table_row);
    }
}

impl<Q: QueryTermGroup> QueryTermGroup for AnyOf<Q> {
    type Item<'w> = <Q::Optional as QueryTermGroup>::Item<'w>;
    type ReadOnly = Self;
    type Optional = ();

    const DENSE: bool = Q::DENSE;

    fn init_terms(world: &mut World, terms: &mut Vec<Term>) {
        let start = terms.len();
        Q::init_terms(world, terms);
        for i in start..terms.len() - 1 {
            terms[i].or = true;
        }
    }

    #[inline(always)]
    unsafe fn set_tables<'w: 'f, 'f>(
        state: &mut impl Iterator<Item = &'f mut TermState<'w>>,
        table: &'w Table,
    ) {
        Q::Optional::set_tables(state, table);
    }

    #[inline(always)]
    unsafe fn set_archetypes<'w: 'f, 'f>(
        state: &mut impl Iterator<Item = &'f mut TermState<'w>>,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        Q::Optional::set_archetypes(state, archetype, table);
    }

    #[inline(always)]
    unsafe fn fetch_terms<'w: 'f, 'f>(
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
        state: &mut impl Iterator<Item = &'f TermState<'w>>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        Q::Optional::fetch_terms(world, last_run, this_run, state, entity, table_row)
    }
}
