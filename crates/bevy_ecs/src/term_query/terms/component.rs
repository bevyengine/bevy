use std::cell::UnsafeCell;

use bevy_ptr::{Ptr, PtrMut, UnsafeCellDeref};

use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    change_detection::{Mut, TicksMut},
    component::{ComponentId, Tick, TickCells},
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
}

pub enum StoragePtr<'w> {
    SpareSet(&'w ComponentSparseSet),
    Table(Ptr<'w>),
}

pub struct ComponentTermState<'w> {
    pointer: Option<StoragePtr<'w>>,
    change_detection: Option<TickCells<'w>>,

    last_run: Tick,
    this_run: Tick,

    size: usize,
}

pub struct FetchedChangeDetection<'w> {
    added: &'w UnsafeCell<Tick>,
    changed: &'w UnsafeCell<Tick>,

    last_run: Tick,
    this_run: Tick,
}

pub struct FetchedComponent<'w> {
    component: Option<Ptr<'w>>,
    change_detection: Option<FetchedChangeDetection<'w>>,
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
            pointer: world
                .storages()
                .sparse_sets
                .get(self.id())
                .map(|set| StoragePtr::SpareSet(set)),
            change_detection: None,

            size: info.layout().size(),

            last_run,
            this_run,
        }
    }

    unsafe fn set_table<'w>(&self, state: &mut Self::State<'w>, table: &'w Table) {
        if let Some(column) = table.get_column(self.id()) {
            state.pointer = Some(StoragePtr::Table(column.get_data_ptr()));
        }
    }

    unsafe fn fetch<'w>(
        &self,
        state: &mut Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        match state.pointer.as_ref() {
            Some(StoragePtr::SpareSet(sparse_set)) => {
                let (component, ticks) = sparse_set.get_with_ticks(entity).debug_checked_unwrap();
                FetchedComponent {
                    component: Some(component),
                    change_detection: self.change_detection.then(|| FetchedChangeDetection {
                        added: ticks.added,
                        changed: ticks.changed,

                        last_run: state.last_run,
                        this_run: state.this_run,
                    }),
                }
            }
            Some(StoragePtr::Table(table)) => FetchedComponent {
                component: Some(table.byte_add(table_row.index() * state.size)),
                change_detection: self.change_detection.then(|| {
                    let change_detection = state.change_detection.unwrap();
                    FetchedChangeDetection {
                        added: change_detection.added,
                        changed: change_detection.changed,
                        last_run: state.last_run,
                        this_run: state.this_run,
                    }
                }),
            },
            None => FetchedComponent {
                component: None,
                change_detection: None,
            },
        }
    }

    unsafe fn filter_fetch<'w>(
        &self,
        state: &mut Self::State<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> bool {
        state.pointer.is_some()
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
            TermOperator::With => set_contains_id(self.id()),
            TermOperator::Without => !set_contains_id(self.id()),
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
        let change_detection = term.change_detection.debug_checked_unwrap();
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
