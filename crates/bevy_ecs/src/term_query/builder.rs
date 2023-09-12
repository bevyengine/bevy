use std::marker::PhantomData;

use crate::{
    component::ComponentId,
    prelude::{Component, With, Without, World},
};

use super::{ComponentTerm, QueryTermGroup, Term, TermQueryState, TermVec};

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
        self.terms.push(Term::Component(ComponentTerm::with(id)));
        self
    }

    pub fn without<T: Component>(&mut self) -> &mut Self {
        self.term::<Without<T>>()
    }

    pub fn without_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::Component(ComponentTerm::without(id)));
        self
    }

    pub fn ref_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::Component(ComponentTerm::read_id(id)));
        self
    }

    pub fn mut_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms
            .push(Term::Component(ComponentTerm::write_id(id)));
        self
    }

    pub fn set_dynamic<T: Component>(&mut self) -> &mut Self {
        let id = self.world.init_component::<T>();
        self.set_dynamic_by_id(id);
        self
    }

    pub fn set_dynamic_by_id(&mut self, id: ComponentId) -> &mut Self {
        if let Term::Component(term) = &mut self.terms[self.current_term] {
            term.set_id(id)
        }
        self
    }

    pub fn build(&mut self) -> TermQueryState<Q> {
        TermQueryState::<Q>::from_terms(self.world, self.terms.clone())
    }
}
