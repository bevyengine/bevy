use crate::{
    component::Component,
    entity::Entity,
    query::{
        Fetch, FilterFetch, QueryCombinationIter, QueryEntityError, QueryIter, QueryState,
        ReadOnlyFetch, WorldQuery,
    },
    world::{Mut, World},
};
use bevy_tasks::TaskPool;
use std::{any::TypeId, fmt::Debug};
use thiserror::Error;

/// Provides scoped access to a [`World`] according to a given [`WorldQuery`] and query filter.
///
/// Queries are a powerful tool enabling the programmer to iterate over entities and their components
/// as well as filtering them on certain conditions.
///
/// # Query Building Primer
///
/// ### Basic Component Access
///
/// A basic query looks like `Query<&UnitHealth>` and all it does is grant immutable access to all
/// `UnitHealth` components. Similarly using `&mut UnitHealth` instead grants mutable access instead.
///
/// The main way to access the components of a query is through the [`Query::iter`] and [`Query::iter_mut`]
/// functions which return a [`QueryIter`] to iterate over:
///
/// ```
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// struct UnitHealth(pub u32);
/// fn system(query: Query<&UnitHealth>) {
///     for UnitHealth(health) in query.iter() {
///         println!("We got {} health points left!", health);
///     }
/// }
/// # system.system();
/// ```
///
/// ### Multiple Component Access
///
/// Instead of asking for just one component like before we can build a query that queries for multiple
/// components with the help of tuples,`Query<(&Shape, &Color, &mut Size)>`. This query retrieves
/// immutable references to the `Shape` and `Color` component and a mutable reference to the `Size`
/// component.
///
/// ```
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// #[derive(Debug)]
/// enum Shape {
///     Circle,
///     Box,
/// };
/// struct Color(pub String);
/// struct Size(pub u32);
/// fn system(mut query: Query<(&Shape, &Color, &mut Size)>) {
///     for (shape, color, mut size) in query.iter_mut() {
///         *size = Size(1);
///         println!("We got a {} colored {:?} and made it one unit big!", color.0, shape);
///     }
/// }
/// # system.system();
/// ```
///
/// Note the use of [`Query::iter_mut`] here, as our query is not read-only anymore due to the use
/// of the `&mut` [`WorldQuery`] we aren't able to use the `iter` method any longer.
///
/// ### Filtering Query Results
///
/// Queries also support filters. A filter is a [`WorldQuery`] that can be used as a predicate to
/// filter out entities that do not meet the requirement set by the predicate. [`With`](crate::query::With)
/// is one such filter and all it does is filter out all entities that do not contain the component
/// it requests. Let's look at an example on how to use this filter.
///
/// ```
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # use bevy_ecs::query::With;
/// struct Person(String);
/// struct IsTallEnough;
/// fn system(query: Query<&Person, With<IsTallEnough>>) {
///     for person in query.iter() {
///         println!("{} is tall enough!", person.0);
///     }
/// }
/// # system.system();
/// ```
///
/// As shown above, the filter is a second type parameter of the query. It is optional (defaults to
/// ()). Filters do not give access to the component data, only limit the entities that the query will match.
///
/// ### Optional Components
///
/// Now we've seen how to narrow down results of a query, but what if we want to act on entities that
/// may have a component but not always. This is where [`Option`] comes into play, with `Option` we
/// can specify just that. The result of the following query, `Query<&Color, Option<&mut Size>>`, is
/// the tuple `(&Color, Option<&mut Size>)` containing all entities that have the `Color` component,
/// some of which also have a `Size` component. Note that we didn't put a [`Component`] inside the
/// `Option` but a [`WorldQuery`], `&mut T` in this case. This means we can also do the following
/// just fine, `Query<Option<(&Size, &Color)>>`.
///
/// Do take care when handling optional components though, as iterating a query that solely consists
/// of optional components will go over all the entities of the [`World`]. Therefore it's best to
/// design your queries in such a way that they at least contain one non-optional [`WorldQuery`].
///
/// This touches all the basics of queries, make sure to check out all the [`WorldQueries`](WorldQuery)
/// bevy has to offer.
pub struct Query<'w, Q: WorldQuery, F: WorldQuery = ()>
where
    F::Fetch: FilterFetch,
{
    pub(crate) world: &'w World,
    pub(crate) state: &'w QueryState<Q, F>,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

impl<'w, Q: WorldQuery, F: WorldQuery> Query<'w, Q, F>
where
    F::Fetch: FilterFetch,
{
    /// Creates a new query.
    ///
    /// # Safety
    ///
    /// This will create a query that could violate memory safety rules. Make sure that this is only
    /// called in ways that ensure the queries have unique mutable access.
    #[inline]
    pub(crate) unsafe fn new(
        world: &'w World,
        state: &'w QueryState<Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        Self {
            world,
            state,
            last_change_tick,
            change_tick,
        }
    }

    /// Returns an [`Iterator`] over the query results.
    ///
    /// This can only be called for read-only queries, see [`Self::iter_mut`] for write-queries.
    #[inline]
    pub fn iter(&self) -> QueryIter<'_, '_, Q, F>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .iter_unchecked_manual(self.world, self.last_change_tick, self.change_tick)
        }
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results without repetition.
    /// This can only be called for read-only queries
    ///
    ///  For permutations of size K of query returning N results, you will get:
    /// - if K == N: one permutation of all query results
    /// - if K < N: all possible K-sized combinations of query results, without repetition
    /// - if K > N: empty set (no K-sized combinations exist)
    #[inline]
    pub fn iter_combinations<const K: usize>(&self) -> QueryCombinationIter<'_, '_, Q, F, K>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.iter_combinations_unchecked_manual(
                self.world,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns an [`Iterator`] over the query results.
    #[inline]
    pub fn iter_mut(&mut self) -> QueryIter<'_, '_, Q, F> {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .iter_unchecked_manual(self.world, self.last_change_tick, self.change_tick)
        }
    }

    /// Iterates over all possible combinations of `K` query results without repetition.
    ///
    /// The returned value is not an `Iterator`, because that would lead to aliasing of mutable references.
    /// In order to iterate it, use `fetch_next` method with `while let Some(..)` loop pattern.
    ///
    /// ```
    /// # struct A;
    /// # use bevy_ecs::prelude::*;
    /// # fn some_system(mut query: Query<&mut A>) {
    /// // iterate using `fetch_next` in while loop
    /// let mut combinations = query.iter_combinations_mut();
    /// while let Some([mut a, mut b]) = combinations.fetch_next() {
    ///    // mutably access components data
    /// }
    /// # }
    /// ```
    ///
    /// There is no `for_each` method, because it cannot be safely implemented
    /// due to a [compiler bug](https://github.com/rust-lang/rust/issues/62529).
    ///
    /// For immutable access see [`Query::iter_combinations`].
    #[inline]
    pub fn iter_combinations_mut<const K: usize>(
        &mut self,
    ) -> QueryCombinationIter<'_, '_, Q, F, K> {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.iter_combinations_unchecked_manual(
                self.world,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns an [`Iterator`] over the query results.
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees. You must make sure
    /// this call does not result in multiple mutable references to the same component
    #[inline]
    pub unsafe fn iter_unsafe(&self) -> QueryIter<'_, '_, Q, F> {
        // SEMI-SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state
            .iter_unchecked_manual(self.world, self.last_change_tick, self.change_tick)
    }

    /// Iterates over all possible combinations of `K` query results without repetition.
    /// See [`Query::iter_combinations`].
    ///
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple
    /// mutable references to the same component
    #[inline]
    pub unsafe fn iter_combinations_unsafe<const K: usize>(
        &self,
    ) -> QueryCombinationIter<'_, '_, Q, F, K> {
        // SEMI-SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state.iter_combinations_unchecked_manual(
            self.world,
            self.last_change_tick,
            self.change_tick,
        )
    }

    /// Runs `f` on each query result. This is faster than the equivalent iter() method, but cannot
    /// be chained like a normal [`Iterator`].
    ///
    /// This can only be called for read-only queries, see [`Self::for_each_mut`] for write-queries.
    #[inline]
    pub fn for_each(&self, f: impl FnMut(<Q::Fetch as Fetch<'w>>::Item))
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.for_each_unchecked_manual(
                self.world,
                f,
                self.last_change_tick,
                self.change_tick,
            )
        };
    }

    /// Runs `f` on each query result. This is faster than the equivalent iter() method, but cannot
    /// be chained like a normal [`Iterator`].
    #[inline]
    pub fn for_each_mut(&mut self, f: impl FnMut(<Q::Fetch as Fetch<'w>>::Item)) {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.for_each_unchecked_manual(
                self.world,
                f,
                self.last_change_tick,
                self.change_tick,
            )
        };
    }

    /// Runs `f` on each query result in parallel using the given task pool.
    ///
    /// This can only be called for read-only queries, see [`Self::par_for_each_mut`] for
    /// write-queries.
    #[inline]
    pub fn par_for_each(
        &self,
        task_pool: &TaskPool,
        batch_size: usize,
        f: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.par_for_each_unchecked_manual(
                self.world,
                task_pool,
                batch_size,
                f,
                self.last_change_tick,
                self.change_tick,
            )
        };
    }

    /// Runs `f` on each query result in parallel using the given task pool.
    #[inline]
    pub fn par_for_each_mut(
        &mut self,
        task_pool: &TaskPool,
        batch_size: usize,
        f: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.par_for_each_unchecked_manual(
                self.world,
                task_pool,
                batch_size,
                f,
                self.last_change_tick,
                self.change_tick,
            )
        };
    }

    /// Gets the query result for the given [`Entity`].
    ///
    /// This can only be called for read-only queries, see [`Self::get_mut`] for write-queries.
    #[inline]
    pub fn get(&self, entity: Entity) -> Result<<Q::Fetch as Fetch>::Item, QueryEntityError>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.get_unchecked_manual(
                self.world,
                entity,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Gets the query result for the given [`Entity`].
    #[inline]
    pub fn get_mut(
        &mut self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, QueryEntityError> {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.get_unchecked_manual(
                self.world,
                entity,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Gets the query result for the given [`Entity`].
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees. You must make sure
    /// this call does not result in multiple mutable references to the same component
    #[inline]
    pub unsafe fn get_unchecked(
        &self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, QueryEntityError> {
        // SEMI-SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state
            .get_unchecked_manual(self.world, entity, self.last_change_tick, self.change_tick)
    }

    /// Gets a reference to the [`Entity`]'s [`Component`] of the given type. This will fail if the
    /// entity does not have the given component type or if the given component type does not match
    /// this query.
    #[inline]
    pub fn get_component<T: Component>(&self, entity: Entity) -> Result<&T, QueryComponentError> {
        let world = self.world;
        let entity_ref = world
            .get_entity(entity)
            .ok_or(QueryComponentError::NoSuchEntity)?;
        let component_id = world
            .components()
            .get_id(TypeId::of::<T>())
            .ok_or(QueryComponentError::MissingComponent)?;
        let archetype_component = entity_ref
            .archetype()
            .get_archetype_component_id(component_id)
            .ok_or(QueryComponentError::MissingComponent)?;
        if self
            .state
            .archetype_component_access
            .has_read(archetype_component)
        {
            entity_ref
                .get::<T>()
                .ok_or(QueryComponentError::MissingComponent)
        } else {
            Err(QueryComponentError::MissingReadAccess)
        }
    }

    /// Gets a mutable reference to the [`Entity`]'s [`Component`] of the given type. This will fail
    /// if the entity does not have the given component type or if the given component type does not
    /// match this query.
    #[inline]
    pub fn get_component_mut<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        // SAFE: unique access to query (preventing aliased access)
        unsafe { self.get_component_unchecked_mut(entity) }
    }

    /// Gets a mutable reference to the [`Entity`]'s [`Component`] of the given type. This will fail
    /// if the entity does not have the given component type or the component does not match the
    /// query.
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees. You must make sure
    /// this call does not result in multiple mutable references to the same component
    #[inline]
    pub unsafe fn get_component_unchecked_mut<T: Component>(
        &self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        let world = self.world;
        let entity_ref = world
            .get_entity(entity)
            .ok_or(QueryComponentError::NoSuchEntity)?;
        let component_id = world
            .components()
            .get_id(TypeId::of::<T>())
            .ok_or(QueryComponentError::MissingComponent)?;
        let archetype_component = entity_ref
            .archetype()
            .get_archetype_component_id(component_id)
            .ok_or(QueryComponentError::MissingComponent)?;
        if self
            .state
            .archetype_component_access
            .has_write(archetype_component)
        {
            entity_ref
                .get_unchecked_mut::<T>(self.last_change_tick, self.change_tick)
                .ok_or(QueryComponentError::MissingComponent)
        } else {
            Err(QueryComponentError::MissingWriteAccess)
        }
    }

    /// Gets the result of a single-result query.
    ///
    /// If the query has exactly one result, returns the result inside `Ok`
    /// otherwise returns either [`QuerySingleError::NoEntities`]
    /// or [`QuerySingleError::MultipleEntities`], as appropriate.
    ///
    /// # Examples
    ///
    /// ```
    ///  # use bevy_ecs::system::{Query, QuerySingleError};
    ///  # use bevy_ecs::prelude::IntoSystem;
    /// struct PlayerScore(i32);
    /// fn player_scoring_system(query: Query<&PlayerScore>) {
    ///     match query.single() {
    ///         Ok(PlayerScore(score)) => {
    ///             // do something with score
    ///         }
    ///         Err(QuerySingleError::NoEntities(_)) => {
    ///             // no PlayerScore
    ///         }
    ///         Err(QuerySingleError::MultipleEntities(_)) => {
    ///             // multiple PlayerScore
    ///         }
    ///     }
    /// }
    /// # let _check_that_its_a_system = player_scoring_system.system();
    /// ```
    ///
    /// This can only be called for read-only queries, see [`Self::single_mut`] for write-queries.
    pub fn single(&self) -> Result<<Q::Fetch as Fetch<'_>>::Item, QuerySingleError>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        let mut query = self.iter();
        let first = query.next();
        let extra = query.next().is_some();

        match (first, extra) {
            (Some(r), false) => Ok(r),
            (None, _) => Err(QuerySingleError::NoEntities(std::any::type_name::<Self>())),
            (Some(_), _) => Err(QuerySingleError::MultipleEntities(std::any::type_name::<
                Self,
            >())),
        }
    }

    /// Gets the query result if it is only a single result, otherwise returns a
    /// [`QuerySingleError`].
    pub fn single_mut(&mut self) -> Result<<Q::Fetch as Fetch<'_>>::Item, QuerySingleError> {
        let mut query = self.iter_mut();
        let first = query.next();
        let extra = query.next().is_some();

        match (first, extra) {
            (Some(r), false) => Ok(r),
            (None, _) => Err(QuerySingleError::NoEntities(std::any::type_name::<Self>())),
            (Some(_), _) => Err(QuerySingleError::MultipleEntities(std::any::type_name::<
                Self,
            >())),
        }
    }

    /// Returns true if this query contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        // TODO: This code can be replaced with `self.iter().next().is_none()` if/when
        // we sort out how to convert "write" queries to "read" queries.
        self.state
            .is_empty(self.world, self.last_change_tick, self.change_tick)
    }
}

/// An error that occurs when retrieving a specific [`Entity`]'s component from a [`Query`]
#[derive(Error, Debug)]
pub enum QueryComponentError {
    #[error("This query does not have read access to the requested component.")]
    MissingReadAccess,
    #[error("This query does not have read access to the requested component.")]
    MissingWriteAccess,
    #[error("The given entity does not have the requested component.")]
    MissingComponent,
    #[error("The requested entity does not exist.")]
    NoSuchEntity,
}

/// An error that occurs when evaluating a [`Query`] as a single expected resulted via
/// [`Query::single`] or [`Query::single_mut`].
#[derive(Debug, Error)]
pub enum QuerySingleError {
    #[error("No entities fit the query {0}")]
    NoEntities(&'static str),
    #[error("Multiple entities fit the query {0}!")]
    MultipleEntities(&'static str),
}
