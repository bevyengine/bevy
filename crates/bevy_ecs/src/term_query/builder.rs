use std::marker::PhantomData;

use crate::{component::ComponentId, prelude::*};

use super::{QueryFetchGroup, Term, TermFilter, TermQueryState};

/// Builder for [`TermQuery`](crate::system::TermQuery)
///
/// ```
/// use bevy_ecs::prelude::*;
///
/// #[derive(Component)]
/// struct A(usize);
///
/// #[derive(Component)]
/// struct B(usize);
///
/// #[derive(Component)]
/// struct C(usize);
///
/// let mut world = World::new();
/// let entity_a = world.spawn((A(0), B(0))).id();
/// let entity_b = world.spawn((A(0), C(0))).id();
///
/// // Instantiate the builder using the type signature of the iterator you will consume
/// let mut query_a = QueryBuilder::<(Entity, &B)>::new(&mut world)
/// // Add additional terms through builder methods
///     .with::<A>()
///     .without::<C>()
///     .build();
///
/// // Consume an iterator
/// let (entity, b) = query_a.single(&world);
///```
pub struct QueryBuilder<'w, Q: QueryFetchGroup = (), F: QueryFetchGroup = ()> {
    fetch_terms: Vec<Term>,
    filter_terms: Vec<Term>,
    world: &'w mut World,
    _marker: PhantomData<(Q, F)>,
}

impl<'w, Q: QueryFetchGroup, F: QueryFetchGroup> QueryBuilder<'w, Q, F> {
    /// Creates a new builder with the [`Term`]s represented by `Q`
    pub fn new(world: &'w mut World) -> Self {
        let mut fetch_terms = Vec::new();
        let mut filter_terms = Vec::new();
        Q::init_terms(world, &mut fetch_terms, 0);
        F::init_terms(world, &mut filter_terms, 0);
        Self {
            fetch_terms,
            filter_terms,
            world,
            _marker: PhantomData,
        }
    }

    /// Adds a term representing `T` to self
    pub fn fetch<T: QueryFetchGroup>(&mut self) -> &mut Self {
        T::init_terms(self.world, &mut self.fetch_terms, 0);
        self
    }

    /// Adds a term representing `T` to self
    pub fn filter<T: QueryFetchGroup>(&mut self) -> &mut Self {
        T::init_terms(self.world, &mut self.filter_terms, 0);
        self
    }

    /// Add a dynamic fetch term to the list of `fetch_terms`
    pub fn push_fetch(&mut self, term: Term) -> &mut Self {
        self.fetch_terms.push(term);
        self
    }

    /// Add a dynamic filter term to the list of `filter_terms`
    pub fn push_filter(&mut self, term: Term) -> &mut Self {
        self.filter_terms.push(term);
        self
    }

    /// Adds a [`Term`] to the builder equivalent to [`With<T>`]
    pub fn with<T: Component>(&mut self) -> &mut Self {
        self.filter::<With<T>>()
    }

    /// Adds a [`Term`] to the builder equivalent to [`With<T>`] where T is represented by `id`
    pub fn with_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.filter_terms.push(Term::new(0).with_id(id));
        self
    }

    /// Adds a [`Term`] to the builder equivalent to [`Without<T>`]
    pub fn without<T: Component>(&mut self) -> &mut Self {
        self.filter::<Without<T>>()
    }

    /// Adds a [`Term`] to the builder equivalent to [`Without<T>`] where T is represented by `id`
    pub fn without_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.filter_terms
            .push(Term::new(0).with_id(id).with_filter(TermFilter::Without));
        self
    }

    /// Adds a [`Term`] to the builder equivalent to `&T` where T is represented by `id`
    pub fn ref_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.fetch_terms
            .push(Term::new(0).with_id(id).with_access(TermAccess::Read));
        self
    }

    /// Adds a [`Term`] to the builder equivalent to `&mut T` where T is represented by `id`
    pub fn mut_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.fetch_terms
            .push(Term::new(0).with_id(id).with_access(TermAccess::Write));
        self
    }

    /// Adds a [`Term`] to the builder equivalent to [`Added<T>`] where T is represented by `id`
    pub fn added_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.filter_terms
            .push(Term::new(0).with_id(id).with_filter(TermFilter::Added));
        self
    }

    /// Adds a [`Term`] to the builder equivalent to [`Changed<T>`] where T is represented by `id`
    pub fn changed_by_id(&mut self, id: ComponentId) -> &mut Self {
        self.filter_terms
            .push(Term::new(0).with_id(id).with_filter(TermFilter::Changed));
        self
    }

    /// Takes a function over mutable access to a [`QueryBuilder`], calls that function
    /// on an empty builder and then adds all terms from that builder to self marked as
    /// [`TermOperator::Optional`]
    pub fn optional(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        let terms = builder
            .fetch_terms
            .into_iter()
            .map(|term| term.with_filter(TermFilter::Optional));
        self.fetch_terms.extend(terms);
        self
    }

    /// Takes a function over mutable access to a [`QueryBuilder`], calls that function
    /// on an empty builder, adds all `filter_terms` from that builder joined by `||` to this builder
    /// and all `fetch_terms` as as they are
    pub fn or(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        builder.fetch_terms.into_iter().for_each(|term| {
            self.fetch_terms.push(term);
        });
        builder.filter_terms.into_iter().for_each(|mut term| {
            if term.depth == 0 {
                term.or = true;
            }
            self.filter_terms.push(term);
        });
        if let Some(last) = self.filter_terms.last_mut() {
            last.or = false;
        }
        self
    }

    /// Takes a function over mutable access to a [`QueryBuilder`], calls that function
    /// on an empty builder, adds all `fetch_terms` from that builder joined by `||` to this builder
    /// and all `filter_terms` as as they are
    pub fn any_of(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        builder.fetch_terms.into_iter().for_each(|mut term| {
            if term.depth == 0 {
                term.or = true;
            }
            self.fetch_terms.push(term);
        });
        if let Some(last) = self.fetch_terms.last_mut() {
            last.or = false;
        }
        builder.filter_terms.into_iter().for_each(|term| {
            self.filter_terms.push(term);
        });
        self
    }

    /// Takes a function over mutable access to a [`QueryBuilder`], calls that function
    /// on an empty builder, adds all terms from that builder to this builder wrapped in braces
    pub fn scope(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        builder.fetch_terms.into_iter().for_each(|mut term| {
            term.depth += 1;
            self.fetch_terms.push(term);
        });
        builder.filter_terms.into_iter().for_each(|mut term| {
            term.depth += 1;
            self.filter_terms.push(term);
        });
        self
    }

    /// Push a [`Term`] to the list of terms within the builder
    pub fn push(&mut self, term: Term) -> &mut Self {
        self.fetch_terms.push(term);
        self
    }

    /// Set the [`ComponentId`] of the term indexed by `current_term` to the one associated with `T`
    ///
    /// Intended to be used primarily with queries with [`Ptr`](bevy_ptr::Ptr) terms
    ///
    /// # Safety
    ///
    /// - `index` must only index into dynamic fetch types e.g. `Ptr`, `PtrMut`
    pub unsafe fn set_dynamic<T: Component>(&mut self, index: usize) -> &mut Self {
        let id = self.world.init_component::<T>();
        self.set_dynamic_by_id(index, id);
        self
    }

    /// Set the [`ComponentId`] of the term indexed by `current_term`
    ///
    /// Intended to be used primarily with queries with [`Ptr`](bevy_ptr::Ptr) terms
    ///
    /// # Safety
    ///
    /// - `index` must only index into dynamic fetch types e.g. `Ptr`, `PtrMut`
    pub unsafe fn set_dynamic_by_id(&mut self, index: usize, id: ComponentId) -> &mut Self {
        self.fetch_terms[index].component = Some(id);
        self
    }

    /// Immutable access to the current list of [`Term`]s within the builder
    pub fn terms(&self) -> &Vec<Term> {
        &self.fetch_terms
    }

    /// Returns true if this builder could safely build a [`TermQueryState<NewQ>`]
    pub fn interpretable_as<NewQ: QueryFetchGroup, NewF: QueryFetchGroup>(&mut self) -> bool {
        let mut fetch_terms = Vec::new();
        let mut filter_terms = Vec::new();
        NewQ::init_terms(self.world, &mut fetch_terms, 0);
        NewF::init_terms(self.world, &mut filter_terms, 0);
        fetch_terms.iter().enumerate().all(|(i, a)| {
            self.fetch_terms
                .get(i)
                .is_some_and(|b| b.interpretable_as(a))
        }) && filter_terms.iter().enumerate().all(|(i, a)| {
            self.filter_terms
                .get(i)
                .is_some_and(|b| b.interpretable_as(a))
        })
    }

    /// Attempts to re-interpret this builder as [`QueryBuilder<NewQ>`]
    pub fn try_transmute<NewQ: QueryFetchGroup, NewF: QueryFetchGroup>(
        &mut self,
    ) -> Option<&mut QueryBuilder<'w, NewQ, NewF>> {
        if self.interpretable_as::<NewQ, NewF>() {
            // SAFETY: Just checked that NewQ is compatible with Q
            Some(unsafe { std::mem::transmute(self) })
        } else {
            None
        }
    }

    /// Re-interprets this builder as [`QueryBuilder<NewQ>`]
    ///
    /// # Safety
    ///
    /// Caller must ensure that [`Self::interpretable_as::<NewQ>()`] is true
    pub unsafe fn transmute<NewQ: QueryFetchGroup, NewF: QueryFetchGroup>(
        self,
    ) -> QueryBuilder<'w, NewQ, NewF> {
        std::mem::transmute(self)
    }

    /// Create a [`TermQueryState`] from the [`Term`]s within the builder
    pub fn build(&mut self) -> TermQueryState<Q, F> {
        // SAFETY: Terms are generated by Q unless modified using unsafe operations
        unsafe {
            TermQueryState::<Q, F>::from_terms(
                self.world,
                self.fetch_terms.clone(),
                self.filter_terms.clone(),
            )
        }
    }
}
