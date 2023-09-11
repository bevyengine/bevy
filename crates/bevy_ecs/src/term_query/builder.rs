use std::marker::PhantomData;

use crate::{
    component::ComponentId,
    prelude::{Component, World},
};

use super::{QueryTermGroup, Term, TermQueryState};

pub struct QueryBuilder<'w, Q: QueryTermGroup = ()> {
    terms: Vec<Term>,
    current_term: usize,
    world: &'w mut World,
    _marker: PhantomData<Q>,
}

impl<'w, Q: QueryTermGroup> QueryBuilder<'w, Q> {
    pub fn new(world: &'w mut World) -> Self {
        let mut terms = Vec::new();
        Q::init_terms(world, &mut terms);
        Self {
            current_term: terms.len(),
            terms,
            world,
            _marker: PhantomData::default(),
        }
    }

    pub fn term<T: QueryTermGroup>(&mut self) -> &mut Self {
        self.current_term = self.terms.len();
        T::init_terms(self.world, &mut self.terms);
        self
    }

    pub fn term_at(&mut self, index: usize) -> &mut Self {
        self.current_term = index;
        self
    }

    pub fn set<T: Component>(&mut self) -> &mut Self {
        let id = self.world.init_component::<T>();
        self.set_id(id);
        self
    }

    pub fn set_id(&mut self, id: ComponentId) -> &mut Self {
        if let Term::Component(term) = &mut self.terms[self.current_term] {
            term.set_id(id)
        }
        self
    }

    pub fn build(&mut self) -> TermQueryState<Q> {
        TermQueryState::<Q>::from_terms(self.world, self.terms.clone())
    }
}
