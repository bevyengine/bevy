use bevy_utils::all_tuples;

use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{ComponentId, Tick},
    entity::Entity,
    prelude::World,
    query::{Access, DebugCheckedUnwrap, FilteredAccess},
    storage::{Table, TableRow},
    world::unsafe_world_cell::UnsafeWorldCell,
};

mod component;
mod entity;
mod group;

pub use component::*;
pub use entity::*;
pub use group::*;

#[derive(Clone)]
pub enum Term {
    Entity(EntityTerm),
    Component(ComponentTerm),
    Or(OrTerm),
}

#[derive(Eq, PartialEq, Clone)]
pub enum TermAccess {
    Read,
    Write,
}

pub enum FetchedTerm<'w> {
    Entity(<EntityTerm as Fetchable>::Item<'w>),
    Component(<ComponentTerm as Fetchable>::Item<'w>),
    Or(Vec<<ComponentTerm as Fetchable>::Item<'w>>),
}

impl<'w> FetchedTerm<'w> {
    pub fn component(self) -> Option<<ComponentTerm as Fetchable>::Item<'w>> {
        if let FetchedTerm::Component(term) = self {
            Some(term)
        } else {
            None
        }
    }

    pub fn entity(&self) -> Option<&<EntityTerm as Fetchable>::Item<'w>> {
        if let FetchedTerm::Entity(term) = self {
            Some(term)
        } else {
            None
        }
    }

    pub fn group(self) -> Option<Vec<<ComponentTerm as Fetchable>::Item<'w>>> {
        if let FetchedTerm::Or(term) = self {
            Some(term)
        } else {
            None
        }
    }
}

pub enum TermState<'w> {
    Entity(<EntityTerm as Fetchable>::State<'w>),
    Component(<ComponentTerm as Fetchable>::State<'w>),
    Or(<OrTerm as Fetchable>::State<'w>),
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
        state: &mut Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w>;

    unsafe fn filter_fetch<'w>(
        &self,
        state: &mut Self::State<'w>,
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

    unsafe fn init_state<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> TermState<'w> {
        match self {
            Term::Component(term) => {
                TermState::Component(term.init_state(world, last_run, this_run))
            }
            Term::Entity(term) => TermState::Entity(term.init_state(world, last_run, this_run)),
            Term::Or(terms) => TermState::Or(terms.init_state(world, last_run, this_run)),
        }
    }

    unsafe fn set_table<'w>(&self, state: &mut Self::State<'w>, table: &'w Table) {
        match (self, state) {
            (Term::Entity(term), TermState::Entity(state)) => term.set_table(state, table),
            (Term::Component(term), TermState::Component(state)) => term.set_table(state, table),
            (Term::Or(term), TermState::Or(state)) => term.set_table(state, table),
            _ => unreachable!(),
        }
    }

    unsafe fn fetch<'w>(
        &self,
        state: &mut Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        match (self, state) {
            (Term::Entity(term), TermState::Entity(state)) => {
                FetchedTerm::Entity(term.fetch(state, entity, table_row))
            }
            (Term::Component(term), TermState::Component(state)) => {
                FetchedTerm::Component(term.fetch(state, entity, table_row))
            }
            (Term::Or(term), TermState::Or(state)) => {
                FetchedTerm::Or(term.fetch(state, entity, table_row))
            }
            _ => unreachable!(),
        }
    }

    unsafe fn filter_fetch<'w>(
        &self,
        state: &mut Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool {
        match (self, state) {
            (Term::Entity(term), TermState::Entity(state)) => {
                term.filter_fetch(state, entity, table_row)
            }
            (Term::Component(term), TermState::Component(state)) => {
                term.filter_fetch(state, entity, table_row)
            }
            (Term::Or(term), TermState::Or(state)) => term.filter_fetch(state, entity, table_row),
            _ => unreachable!(),
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        match self {
            Term::Entity(term) => term.update_component_access(access),
            Term::Component(term) => term.update_component_access(access),
            Term::Or(term) => term.update_component_access(access),
        }
    }

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        match self {
            Term::Entity(term) => term.update_archetype_component_access(archetype, access),
            Term::Component(term) => term.update_archetype_component_access(archetype, access),
            Term::Or(term) => term.update_archetype_component_access(archetype, access),
        }
    }

    fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        match self {
            Term::Entity(term) => term.matches_component_set(set_contains_id),
            Term::Component(term) => term.matches_component_set(set_contains_id),
            Term::Or(term) => term.matches_component_set(set_contains_id),
        }
    }
}

pub trait QueryTerm {
    type Item<'w>;
    type ReadOnly: QueryTermGroup;

    fn init_term(world: &mut World) -> Term;

    unsafe fn from_fetch(fetch: FetchedTerm<'_>) -> Self::Item<'_>;
}

pub trait ComponentQueryTerm {
    type Item<'w>;
    type ReadOnly: QueryTermGroup + ComponentQueryTerm;

    fn init_term(world: &mut World) -> ComponentTerm;

    unsafe fn from_fetch<'w>(_term: FetchedComponent<'w>) -> Self::Item<'w>;
}

impl<T: ComponentQueryTerm> QueryTerm for T {
    type Item<'w> = T::Item<'w>;
    type ReadOnly = T::ReadOnly;

    fn init_term(world: &mut World) -> Term {
        Term::Component(T::init_term(world))
    }

    unsafe fn from_fetch(term: FetchedTerm<'_>) -> Self::Item<'_> {
        let term = term.component().debug_checked_unwrap();
        T::from_fetch(term)
    }
}

pub trait QueryTermGroup {
    type Item<'w>;
    type ReadOnly: QueryTermGroup;

    fn init_terms(world: &mut World, terms: &mut Vec<Term>);

    unsafe fn from_fetches<'w>(terms: &mut impl Iterator<Item = FetchedTerm<'w>>)
        -> Self::Item<'w>;
}

impl<T: QueryTerm> QueryTermGroup for T {
    type Item<'w> = T::Item<'w>;
    type ReadOnly = T::ReadOnly;

    fn init_terms(world: &mut World, terms: &mut Vec<Term>) {
        terms.push(T::init_term(world));
    }

    unsafe fn from_fetches<'w>(
        terms: &mut impl Iterator<Item = FetchedTerm<'w>>,
    ) -> Self::Item<'w> {
        T::from_fetch(terms.next().unwrap())
    }
}

pub trait ComponentQueryTermGroup {
    type Item<'w>;
    type ReadOnly: ComponentQueryTermGroup;
    type Optional: ComponentQueryTermGroup;

    fn init_terms(world: &mut World, terms: &mut Vec<ComponentTerm>);

    unsafe fn from_fetches<'w>(
        terms: &mut impl Iterator<Item = FetchedComponent<'w>>,
    ) -> Self::Item<'w>;
}

impl<T: ComponentQueryTerm> ComponentQueryTermGroup for T {
    type Item<'w> = T::Item<'w>;
    type ReadOnly = T::ReadOnly;
    type Optional = Option<T>;

    fn init_terms(world: &mut World, terms: &mut Vec<ComponentTerm>) {
        terms.push(T::init_term(world));
    }

    unsafe fn from_fetches<'w>(
        terms: &mut impl Iterator<Item = FetchedComponent<'w>>,
    ) -> Self::Item<'w> {
        T::from_fetch(terms.next().unwrap())
    }
}

macro_rules! impl_query_term_tuple {
    ($($term: ident),*) => {
        impl<$($term: QueryTermGroup),*> QueryTermGroup for ($($term,)*) {
            type Item<'w> = ($($term::Item<'w>,)*);
            type ReadOnly = ($($term::ReadOnly,)*);

            fn init_terms(_world: &mut World, _terms: &mut Vec<Term>) {
                $(
                    $term::init_terms(_world, _terms);
                )*
            }


            unsafe fn from_fetches<'w>(_terms: &mut impl Iterator<Item = FetchedTerm<'w>>) -> Self::Item<'w> {
                ($(
                    $term::from_fetches(_terms),
                )*)
            }
        }
    };
}

macro_rules! impl_component_query_term_tuple {
    ($($term: ident),*) => {
        impl<$($term: ComponentQueryTermGroup),*> ComponentQueryTermGroup for ($($term,)*) {
            type Item<'w> = ($($term::Item<'w>,)*);
            type ReadOnly = ($($term::ReadOnly,)*);
            type Optional = ($($term::Optional,)*);

            fn init_terms(_world: &mut World, _terms: &mut Vec<ComponentTerm>) {
                $(
                    $term::init_terms(_world, _terms);
                )*
            }


            unsafe fn from_fetches<'w>(_terms: &mut impl Iterator<Item = FetchedComponent<'w>>) -> Self::Item<'w> {
                ($(
                    $term::from_fetches(_terms),
                )*)
            }
        }
    };
}

all_tuples!(impl_query_term_tuple, 0, 15, T);
all_tuples!(impl_component_query_term_tuple, 0, 15, T);
