use std::marker::PhantomData;

use crate::{
    component::Tick,
    entity::Entity,
    prelude::TermQueryState,
    query::{QueryEntityError, QuerySingleError},
    term_query::{QueryTermGroup, ROTermItem, TermQueryIter, TermQueryIterUntyped},
    world::unsafe_world_cell::UnsafeWorldCell,
};

/// [System parameter] that provides selective access to the [`Component`] data stored in a [`World`].
///
/// This is broadly equivalent to [`Query`] supporting the same type API and near identical methods.
/// In all non-dynamic cases a [`Query`] will out-perform an equivalent [`TermQuery`]
///
/// For more information on dynamically building a [`TermQuery`] see [`QueryBuilder`]
///
/// [System parameter]: crate::system::SystemParam
/// [`QueryBuilder`]: crate::term_query::QueryBuilder
/// [`Query`]: crate::system::Query
/// [`Component`]: crate::component::Component
/// [`World`]: crate::World
pub struct TermQuery<'w, 's, Q: QueryTermGroup, F: QueryTermGroup = ()> {
    // SAFETY: Must have access to the components registered in `state`.
    world: UnsafeWorldCell<'w>,
    state: &'s TermQueryState<Q, F>,
    last_run: Tick,
    this_run: Tick,
    _marker: PhantomData<Q>,
}

impl<'w, 's, Q: QueryTermGroup, F: QueryTermGroup> TermQuery<'w, 's, Q, F> {
    /// Creates a new term query.
    ///
    /// # Panics
    ///
    /// This will panic if the world used to create `state` is not `world`.
    ///
    /// # Safety
    ///
    /// This will create a query that could violate memory safety rules. Make sure that this is only
    /// called in ways that ensure the queries have unique mutable access.
    pub fn new(
        world: UnsafeWorldCell<'w>,
        state: &'s TermQueryState<Q, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        state.validate_world(world.id());

        Self {
            world,
            state,
            last_run,
            this_run,
            _marker: PhantomData::default(),
        }
    }

    /// Returns an [`Iterator`] over the read-only query items.
    ///
    /// # See also
    ///
    /// - [`iter_mut`](Self::iter_mut) for mutable query items.
    /// - [`for_each`](Self::for_each) for the closure based alternative.
    /// - [`Query::iter`](crate::query::Query::iter) for more examples
    #[inline]
    pub fn iter(&self) -> TermQueryIter<'_, 's, Q::ReadOnly> {
        // SAFETY:
        // - `self.world` has permission to access the required components.
        // - The query is read-only, so it can be aliased even if it was originally mutable.
        unsafe {
            self.state
                .as_readonly()
                .iter_unchecked_manual(self.world, self.last_run, self.this_run)
        }
    }

    /// Returns an [`Iterator`] over the query items.
    ///
    /// # See also
    ///
    /// - [`iter`](Self::iter) for read-only query items.
    /// - [`for_each_mut`](Self::for_each_mut) for the closure based alternative.
    /// - [`Query::iter_mut`](crate::query::Query::iter_mut) for more examples
    #[inline]
    pub fn iter_mut(&mut self) -> TermQueryIter<'_, 's, Q> {
        // SAFETY: `self.world` has permission to access the required components.
        unsafe {
            self.state
                .iter_unchecked_manual(self.world, self.last_run, self.this_run)
        }
    }

    /// Returns an untyped [`Iterator`] over the query items.
    /// Provides access to a list of the [`Term`]s used to resolve this query as well as
    /// a list of the resolved [`FetchedTerm`]
    ///
    /// # Example
    ///
    /// Here, the `update_system` increments all terms with write access.
    /// For a more advanced use case see the `dynamic_query` example.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct A(usize);
    /// #
    /// # #[derive(Component)]
    /// # struct B(usize);
    /// #
    /// # #[derive(Component)]
    /// # struct C(usize);
    ///
    /// fn update_system(mut query: TermQuery<(&mut A, &B, &mut C)>) {
    ///     query.iter_raw().for_each(|terms| {
    ///         terms.for_each(|(term, fetch)| {
    ///             if term.access == TermAccess::Write {
    ///                 // Since all the components have the same layout we can cast them all to the same value
    ///                 let component = <mut &A>::from_fetch(fetch);
    ///                 component += 1;
    ///             }
    ///         })
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(update_system);
    /// ```
    #[inline]
    pub fn iter_raw(&mut self) -> TermQueryIterUntyped<'_, 's> {
        unsafe {
            self.state
                .iter_raw_manual(self.world, self.last_run, self.this_run)
        }
    }

    /// Returns the read-only query item for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// # See also
    ///
    /// - [`get_mut`](Self::get_mut) to get a mutable query item.
    /// - [`Query::get_mut`](crate::query::Query::get_mut) for more examples
    #[inline]
    pub fn get(&self, entity: Entity) -> Result<ROTermItem<'_, Q>, QueryEntityError> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.as_readonly().get_unchecked_manual(
                self.world,
                entity,
                self.last_run,
                self.this_run,
            )
        }
    }

    /// Returns the query item for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// # See also
    ///
    /// - [`get`](Self::get) to get a read-only query item.
    /// - [`Query::get`](crate::query::Query::get) for more examples
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Result<Q::Item<'_>, QueryEntityError> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .get_unchecked_manual(self.world, entity, self.last_run, self.this_run)
        }
    }

    /// Returns a single read-only query item when there is exactly one entity matching the query.
    ///
    /// # See also
    ///
    /// - [`get_single`](Self::get_single) for the non-panicking version.
    /// - [`single_mut`](Self::single_mut) to get the mutable query item.
    /// - [`Query::single`](crate::query::Query::single) for more examples
    pub fn single(&self) -> ROTermItem<'_, Q> {
        self.get_single().unwrap()
    }

    /// Returns a single read-only query item when there is exactly one entity matching the query.
    ///
    /// If the number of query items is not exactly one, a [`QuerySingleError`] is returned instead.
    ///
    /// # See also
    ///
    /// - [`get_single_mut`](Self::get_single_mut) to get the mutable query item.
    /// - [`single`](Self::single) for the panicking version.
    /// - [`Query::get_single`](crate::query::Query::get_single) for more examples
    #[inline]
    pub fn get_single(&self) -> Result<ROTermItem<'_, Q>, QuerySingleError> {
        // SAFETY:
        // the query ensures that the components it accesses are not mutably accessible somewhere else
        // and the query is read only.
        unsafe {
            self.state.as_readonly().get_single_unchecked_manual(
                self.world,
                self.last_run,
                self.this_run,
            )
        }
    }

    /// Returns a single query item when there is exactly one entity matching the query.
    ///
    /// # See also
    ///
    /// - [`get_single_mut`](Self::get_single_mut) for the non-panicking version.
    /// - [`single`](Self::single) to get the read-only query item.
    /// - [`Query::single_mut`](crate::query::Query::single_mut) for more examples
    pub fn single_mut(&mut self) -> Q::Item<'_> {
        self.get_single_mut().unwrap()
    }

    /// Returns a single query item when there is exactly one entity matching the query.
    ///
    /// If the number of query items is not exactly one, a [`QuerySingleError`] is returned instead.
    ///
    /// # See also
    ///
    /// - [`get_single`](Self::get_single) to get the read-only query item.
    /// - [`single_mut`](Self::single_mut) for the panicking version.
    /// - [`Query::get_single_mut`](crate::query::Query::get_single_mut) for more examples
    #[inline]
    pub fn get_single_mut(&mut self) -> Result<Q::Item<'_>, QuerySingleError> {
        // SAFETY:
        // the query ensures mutable access to the components it accesses, and the query
        // is uniquely borrowed
        unsafe {
            self.state
                .get_single_unchecked_manual(self.world, self.last_run, self.this_run)
        }
    }
}

impl<'w, 's, Q: QueryTermGroup, F: QueryTermGroup> IntoIterator for &'w TermQuery<'_, 's, Q, F> {
    type Item = <Q::ReadOnly as QueryTermGroup>::Item<'w>;
    type IntoIter = TermQueryIter<'w, 's, Q::ReadOnly>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            TermQueryIter::new(
                self.world,
                self.state.as_readonly(),
                self.last_run,
                self.this_run,
            )
        }
    }
}

impl<'w, 's, Q: QueryTermGroup, F: QueryTermGroup> IntoIterator
    for &'w mut TermQuery<'_, 's, Q, F>
{
    type Item = Q::Item<'w>;
    type IntoIter = TermQueryIter<'w, 's, Q>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            TermQueryIter::new(
                self.world,
                self.state.filterless(),
                self.last_run,
                self.this_run,
            )
        }
    }
}
