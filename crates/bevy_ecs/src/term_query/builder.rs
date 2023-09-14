use std::marker::PhantomData;

use crate::{
    component::ComponentId,
    prelude::{Component, With, Without, World},
};

use super::{QueryTermGroup, Term, TermQueryState, TermVec};

pub struct QueryBuilder<'w, Q: QueryTermGroup = ()> {
    terms: TermVec<Term>,
    current_term: usize,
    world: &'w mut World,
    _marker: PhantomData<Q>,
}

impl<'w, Q: QueryTermGroup> QueryBuilder<'w, Q> {
    pub fn new(world: &'w mut World) -> Self {
        let mut terms = TermVec::new();
        Q::init_terms(world, &mut terms);
        Self {
            current_term: terms.len(),
            terms,
            world,
            _marker: PhantomData::default(),
        }
    }

    pub fn term<T: QueryTermGroup>(&mut self) -> &mut Self {
        T::init_terms(self.world, &mut self.terms);
        self
    }

    pub unsafe fn term_at(&mut self, index: usize) -> &mut Self {
        self.current_term = index;
        self
    }

    pub fn with<T: Component>(&mut self) -> &mut Self {
        self.term::<With<T>>()
    }

    pub fn with_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::with_id(id));
        self
    }

    pub fn without<T: Component>(&mut self) -> &mut Self {
        self.term::<Without<T>>()
    }

    pub fn without_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::without_id(id));
        self
    }

    pub fn ref_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::read_id(id));
        self
    }

    pub fn mut_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::write_id(id));
        self
    }

    pub fn added_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::added_id(id));
        self
    }

    pub fn changed_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::changed_id(id));
        self
    }

    pub fn optional(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        let terms = builder.terms.into_iter().map(|term| term.set_optional());
        self.terms.extend(terms);
        self
    }

    pub fn or(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        let term = Term::or_terms(builder.terms);
        self.terms.push(term);
        self
    }

    pub fn any_of(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        let term = Term::any_of_terms(builder.terms);
        self.terms.push(term);
        self
    }

    pub fn push(&mut self, term: Term) -> &mut Self {
        self.terms.push(term);
        self
    }

    pub fn set_dynamic<T: Component>(&mut self) -> &mut Self {
        let id = self.world.init_component::<T>();
        self.set_dynamic_by_id(id);
        self
    }

    pub fn set_dynamic_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms[self.current_term].component = Some(id);
        self
    }

    pub fn terms(&self) -> &TermVec<Term> {
        &self.terms
    }

    pub fn build(&mut self) -> TermQueryState<Q> {
        TermQueryState::<Q>::from_terms(self.world, self.terms.clone())
    }
}
