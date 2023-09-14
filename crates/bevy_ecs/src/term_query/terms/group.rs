use crate::{
    prelude::{AnyOf, Or, World},
    query::DebugCheckedUnwrap,
    term_query::TermVec,
};

use super::{FetchedTerm, QueryTermGroup, Term};

impl<Q: QueryTermGroup> QueryTermGroup for Or<Q> {
    type Item<'w> = ();
    type ReadOnly = Self;
    type Optional = ();

    fn init_terms(world: &mut World, terms: &mut TermVec<Term>) {
        let mut sub_terms = Vec::new();
        Q::init_terms(world, &mut sub_terms);
        terms.push(Term::or_terms(sub_terms));
    }

    #[inline]
    unsafe fn from_fetches<'w: 'f, 'f>(
        terms: &mut impl Iterator<Item = &'f FetchedTerm<'w>>,
    ) -> Self::Item<'w> {
        terms.next();
    }
}

impl<Q: QueryTermGroup> QueryTermGroup for AnyOf<Q> {
    type Item<'w> = <Q::Optional as QueryTermGroup>::Item<'w>;
    type ReadOnly = Self;
    type Optional = ();

    fn init_terms(world: &mut World, terms: &mut TermVec<Term>) {
        let mut sub_terms = Vec::new();
        Q::Optional::init_terms(world, &mut sub_terms);
        terms.push(Term::any_of_terms(sub_terms));
    }

    #[inline]
    unsafe fn from_fetches<'w: 'f, 'f>(
        terms: &mut impl Iterator<Item = &'f FetchedTerm<'w>>,
    ) -> Self::Item<'w> {
        let term = terms.next().debug_checked_unwrap();
        Q::Optional::from_fetches(&mut term.sub_terms().debug_checked_unwrap().iter())
    }
}
