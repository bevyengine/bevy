use std::cell::UnsafeCell;

use bevy_ptr::{Ptr, PtrMut, ThinSlicePtr, UnsafeCellDeref};

use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    change_detection::{Mut, TicksMut},
    component::{ComponentId, Tick},
    entity::Entity,
    prelude::{Component, Has, With, Without, World},
    query::{Access, DebugCheckedUnwrap, FilteredAccess},
    storage::{ComponentSparseSet, Table, TableRow},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{Fetchable, FetchedTerm, QueryTerm, Term, TermAccess};

#[derive(Clone)]
pub enum TermOperator {
    With,
    Without,
    Changed,
    Added,
}

#[derive(Clone)]
pub struct ComponentTerm {
    component: Option<ComponentId>,
    access: Option<TermAccess>,
    operator: TermOperator,
    change_detection: bool,
}

impl ComponentTerm {
    pub fn with(id: ComponentId) -> Self {
        Self {
            component: Some(id),
            access: None,
            operator: TermOperator::With,
            change_detection: false,
        }
    }

    pub fn without(id: ComponentId) -> Self {
        Self {
            component: Some(id),
            access: None,
            operator: TermOperator::Without,
            change_detection: false,
        }
    }

    pub fn read() -> Self {
        Self {
            component: None,
            access: Some(TermAccess::Read),
            operator: TermOperator::With,
            change_detection: false,
        }
    }

    pub fn read_id(id: ComponentId) -> Self {
        Self {
            component: Some(id),
            access: Some(TermAccess::Read),
            operator: TermOperator::With,
            change_detection: false,
        }
    }

    pub fn write() -> Self {
        Self {
            component: None,
            access: Some(TermAccess::Write),
            operator: TermOperator::With,
            change_detection: true,
        }
    }

    pub fn write_id(id: ComponentId) -> Self {
        Self {
            component: Some(id),
            access: Some(TermAccess::Write),
            operator: TermOperator::With,
            change_detection: true,
        }
    }

    pub fn id(&self) -> ComponentId {
        self.component.unwrap()
    }

    pub fn set_id(&mut self, id: ComponentId) {
        self.component = Some(id);
    }

    unsafe fn get_component<'w>(
        &self,
        state: &mut ComponentTermState<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Ptr<'w> {
        match state.pointer.as_ref().unwrap() {
            StoragePtr::SparseSet(sparse_set) => sparse_set.get(entity).debug_checked_unwrap(),
            StoragePtr::Table(table) => table.byte_add(table_row.index() * state.size),
        }
    }

    unsafe fn get_change_ticks<'w>(
        &self,
        state: &mut ComponentTermState<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> FetchedChangeTicks<'w> {
        match state.pointer.as_ref().unwrap() {
            StoragePtr::SparseSet(sparse_set) => {
                let ticks = sparse_set.get_tick_cells(entity).debug_checked_unwrap();
                FetchedChangeTicks {
                    added: ticks.added,
                    changed: ticks.changed,

                    last_run: state.last_run,
                    this_run: state.this_run,
                }
            }
            StoragePtr::Table(_) => {
                let (added, changed) = state.ticks.debug_checked_unwrap();
                FetchedChangeTicks {
                    added: added.get(table_row.index()),
                    changed: changed.get(table_row.index()),

                    last_run: state.last_run,
                    this_run: state.this_run,
                }
            }
        }
    }
}

pub enum StoragePtr<'w> {
    SparseSet(&'w ComponentSparseSet),
    Table(Ptr<'w>),
}

pub struct ComponentTermState<'w> {
    pointer: Option<StoragePtr<'w>>,
    ticks: Option<(
        ThinSlicePtr<'w, UnsafeCell<Tick>>,
        ThinSlicePtr<'w, UnsafeCell<Tick>>,
    )>,

    last_run: Tick,
    this_run: Tick,

    size: usize,
}

pub struct FetchedChangeTicks<'w> {
    added: &'w UnsafeCell<Tick>,
    changed: &'w UnsafeCell<Tick>,

    last_run: Tick,
    this_run: Tick,
}

pub struct FetchedComponent<'w> {
    component: Option<Ptr<'w>>,
    change_ticks: Option<FetchedChangeTicks<'w>>,
}

impl Fetchable for ComponentTerm {
    type State<'w> = ComponentTermState<'w>;
    type Item<'w> = FetchedComponent<'w>;

    unsafe fn init_state<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> ComponentTermState<'w> {
        let id = self.id();
        let info = world.components().get_info_unchecked(id);
        ComponentTermState {
            pointer: self
                .access
                .is_some()
                .then(|| {
                    world
                        .storages()
                        .sparse_sets
                        .get(self.id())
                        .map(|set| StoragePtr::SparseSet(set))
                })
                .flatten(),
            ticks: None,

            size: info.layout().size(),

            last_run,
            this_run,
        }
    }

    unsafe fn set_table<'w>(&self, state: &mut Self::State<'w>, table: &'w Table) {
        if let Some(column) = table.get_column(self.id()) {
            state.pointer = self
                .access
                .is_some()
                .then(|| StoragePtr::Table(column.get_data_ptr()));
            state.ticks = self.change_detection.then(|| {
                (
                    column.get_added_ticks_slice().into(),
                    column.get_changed_ticks_slice().into(),
                )
            })
        }
    }

    unsafe fn fetch<'w>(
        &self,
        state: &mut Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        FetchedComponent {
            component: self
                .access
                .is_some()
                .then(|| self.get_component(state, entity, table_row)),
            change_ticks: self
                .change_detection
                .then(|| self.get_change_ticks(state, entity, table_row)),
        }
    }

    unsafe fn filter_fetch<'w>(
        &self,
        state: &mut Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool {
        match self.operator {
            // These are checked matches_component_set
            TermOperator::With => true,
            TermOperator::Without => true,
            TermOperator::Changed => {
                let ticks = self.get_change_ticks(state, entity, table_row);
                ticks
                    .changed
                    .read()
                    .is_newer_than(ticks.last_run, ticks.this_run)
            }
            TermOperator::Added => {
                let ticks = self.get_change_ticks(state, entity, table_row);
                ticks
                    .added
                    .read()
                    .is_newer_than(ticks.last_run, ticks.this_run)
            }
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        let id = self.id();
        debug_assert!(
            self.access.is_none() || !access.access().has_write(id),
            "{:?} conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
            id,
        );
        match self.access {
            Some(TermAccess::Read) => access.add_read(id),
            Some(TermAccess::Write) => access.add_write(id),
            None => {}
        }
    }

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        let component_id = self.id();
        if let Some(archetype_component_id) = archetype.get_archetype_component_id(component_id) {
            match self.access {
                Some(TermAccess::Read) => access.add_read(archetype_component_id),
                Some(TermAccess::Write) => access.add_write(archetype_component_id),
                None => {}
            }
        }
    }

    fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        match self.operator {
            TermOperator::Without => !set_contains_id(self.id()),
            _ => set_contains_id(self.id()),
        }
    }
}

impl<T: Component> QueryTerm for With<T> {
    type Item<'w> = ();
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::Component(ComponentTerm::with(component))
    }

    unsafe fn from_fetch<'w>(_term: FetchedTerm<'w>) -> Self::Item<'w> {}
}

impl<T: Component> QueryTerm for Without<T> {
    type Item<'w> = ();
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::Component(ComponentTerm::without(component))
    }

    unsafe fn from_fetch<'w>(_term: FetchedTerm<'w>) -> Self::Item<'w> {}
}

impl<T: Component> QueryTerm for Has<T> {
    type Item<'w> = bool;
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::Component(ComponentTerm::without(component))
    }

    unsafe fn from_fetch<'w>(term: FetchedTerm<'w>) -> Self::Item<'w> {
        let FetchedTerm::Component(term) = term else {
            unreachable!();
        };
        term.component.is_some()
    }
}

impl<T: Component> QueryTerm for &T {
    type Item<'w> = &'w T;
    type ReadOnly = Self;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::Component(ComponentTerm::read_id(component))
    }

    unsafe fn from_fetch<'w>(term: FetchedTerm<'w>) -> Self::Item<'w> {
        let FetchedTerm::Component(term) = term else {
            unreachable!();
        };
        term.component.unwrap().deref()
    }
}

impl QueryTerm for Ptr<'_> {
    type Item<'w> = Ptr<'w>;
    type ReadOnly = Self;

    fn init_term(_world: &mut World) -> Term {
        Term::Component(ComponentTerm::read())
    }

    unsafe fn from_fetch<'w>(term: FetchedTerm<'w>) -> Self::Item<'w> {
        let FetchedTerm::Component(term) = term else {
            unreachable!();
        };
        term.component.unwrap()
    }
}

impl<'r, T: Component> QueryTerm for &'r mut T {
    type Item<'w> = Mut<'w, T>;
    type ReadOnly = &'r T;

    fn init_term(world: &mut World) -> Term {
        let component = world.init_component::<T>();
        Term::Component(ComponentTerm::write_id(component))
    }

    unsafe fn from_fetch<'w>(term: FetchedTerm<'w>) -> Self::Item<'w> {
        let FetchedTerm::Component(term) = term else {
            unreachable!();
        };
        let change_detection = term.change_ticks.debug_checked_unwrap();
        Mut {
            value: term
                .component
                .debug_checked_unwrap()
                .assert_unique()
                .deref_mut(),
            ticks: TicksMut {
                added: change_detection.added.deref_mut(),
                changed: change_detection.changed.deref_mut(),

                last_run: change_detection.last_run,
                this_run: change_detection.this_run,
            },
        }
    }
}

impl<'r> QueryTerm for PtrMut<'r> {
    type Item<'w> = PtrMut<'w>;
    type ReadOnly = Ptr<'r>;

    fn init_term(_world: &mut World) -> Term {
        Term::Component(ComponentTerm::read())
    }

    unsafe fn from_fetch<'w>(term: FetchedTerm<'w>) -> Self::Item<'w> {
        let FetchedTerm::Component(term) = term else {
            unreachable!();
        };
        term.component.unwrap().assert_unique()
    }
}
