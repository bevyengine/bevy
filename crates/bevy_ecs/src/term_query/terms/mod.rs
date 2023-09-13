use std::cell::UnsafeCell;

use bevy_ptr::{Ptr, ThinSlicePtr, UnsafeCellDeref};
use bevy_utils::all_tuples;

use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{ComponentId, StorageType, Tick},
    entity::{Entity, EntityLocation},
    prelude::World,
    query::{Access, DebugCheckedUnwrap, FilteredAccess},
    storage::{ComponentSparseSet, Table, TableRow},
    world::unsafe_world_cell::{UnsafeEntityCell, UnsafeWorldCell},
};

mod component;
mod entity;
mod group;

pub use component::*;
pub use entity::*;
pub use group::*;

use super::TermVec;

#[derive(Eq, PartialEq, Clone)]
pub enum TermAccess {
    Read,
    Write,
}

#[derive(Clone, Default)]
pub enum TermOperator {
    #[default]
    With,
    Without,
    Changed,
    Added,
    Optional,
}

#[derive(Clone, Default)]
pub struct Term {
    pub entity: bool,
    pub component: Option<ComponentId>,
    pub access: Option<TermAccess>,
    pub operator: TermOperator,
    pub change_detection: bool,
    pub sub_terms: Vec<Term>,
}

impl Term {
    pub fn sub_terms(sub_terms: Vec<Term>) -> Self {
        let mut term = Self::default();
        term.sub_terms = sub_terms;
        term
    }

    pub fn set_id(mut self, id: ComponentId) -> Self {
        self.component = Some(id);
        self
    }

    pub fn set_operator(mut self, op: TermOperator) -> Self {
        self.operator = op;
        self
    }

    pub fn set_access(mut self, access: TermAccess) -> Self {
        self.access = Some(access);
        self
    }

    pub fn set_optional(mut self) -> Self {
        self.operator = TermOperator::Optional;
        self
    }

    pub fn entity() -> Self {
        let mut term = Self::default();
        term.entity = true;
        term
    }

    pub fn with_id(id: ComponentId) -> Self {
        Self::default().set_id(id)
    }

    pub fn without_id(id: ComponentId) -> Self {
        Self::default()
            .set_operator(TermOperator::Without)
            .set_id(id)
    }

    pub fn read() -> Self {
        Self::default().set_access(TermAccess::Read)
    }

    pub fn read_id(id: ComponentId) -> Self {
        Self::read().set_id(id)
    }

    pub fn write() -> Self {
        Self::default().set_access(TermAccess::Write)
    }

    pub fn write_id(id: ComponentId) -> Self {
        Self::write().set_id(id)
    }

    pub fn added_id(id: ComponentId) -> Self {
        Self::with_id(id).set_operator(TermOperator::Added)
    }

    pub fn changed_id(id: ComponentId) -> Self {
        Self::with_id(id).set_operator(TermOperator::Changed)
    }

    pub fn with_change_detection(mut self) -> Self {
        self.change_detection = true;
        self
    }
}

pub enum TermStatePtr<'w> {
    SparseSet(&'w ComponentSparseSet),
    Table(Option<Ptr<'w>>),
    World(UnsafeWorldCell<'w>),
    Group(Vec<TermState<'w>>),
    None,
}

pub struct TermStateTicks<'w> {
    ptrs: Option<(
        ThinSlicePtr<'w, UnsafeCell<Tick>>,
        ThinSlicePtr<'w, UnsafeCell<Tick>>,
    )>,

    last_run: Tick,
    this_run: Tick,
}

pub struct TermState<'w> {
    ptr: TermStatePtr<'w>,
    ticks: TermStateTicks<'w>,

    size: usize,
    matches: bool,
}

impl TermState<'_> {
    #[inline]
    pub fn dense(&self) -> bool {
        if let TermStatePtr::SparseSet(_) = self.ptr {
            false
        } else {
            true
        }
    }
}

#[derive(Clone)]
pub struct FetchedTicks<'w> {
    added: &'w UnsafeCell<Tick>,
    changed: &'w UnsafeCell<Tick>,

    last_run: Tick,
    this_run: Tick,
}

#[derive(Clone)]
pub enum FetchPtr<'w> {
    Component {
        component: Ptr<'w>,
        change_ticks: Option<FetchedTicks<'w>>,
    },
    Entity {
        location: EntityLocation,
        world: UnsafeWorldCell<'w>,
    },
    Group {
        sub_terms: Vec<FetchedTerm<'w>>,
    },
    None,
}

#[derive(Clone)]
pub struct FetchedTerm<'w> {
    entity: Entity,
    ptr: FetchPtr<'w>,
    matched: bool,
}

impl<'w> FetchedTerm<'w> {
    pub fn component_ptr(&self) -> Option<Ptr<'w>> {
        if let FetchPtr::Component { component, .. } = self.ptr {
            Some(component)
        } else {
            None
        }
    }

    pub fn change_ticks(&self) -> Option<&FetchedTicks<'w>> {
        if let FetchPtr::Component {
            change_ticks: Some(change_ticks),
            ..
        } = &self.ptr
        {
            Some(change_ticks)
        } else {
            None
        }
    }

    pub fn entity_cell(&self) -> Option<UnsafeEntityCell<'w>> {
        if let FetchPtr::Entity { location, world } = self.ptr {
            Some(UnsafeEntityCell::new(world, self.entity, location))
        } else {
            None
        }
    }

    pub fn sub_terms(&self) -> Option<&Vec<FetchedTerm<'w>>> {
        if let FetchPtr::Group { sub_terms } = &self.ptr {
            Some(sub_terms)
        } else {
            None
        }
    }
}

pub trait Fetchable {
    type State<'w>;
    type Item<'w>;

    unsafe fn init_state<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self::State<'w>;

    unsafe fn set_table<'w>(&self, state: &mut Self::State<'w>, table: &'w Table);

    unsafe fn fetch<'w>(
        &self,
        state: &Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w>;

    unsafe fn filter_fetch<'w>(
        &self,
        state: &Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool;

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>);

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    );

    fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool;
}

impl Fetchable for Term {
    type State<'w> = TermState<'w>;
    type Item<'w> = FetchedTerm<'w>;

    #[inline]
    unsafe fn init_state<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> TermState<'w> {
        let change_ticks = TermStateTicks {
            ptrs: None,
            last_run,
            this_run,
        };
        if self.entity {
            TermState {
                ptr: TermStatePtr::World(world),
                ticks: change_ticks,
                size: 0,
                matches: true,
            }
        } else if let Some(component_id) = self.component {
            let info = world.components().get_info_unchecked(component_id);
            let storage = info.storage_type();
            let mut matches = false;
            let mut pointer = TermStatePtr::Table(None);
            if let StorageType::SparseSet = storage {
                let set = world.storages().sparse_sets.get(component_id);
                if let Some(set) = set {
                    pointer = TermStatePtr::SparseSet(set);
                    matches = true;
                }
            }
            TermState {
                ptr: pointer,
                size: info.layout().size(),
                ticks: change_ticks,
                matches,
            }
        } else {
            let state = self
                .sub_terms
                .iter()
                .map(|term| term.init_state(world, last_run, this_run))
                .collect();
            TermState {
                ptr: TermStatePtr::Group(state),
                ticks: change_ticks,
                size: 0,
                matches: false,
            }
        }
    }

    #[inline]
    unsafe fn set_table<'w>(&self, state: &mut Self::State<'w>, table: &'w Table) {
        state.matches = match &mut state.ptr {
            TermStatePtr::Table(_) => {
                if let Some(column) = table.get_column(self.component.debug_checked_unwrap()) {
                    state.ptr = TermStatePtr::Table(Some(column.get_data_ptr()));
                    state.ticks.ptrs = Some((
                        column.get_added_ticks_slice().into(),
                        column.get_changed_ticks_slice().into(),
                    ));

                    true
                } else {
                    false
                }
            }
            TermStatePtr::Group(state) => {
                let mut group_matches = false;
                self.sub_terms
                    .iter()
                    .zip(state.iter_mut())
                    .for_each(|(term, mut state)| {
                        term.set_table(&mut state, table);
                        group_matches |= state.matches
                    });
                group_matches
            }
            _ => true,
        }
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        &self,
        state: &Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        if !state.matches {
            return FetchedTerm {
                entity,
                ptr: FetchPtr::None,
                matched: false,
            };
        }

        if self.access.is_none() {
            return FetchedTerm {
                entity,
                ptr: FetchPtr::None,
                matched: true,
            };
        }

        match &state.ptr {
            TermStatePtr::World(world) => FetchedTerm {
                entity,
                ptr: if self.access.is_some() {
                    FetchPtr::Entity {
                        world: *world,
                        location: world.entities().get(entity).debug_checked_unwrap(),
                    }
                } else {
                    FetchPtr::None
                },
                matched: true,
            },
            TermStatePtr::Table(table) => FetchedTerm {
                entity,
                ptr: FetchPtr::Component {
                    component: table
                        .debug_checked_unwrap()
                        .byte_add(table_row.index() * state.size),
                    change_ticks: if self.change_detection {
                        let (added, changed) = state.ticks.ptrs.debug_checked_unwrap();

                        Some(FetchedTicks {
                            added: added.get(table_row.index()),
                            changed: changed.get(table_row.index()),

                            last_run: state.ticks.last_run,
                            this_run: state.ticks.this_run,
                        })
                    } else {
                        None
                    },
                },
                matched: true,
            },

            TermStatePtr::SparseSet(sparse_set) => FetchedTerm {
                entity,
                ptr: FetchPtr::Component {
                    component: sparse_set.get(entity).debug_checked_unwrap(),
                    change_ticks: if self.change_detection {
                        let ticks = sparse_set.get_tick_cells(entity).debug_checked_unwrap();
                        Some(FetchedTicks {
                            added: ticks.added,
                            changed: ticks.changed,

                            last_run: state.ticks.last_run,
                            this_run: state.ticks.this_run,
                        })
                    } else {
                        None
                    },
                },
                matched: true,
            },
            TermStatePtr::Group(sub_state) => FetchedTerm {
                entity,
                ptr: FetchPtr::Group {
                    sub_terms: {
                        self.sub_terms
                            .iter()
                            .zip(sub_state.iter())
                            .map(|(term, state)| term.fetch(state, entity, table_row))
                            .collect()
                    },
                },
                matched: true,
            },
            TermStatePtr::None => FetchedTerm {
                entity,
                ptr: FetchPtr::None,
                matched: false,
            },
        }
    }

    #[inline(always)]
    unsafe fn filter_fetch<'w>(
        &self,
        state: &Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool {
        match &state.ptr {
            TermStatePtr::World(_) => true,
            TermStatePtr::SparseSet(set) => {
                match self.operator {
                    TermOperator::Optional => true,
                    // These are checked in matches_component_set
                    TermOperator::With => true,
                    TermOperator::Without => true,
                    TermOperator::Added => {
                        let cells = set.get_tick_cells(entity).debug_checked_unwrap();
                        cells
                            .added
                            .read()
                            .is_newer_than(state.ticks.last_run, state.ticks.this_run)
                    }
                    TermOperator::Changed => {
                        let cells = set.get_tick_cells(entity).debug_checked_unwrap();
                        cells
                            .changed
                            .read()
                            .is_newer_than(state.ticks.last_run, state.ticks.this_run)
                    }
                }
            }
            TermStatePtr::Table(_) => {
                match self.operator {
                    TermOperator::Optional => true,
                    // These are checked in matches_component_set
                    TermOperator::With => true,
                    TermOperator::Without => true,
                    TermOperator::Added => {
                        let (added, _) = state.ticks.ptrs.debug_checked_unwrap();
                        added
                            .get(table_row.index())
                            .read()
                            .is_newer_than(state.ticks.last_run, state.ticks.this_run)
                    }
                    TermOperator::Changed => {
                        let (_, changed) = state.ticks.ptrs.debug_checked_unwrap();
                        changed
                            .get(table_row.index())
                            .read()
                            .is_newer_than(state.ticks.last_run, state.ticks.this_run)
                    }
                }
            }
            TermStatePtr::Group(states) => self
                .sub_terms
                .iter()
                .zip(states.iter())
                .all(|(term, state)| term.filter_fetch(state, entity, table_row)),
            TermStatePtr::None => true,
        }
    }

    #[inline]
    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        if self.entity {
            debug_assert!(
                self.access.is_none() || !access.access().has_any_write(),
                "EntityTerm has conflicts with a previous access in this query. Exclusive access cannot coincide with any other accesses.",
            );
            match self.access {
                Some(TermAccess::Read) => access.read_all(),
                Some(TermAccess::Write) => access.write_all(),
                None => {}
            }
        } else if let Some(component_id) = self.component {
            debug_assert!(
                self.access.is_none() || !access.access().has_write(component_id),
                "{:?} conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                component_id,
            );
            match self.access {
                Some(TermAccess::Read) => access.add_read(component_id),
                Some(TermAccess::Write) => access.add_write(component_id),
                None => {}
            };
        } else {
            let mut iter = self.sub_terms.iter();
            let Some(term) = iter.next() else {
                return
            };
            let mut new_access = access.clone();
            term.update_component_access(&mut new_access);
            iter.for_each(|term| {
                let mut intermediate = access.clone();
                term.update_component_access(&mut intermediate);
                new_access.append_or(&intermediate);
                new_access.extend_access(&intermediate);
            });
            *access = new_access;
        }
    }

    #[inline]
    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if self.entity {
            match self.access {
                Some(TermAccess::Read) => {
                    for component_id in archetype.components() {
                        let archetype_id =
                            archetype.get_archetype_component_id(component_id).unwrap();
                        access.add_read(archetype_id);
                    }
                }
                Some(TermAccess::Write) => {
                    for component_id in archetype.components() {
                        let archetype_id =
                            archetype.get_archetype_component_id(component_id).unwrap();
                        access.add_write(archetype_id);
                    }
                }
                None => {}
            }
        } else if let Some(component_id) = self.component {
            if let Some(archetype_component_id) = archetype.get_archetype_component_id(component_id)
            {
                match self.access {
                    Some(TermAccess::Read) => access.add_read(archetype_component_id),
                    Some(TermAccess::Write) => access.add_write(archetype_component_id),
                    None => {}
                }
            }
        } else {
            self.sub_terms
                .iter()
                .for_each(|term| term.update_archetype_component_access(archetype, access))
        }
    }

    #[inline]
    fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        if self.entity {
            return true;
        } else if let Some(component_id) = self.component {
            match self.operator {
                TermOperator::Without => !set_contains_id(component_id),
                TermOperator::Optional => true,
                _ => set_contains_id(component_id),
            }
        } else {
            self.sub_terms
                .iter()
                .any(|term| term.matches_component_set(set_contains_id))
        }
    }
}

pub trait QueryTerm {
    type Item<'w>;
    type ReadOnly: QueryTerm;

    fn init_term(world: &mut World) -> Term;

    unsafe fn from_fetch<'w>(fetch: &FetchedTerm<'w>) -> Self::Item<'w>;
}

pub trait QueryTermGroup {
    type Item<'w>;
    type ReadOnly: QueryTermGroup;
    type Optional: QueryTermGroup;

    fn init_terms(world: &mut World, terms: &mut TermVec<Term>);

    unsafe fn from_fetches<'w: 'f, 'f>(
        terms: &mut impl Iterator<Item = &'f FetchedTerm<'w>>,
    ) -> Self::Item<'w>;
}

impl<T: QueryTerm> QueryTermGroup for T {
    type Item<'w> = T::Item<'w>;
    type ReadOnly = T::ReadOnly;
    type Optional = Option<T>;

    fn init_terms(world: &mut World, terms: &mut TermVec<Term>) {
        terms.push(T::init_term(world));
    }

    #[inline]
    unsafe fn from_fetches<'w: 'f, 'f>(
        terms: &mut impl Iterator<Item = &'f FetchedTerm<'w>>,
    ) -> Self::Item<'w> {
        T::from_fetch(terms.next().debug_checked_unwrap())
    }
}

macro_rules! impl_query_term_tuple {
    ($($term: ident),*) => {
        impl<$($term: QueryTermGroup),*> QueryTermGroup for ($($term,)*) {
            type Item<'w> = ($($term::Item<'w>,)*);
            type ReadOnly = ($($term::ReadOnly,)*);
            type Optional = ($($term::Optional,)*);

            fn init_terms(_world: &mut World, _terms: &mut TermVec<Term>) {
                $(
                    $term::init_terms(_world, _terms);
                )*
            }

            #[inline]
            unsafe fn from_fetches<'w: 'f, 'f>(_terms: &mut impl Iterator<Item = &'f FetchedTerm<'w>>) -> Self::Item<'w> {
                ($(
                    $term::from_fetches(_terms),
                )*)
            }
        }
    };
}

all_tuples!(impl_query_term_tuple, 0, 15, T);
