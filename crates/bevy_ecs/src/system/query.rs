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

/// Provides scoped access to components in a [`World`].
///
/// Queries allow to iterate over entities and their components as well as filtering them
/// on certain conditions. A query matches its parameters against the world to produce a series
/// of results. Each *query result* is a tuple of components (the same components defined
/// in the query) that belong to the same entity.
///
/// Query functionality is based on the [`WorldQuery`] trait. Both tuples of components
/// (up to 16 elements) and query filters implement this trait.
///
/// `Query` accepts two type parameters:
///
/// 1. **Component access:** the components that an entity must have at the same time to return
///    a query result.
/// 2. **Query filters (optional):** a predicate that ignores query results that don't match
///    its conditions.
///
/// # Usage as system parameter
///
/// A query is defined by declaring it as a system parameter. This section shows the various
/// use cases of `Query` as a system parameter.
///
/// ## Immutable component access
///
/// The following example defines a query that gives an iterator over `(&ComponentA, &ComponentB)`
/// tuples, where `ComponentA` and `ComponentB` belong to the same entity. Accessing components
/// immutably helps system parallelization.
///
/// ```
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # struct ComponentA;
/// # struct ComponentB;
/// # fn system(
/// query: Query<(&ComponentA, &ComponentB)>
/// # ) {}
/// # system.system();
/// ```
///
/// ## Mutable component access
///
/// The following example is similar to the previous one, with the exception of `ComponentA`
/// being accessed mutably here. Note that both mutable and immutable accesses are allowed
/// in the same query.
///
/// ```
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # struct ComponentA;
/// # struct ComponentB;
/// # fn system(
/// // `ComponentA` is accessed mutably, while `ComponentB` is accessed immutably.
/// mut query: Query<(&mut ComponentA, &ComponentB)>
/// # ) {}
/// # system.system();
/// ```
///
/// Two systems cannot be executed in parallel if both access a certain component and
/// at least one of the accesses is mutable, unless the schedule can verify that no entity
/// could be found in both queries.
///
/// ## Entity handle access
///
/// Inserting [`Entity`](crate::entity::Entity) at any position in the type parameter tuple
/// will give access to the entity handle.
///
/// ```
/// # use bevy_ecs::entity::Entity;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # struct ComponentA;
/// # struct ComponentB;
/// # fn system(
/// query: Query<(Entity, &ComponentA, &ComponentB)>
/// # ) {}
/// # system.system();
/// ```
///
/// ## Query filtering
///
/// The second, optional type parameter of query, is used for filters can be added to filter
/// out the query results that don't satisfy the given condition.
///
/// ```
/// # use bevy_ecs::query::With;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # struct ComponentA;
/// # struct ComponentB;
/// # struct ComponentC;
/// # fn system(
/// // `ComponentC` data won't be accessed, but only entities that contain it will be queried.
/// query: Query<(&ComponentA, &ComponentB), With<ComponentC>>
/// # ) {}
/// # system.system();
/// ```
///
/// If you need to apply more filters in a single query, group them into a tuple:
///
/// ```
/// # use bevy_ecs::query::{Changed, With};
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # struct ComponentA;
/// # struct ComponentB;
/// # struct ComponentC;
/// # fn system(
/// // Similar to the previous query, but with the addition of a `Changed` filter.
/// query: Query<(&ComponentA, &ComponentB), (With<ComponentC>, Changed<ComponentA>)>
/// # ) {}
/// # system.system();
/// ```
///
/// See the [`query`](crate::query) module for a full list of available filters.
///
/// ## Optional component access
///
/// A component can be made optional in a query by wrapping it into an [`Option`]. In the
/// following example, the query will iterate over components of both entities that contain
/// `ComponentA` and `ComponentB`, and entities that contain `ComponentA` but not `ComponentB`.
///
/// ```
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # struct ComponentA;
/// # struct ComponentB;
/// # fn system(
/// query: Query<(&ComponentA, Option<&ComponentB>)>
/// # ) {}
/// # system.system();
/// ```
///
/// If an entity does not contain a component, its corresponding query result value will be
/// `None`. Optional components increase the number of entities a query has to match against,
/// therefore they can hurt iteration performance, especially in the worst case scenario where
/// the query solely consists of only optional components, since all entities will be iterated
/// over.
///
/// ## Single component access
///
/// If just a single component needs to be accessed, using a tuple as the first type parameter
/// of `Query` can be omitted.
///
/// ```
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # struct MyComponent;
/// # fn tuple_system(
/// // This is correct, but can be avoided.
/// query: Query<(&MyComponent,)>
/// # ) {}
/// # tuple_system.system();
///
/// # fn non_tuple_system(
/// // This is the preferred method.    
/// query: Query<&MyComponent>
/// # ) {}
/// # non_tuple_system.system();
/// ```
///
/// # Usage of query results
///
/// Inside the body of the system function, the `Query` is available as a function parameter.
/// This section shows various methods to access query results.
///
/// ## Iteration over every query result
///
/// The [`iter`](Self::iter) and [`iter_mut`](Self::iter_mut) methods are used to iterate
/// over every query result. Refer to the
/// [`Iterator` API docs](https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html)
/// for advanced iterator usage.
///
/// ```
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # struct ComponentA;
/// # struct ComponentB;
/// fn immutable_query_system(mut query: Query<(&ComponentA, &ComponentB)>) {
///     for (a, b) in query.iter() {
///         // Here, `a` and `b` are normal references to components, relatively of
///         // `&ComponentA` and `&ComponentB` types.
///     }
/// }
/// # immutable_query_system.system();
///
/// fn mutable_query_system(mut query: Query<(&mut ComponentA, &ComponentB)>) {
///     for (mut a, b) in query.iter_mut() {
///         // Similar to the above system, but this time `ComponentA` can be accessed mutably.
///         // Note the usage of `mut` in the tuple and the call to `iter_mut` instead of `iter`.
///     }
/// }
/// # mutable_query_system.system();
/// ```
///
/// ## Getting the query result for a particular entity
///
/// If you have an [`Entity`] handle, you can use the [`get`](Self::get) or
/// [`get_mut`](Self::get_mut) methods to access the query result for that particular entity.
///
/// ## Getting a single query result
///
/// While it's possible to get a single result from a query by using `iter.next()`, a more
/// idiomatic approach would use the [`single`](Self::single) or [`single_mut`](Self::single_mut)
/// methods instead. Keep in mind though that they will return a [`QuerySingleError`] if the
/// number of query results differ from being exactly one. If that's the case, use `iter.next()`
/// (or `iter_mut.next()`) to only get the first query result.
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
    /// This can only be called for read-only queries (due to the [`ReadOnlyFetch`] trait
    /// bound). See [`Self::iter_mut`] for queries that contain at least one mutable component.
    ///
    /// # Example
    ///
    /// In this example, the `report_names_system` iterates over the `Player` component of
    /// all the entities that contains it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct Player { name: String }
    /// #
    /// fn report_names_system(query: Query<&Player>) {
    ///     for player in query.iter() {
    ///         println!("Say hello to {}!", player.name);
    ///     }
    /// }
    /// # report_names_system.system();
    /// ```
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
    ///
    /// # Example
    ///
    /// In this example, the `gravity_system` iterates over the `Velocity` component of every
    /// entity in the world that contains it in order to update it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct Velocity { x: f32, y: f32, z: f32 }
    /// fn gravity_system(mut query: Query<&mut Velocity>) {
    ///     const DELTA: f32 = 1.0 / 60.0;
    ///     for mut velocity in query.iter_mut() {
    ///         velocity.y -= 9.8 * DELTA;
    ///     }
    /// }
    /// # gravity_system.system();
    /// ```
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
    ///
    /// # Example
    ///
    /// In this example, the `report_names_system` iterates over the `Player` component of
    /// all the entities that contains it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct Player { name: String }
    /// #
    /// fn report_names_system(query: Query<&Player>) {
    ///     query.for_each(|player| {
    ///         println!("Say hello to {}!", player.name);
    ///     });
    /// }
    /// # report_names_system.system();
    /// ```
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
    ///
    /// # Example
    ///
    /// In this example, the `gravity_system` iterates over the `Velocity` component of every
    /// entity in the world that contains it in order to update it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct Velocity { x: f32, y: f32, z: f32 }
    /// fn gravity_system(mut query: Query<&mut Velocity>) {
    ///     const DELTA: f32 = 1.0 / 60.0;
    ///     query.for_each_mut(|mut velocity| {
    ///         velocity.y -= 9.8 * DELTA;
    ///     });
    /// }
    /// # gravity_system.system();
    /// ```
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
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// This can only be called for read-only queries (due to the [`ReadOnlyFetch`] trait bound).
    /// see [`get_mut`](Self::get_mut) for queries that contain at least one mutable component.
    ///
    /// # Example
    ///
    /// In the following system, the `Entity` handle contained in the `winner` resource is
    /// used to get the `Person` component of that entity.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct Person { name: String, credits: u32 }
    /// # struct Winner { entity: Entity }
    /// #
    /// fn check_credits_system(query: Query<&Person>, winner: Res<Winner>) {
    ///     if let Ok(person) = query.get(winner.entity) {
    ///         if person.credits > 35000 {
    ///             println!("{} won a prize!", person.name);
    ///         }
    ///     }
    /// }
    /// # check_credits_system.system();
    /// ```
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
