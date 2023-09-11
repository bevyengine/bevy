use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{ComponentId, Tick},
    entity::Entity,
    prelude::{Or, World},
    query::{Access, FilteredAccess},
    storage::{Table, TableRow},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{Fetchable, FetchedTerm, QueryTermGroup, Term, TermState};

#[derive(Clone)]
pub struct OrTerm {
    terms: Vec<Term>,
}

pub struct OrTermState<'w> {
    state: TermState<'w>,
    matches: bool,
}

impl Fetchable for OrTerm {
    type State<'w> = Vec<OrTermState<'w>>;
    type Item<'w> = ();

    unsafe fn init_state<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Vec<OrTermState<'w>> {
        self.terms
            .iter()
            .map(|term| OrTermState {
                state: term.init_state(world, last_run, this_run),
                matches: false,
            })
            .collect()
    }

    unsafe fn set_table<'w>(&self, state: &mut Self::State<'w>, table: &'w Table) {
        self.terms
            .iter()
            .zip(state.iter_mut())
            .for_each(|(term, state)| {
                state.matches = term.matches_component_set(&|id| table.has_column(id));
                if state.matches {
                    term.set_table(&mut state.state, table)
                }
            })
    }

    unsafe fn fetch<'w>(
        &self,
        _state: &mut Self::State<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
    }

    unsafe fn filter_fetch<'w>(
        &self,
        state: &mut Self::State<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool {
        self.terms
            .iter()
            .zip(state.iter_mut())
            .any(|(term, state)| {
                state.matches && term.filter_fetch(&mut state.state, entity, table_row)
            })
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        let mut iter = self.terms.iter();
        let Some(term) = iter.next() else {
            return
        };
        let mut new_access = access.clone();
        term.update_component_access(&mut new_access);
        self.terms.iter().for_each(|term| {
            let mut intermediate = access.clone();
            term.update_component_access(&mut intermediate);
            new_access.append_or(&intermediate);
            new_access.extend_access(&intermediate);
        });
        *access = new_access;
    }

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        self.terms
            .iter()
            .for_each(|term| term.update_archetype_component_access(archetype, access))
    }

    fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        self.terms
            .iter()
            .any(|term| term.matches_component_set(set_contains_id))
    }
}

impl<Q: QueryTermGroup> QueryTermGroup for Or<Q> {
    type Item<'w> = ();
    type ReadOnly = Self;

    fn init_terms(world: &mut World, terms: &mut Vec<Term>) {
        let mut sub_terms = Vec::new();
        Q::init_terms(world, &mut sub_terms);
        terms.push(Term::Or(OrTerm { terms: sub_terms }));
    }

    unsafe fn from_fetches<'w>(
        terms: &mut impl Iterator<Item = FetchedTerm<'w>>,
    ) -> Self::Item<'w> {
        terms.next();
    }
}
