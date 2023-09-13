use crate::{
    entity::Entity,
    prelude::{EntityMut, EntityRef, World},
    query::DebugCheckedUnwrap,
};

use super::{FetchedTerm, QueryTerm, Term, TermAccess};

impl QueryTerm for Entity {
    type Item<'w> = Self;
    type ReadOnly = Self;

    fn init_term(_world: &mut World) -> Term {
        Term::entity()
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &'w FetchedTerm<'w>) -> Self::Item<'w> {
        term.entity
    }
}

impl QueryTerm for EntityRef<'_> {
    type Item<'w> = EntityRef<'w>;
    type ReadOnly = Self;

    fn init_term(_world: &mut World) -> Term {
        Term::entity().set_access(TermAccess::Read)
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        EntityRef::new(term.entity_cell().debug_checked_unwrap())
    }
}

impl<'r> QueryTerm for EntityMut<'r> {
    type Item<'w> = EntityMut<'w>;
    type ReadOnly = EntityRef<'r>;

    fn init_term(_world: &mut World) -> Term {
        Term::entity().set_access(TermAccess::Write)
    }

    #[inline]
    unsafe fn from_fetch<'w>(term: &FetchedTerm<'w>) -> Self::Item<'w> {
        EntityMut::new(term.entity_cell().debug_checked_unwrap())
    }
}
