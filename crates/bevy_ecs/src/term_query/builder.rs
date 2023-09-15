use std::marker::PhantomData;

use crate::{component::ComponentId, prelude::*};

use super::{QueryTermGroup, Term, TermOperator, TermQueryState};

/// Builder for [`TermQuery`]
///
/// TODO: Doc examples
///
/// See tests in `mod.rs` for more details.
pub struct QueryBuilder<'w, Q: QueryTermGroup = ()> {
    terms: Vec<Term>,
    current_term: usize,
    world: &'w mut World,
    _marker: PhantomData<Q>,
}

impl<'w, Q: QueryTermGroup> QueryBuilder<'w, Q> {
    /// Creates a new builder with the [`Term`]s represented by `Q`
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

    /// Adds a term representing `T` to self
    pub fn term<T: QueryTermGroup>(&mut self) -> &mut Self {
        T::init_terms(self.world, &mut self.terms);
        self
    }

    /// Sets `current_term` to `index`
    ///
    /// This is primarily intended for use with [`Self::set_dynamic`] and [`Self::set_dynamic_by_id].
    ///
    /// SAFETY: terms must not be modified such that they become incompatible with Q
    pub unsafe fn term_at(&mut self, index: usize) -> &mut Self {
        self.current_term = index;
        self
    }

    /// Adds a [`Term`] to the builder equivalent to [`With<T>`]
    pub fn with<T: Component>(&mut self) -> &mut Self {
        self.term::<With<T>>()
    }

    /// Adds a [`Term`] to the builder equivalent to [`With<T>`] where T is represented by `id`
    pub fn with_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::with_id(id));
        self
    }

    /// Adds a [`Term`] to the builder equivalent to [`Without<T>`]
    pub fn without<T: Component>(&mut self) -> &mut Self {
        self.term::<Without<T>>()
    }

    /// Adds a [`Term`] to the builder equivalent to [`Without<T>`] where T is represented by `id`
    pub fn without_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::without_id(id));
        self
    }

    /// Adds a [`Term`] to the builder equivalent to `&T` where T is represented by `id`
    pub fn ref_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::read_id(id));
        self
    }

    /// Adds a [`Term`] to the builder equivalent to `&mut T` where T is represented by `id`
    pub fn mut_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::write_id(id));
        self
    }

    /// Adds a [`Term`] to the builder equivalent to [`Added<T>`] where T is represented by `id`
    pub fn added_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::added_id(id));
        self
    }

    /// Adds a [`Term`] to the builder equivalent to [`Changed<T>`] where T is represented by `id`
    pub fn changed_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms.push(Term::changed_id(id));
        self
    }

    /// Takes a function over mutable access to a [`QueryBuilder`], calls that function
    /// on an empty builder and then adds all terms from that builder to self marked as
    /// [`TermAccess::Optional`]
    pub fn optional(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        let terms = builder
            .terms
            .into_iter()
            .map(|term| term.set_operator(TermOperator::Optional));
        self.terms.extend(terms);
        self
    }

    /// Takes a function over mutable access to a [`QueryBuilder`], calls that function
    /// on an empty builder, adds all terms from that builder as sub terms to an [`Or`]
    /// term which is then added to self
    pub fn or(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        let term = Term::or_terms(builder.terms);
        self.terms.push(term);
        self
    }

    /// Takes a function over mutable access to a [`QueryBuilder`], calls that function
    /// on an empty builder, adds all terms from that builder as sub terms to an [`AnyOf`]
    /// term which is added to self     
    pub fn any_of(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        let term = Term::any_of_terms(builder.terms);
        self.terms.push(term);
        self
    }

    /// Push a [`Term`] to the list of terms within the builder
    pub fn push(&mut self, term: Term) -> &mut Self {
        self.terms.push(term);
        self
    }

    /// Set the [`ComponentId`] of the term indexed by `current_term` to the one associated with `T`
    ///
    /// Intended to be used primarily with queries with [`Ptr`] terms
    pub fn set_dynamic<T: Component>(&mut self) -> &mut Self {
        let id = self.world.init_component::<T>();
        self.set_dynamic_by_id(id);
        self
    }

    /// Set the [`ComponentId`] of the term indexed by `current_term`
    ///
    /// Intended to be used primarily with queries with [`Ptr`] terms
    pub fn set_dynamic_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.terms[self.current_term].component = Some(id);
        self
    }

    /// Immutable access to the current list of [`Term`]s within the builder
    pub fn terms(&self) -> &Vec<Term> {
        &self.terms
    }

    /// Create a [`TermQueryState`] from the [`Term`]s within the builder
    pub fn build(&mut self) -> TermQueryState<Q> {
        // SAFETY: Terms are generated by Q unless modified using unsafe operations
        unsafe { TermQueryState::<Q>::from_terms(self.world, self.terms.clone()) }
    }
}
