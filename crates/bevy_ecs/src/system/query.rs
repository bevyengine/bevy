use crate::{
    component::Component,
    entity::Entity,
    query::{
        QueryCombinationIter, QueryEntityError, QueryItem, QueryIter, QueryManyIter,
        QuerySingleError, QueryState, ROQueryItem, ReadOnlyWorldQuery, WorldQuery,
    },
    world::{Mut, World},
};
use std::{any::TypeId, borrow::Borrow, fmt::Debug};

/// Provides scoped access to components in a [`World`].
///
/// Queries enable iteration over entities and their components as well as filtering them
/// on certain conditions. A query matches its parameters against the world to produce a series
/// of results. Each *query result* is a tuple of components (the same components defined
/// in the query) that belong to the same entity.
///
/// Computational cost of queries is reduced by the fact that they have an internal archetype
/// cache to avoid re-computing archetype matches on each query access.
///
/// Query functionality is based on the [`WorldQuery`] trait. Both tuples of components
/// (up to 16 elements) and query filters implement this trait.
///
/// `Query` accepts two type parameters:
///
/// 1. **Component access:** the components that an entity must have at the same time to yield
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
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # fn system(
/// query: Query<(&ComponentA, &ComponentB)>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// ## Mutable component access
///
/// The following example is similar to the previous one, with the exception of `ComponentA`
/// being accessed mutably here. Note that both mutable and immutable accesses are allowed
/// in the same query.
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # fn system(
/// // `ComponentA` is accessed mutably, while `ComponentB` is accessed immutably.
/// mut query: Query<(&mut ComponentA, &ComponentB)>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// Two systems cannot be executed in parallel if both access a certain component and
/// at least one of the accesses is mutable, unless the schedule can verify that no entity
/// could be found in both queries, as otherwise Rusts mutability Rules would be broken.
///
/// Similarly, a system cannot contain two queries that would break Rust's mutability Rules.
/// If you need such Queries, you can use Filters to make the Queries disjoint or use a
/// [`ParamSet`](super::ParamSet).
///
/// ## Entity ID access
///
/// Inserting [`Entity`](crate::entity::Entity) at any position in the type parameter tuple
/// will give access to the entity ID.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # fn system(
/// query: Query<(Entity, &ComponentA, &ComponentB)>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// ## Query filtering
///
/// The second, optional type parameter of query, is used for filters can be added to filter
/// out the query results that don't satisfy the given condition.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # #[derive(Component)]
/// # struct ComponentC;
/// # fn system(
/// // `ComponentC` data won't be accessed, but only entities that contain it will be queried.
/// query: Query<(&ComponentA, &ComponentB), With<ComponentC>>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// If you need to apply more filters in a single query, group them into a tuple:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # #[derive(Component)]
/// # struct ComponentC;
/// # fn system(
/// // Similar to the previous query, but with the addition of a `Changed` filter.
/// query: Query<(&ComponentA, &ComponentB), (With<ComponentC>, Changed<ComponentA>)>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// The following list contains all the available query filters:
///
/// - [`Added`](crate::query::Added)
/// - [`Changed`](crate::query::Changed)
/// - [`With`](crate::query::With)
/// - [`Without`](crate::query::Without)
/// - [`Or`](crate::query::Or)
///
/// ## Optional component access
///
/// A component can be made optional in a query by wrapping it into an [`Option`]. In the
/// following example, the query will iterate over components of both entities that contain
/// `ComponentA` and `ComponentB`, and entities that contain `ComponentA` but not `ComponentB`.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # fn system(
/// query: Query<(&ComponentA, Option<&ComponentB>)>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(system);
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
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct MyComponent;
/// # fn tuple_system(
/// // This is correct, but can be avoided.
/// query: Query<(&MyComponent,)>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(tuple_system);
///
/// # fn non_tuple_system(
/// // This is the preferred method.
/// query: Query<&MyComponent>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(non_tuple_system);
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
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// fn immutable_query_system(query: Query<(&ComponentA, &ComponentB)>) {
///     for (a, b) in &query {
///         // Here, `a` and `b` are normal references to components, relatively of
///         // `&ComponentA` and `&ComponentB` types.
///     }
/// }
/// # bevy_ecs::system::assert_is_system(immutable_query_system);
///
/// fn mutable_query_system(mut query: Query<(&mut ComponentA, &ComponentB)>) {
///     for (mut a, b) in &mut query {
///         // Similar to the above system, but this time `ComponentA` can be accessed mutably.
///         // Note the usage of `mut` in the tuple and the call to `iter_mut` instead of `iter`.
///     }
/// }
/// # bevy_ecs::system::assert_is_system(mutable_query_system);
/// ```
///
/// ## Getting the query result for a particular entity
///
/// If you have an [`Entity`] ID, you can use the [`get`](Self::get) or
/// [`get_mut`](Self::get_mut) methods to access the query result for that particular entity.
///
/// ## Getting a single query result
///
/// While it's possible to get a single result from a query by using `iter.next()`, a more
/// idiomatic approach would use the [`single`](Self::single) or [`single_mut`](Self::single_mut)
/// methods instead. Keep in mind though that they will return a [`QuerySingleError`] if the
/// number of query results differ from being exactly one. If that's the case, use `iter.next()`
/// (or `iter_mut.next()`) to only get the first query result.
pub struct Query<'world, 'state, Q: WorldQuery, F: WorldQuery = ()> {
    pub(crate) world: &'world World,
    pub(crate) state: &'state QueryState<Q, F>,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> Query<'w, 's, Q, F> {
    /// Creates a new query.
    ///
    /// # Safety
    ///
    /// This will create a query that could violate memory safety rules. Make sure that this is only
    /// called in ways that ensure the queries have unique mutable access.
    #[inline]
    pub(crate) unsafe fn new(
        world: &'w World,
        state: &'s QueryState<Q, F>,
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
    /// This can only return immutable data (mutable data will be cast to an immutable form).
    /// See [`Self::iter_mut`] for queries that contain at least one mutable component.
    ///
    /// # Example
    ///
    /// Here, the `report_names_system` iterates over the `Player` component of every entity
    /// that contains it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player { name: String }
    /// #
    /// fn report_names_system(query: Query<&Player>) {
    ///     for player in &query {
    ///         println!("Say hello to {}!", player.name);
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(report_names_system);
    /// ```
    #[inline]
    pub fn iter(&self) -> QueryIter<'_, 's, Q::ReadOnly, F::ReadOnly> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.as_readonly().iter_unchecked_manual(
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
    /// Here, the `gravity_system` iterates over the `Velocity` component of every entity in
    /// the world that contains it in order to update it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32, z: f32 }
    /// fn gravity_system(mut query: Query<&mut Velocity>) {
    ///     const DELTA: f32 = 1.0 / 60.0;
    ///     for mut velocity in &mut query {
    ///         velocity.y -= 9.8 * DELTA;
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(gravity_system);
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> QueryIter<'_, 's, Q, F> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .iter_unchecked_manual(self.world, self.last_change_tick, self.change_tick)
        }
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results without repetition.
    /// This can only return immutable data
    ///
    ///  For permutations of size `K` of query returning `N` results, you will get:
    /// - if `K == N`: one permutation of all query results
    /// - if `K < N`: all possible `K`-sized combinations of query results, without repetition
    /// - if `K > N`: empty set (no `K`-sized combinations exist)
    #[inline]
    pub fn iter_combinations<const K: usize>(
        &self,
    ) -> QueryCombinationIter<'_, '_, Q::ReadOnly, F::ReadOnly, K> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.as_readonly().iter_combinations_unchecked_manual(
                self.world,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Iterates over all possible combinations of `K` query results without repetition.
    ///
    /// The returned value is not an `Iterator`, because that would lead to aliasing of mutable references.
    /// In order to iterate it, use `fetch_next` method with `while let Some(..)` loop pattern.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// # struct A;
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
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.iter_combinations_unchecked_manual(
                self.world,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns an [`Iterator`] over the query results of a list of [`Entity`]'s.
    ///
    /// This can only return immutable data (mutable data will be cast to an immutable form).
    /// See [`Self::many_for_each_mut`] for queries that contain at least one mutable component.
    ///
    /// # Examples
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct Counter {
    ///     value: i32
    /// }
    ///
    /// #[derive(Component)]
    /// struct Friends {
    ///     list: Vec<Entity>,
    /// }
    ///
    /// fn system(
    ///     friends_query: Query<&Friends>,
    ///     counter_query: Query<&Counter>,
    /// ) {
    ///     for friends in &friends_query {
    ///         for counter in counter_query.iter_many(&friends.list) {
    ///             println!("Friend's counter: {:?}", counter.value);
    ///         }
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn iter_many<EntityList: IntoIterator>(
        &self,
        entities: EntityList,
    ) -> QueryManyIter<'_, '_, Q::ReadOnly, F::ReadOnly, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.as_readonly().iter_many_unchecked_manual(
                entities,
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
    pub unsafe fn iter_unsafe(&'s self) -> QueryIter<'w, 's, Q, F> {
        // SEMI-SAFETY: system runs without conflicts with other systems.
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
        // SEMI-SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state.iter_combinations_unchecked_manual(
            self.world,
            self.last_change_tick,
            self.change_tick,
        )
    }

    /// Returns an [`Iterator`] over the query results of a list of [`Entity`]'s.
    ///
    /// If you want safe mutable access to query results of a list of [`Entity`]'s. See [`Self::many_for_each_mut`].
    ///
    /// # Safety
    /// This allows aliased mutability and does not check for entity uniqueness.
    /// You must make sure this call does not result in multiple mutable references to the same component.
    /// Particular care must be taken when collecting the data (rather than iterating over it one item at a time) such as via `[Iterator::collect()]`.
    pub unsafe fn iter_many_unsafe<EntityList: IntoIterator>(
        &self,
        entities: EntityList,
    ) -> QueryManyIter<'_, '_, Q, F, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        self.state.iter_many_unchecked_manual(
            entities,
            self.world,
            self.last_change_tick,
            self.change_tick,
        )
    }

    /// Runs `f` on each query result. This is faster than the equivalent iter() method, but cannot
    /// be chained like a normal [`Iterator`].
    ///
    /// This can only pass in immutable data, see [`Self::for_each_mut`] for mutable access.
    ///
    /// # Example
    ///
    /// Here, the `report_names_system` iterates over the `Player` component of every entity
    /// that contains it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player { name: String }
    /// #
    /// fn report_names_system(query: Query<&Player>) {
    ///     query.for_each(|player| {
    ///         println!("Say hello to {}!", player.name);
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(report_names_system);
    /// ```
    #[inline]
    pub fn for_each<'this>(&'this self, f: impl FnMut(ROQueryItem<'this, Q>)) {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.as_readonly().for_each_unchecked_manual(
                self.world,
                f,
                self.last_change_tick,
                self.change_tick,
            );
        };
    }

    /// Runs `f` on each query result. This is faster than the equivalent iter() method, but cannot
    /// be chained like a normal [`Iterator`].
    ///
    /// # Example
    ///
    /// Here, the `gravity_system` iterates over the `Velocity` component of every entity in
    /// the world that contains it in order to update it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Velocity { x: f32, y: f32, z: f32 }
    /// fn gravity_system(mut query: Query<&mut Velocity>) {
    ///     const DELTA: f32 = 1.0 / 60.0;
    ///     query.for_each_mut(|mut velocity| {
    ///         velocity.y -= 9.8 * DELTA;
    ///     });
    /// }
    /// # bevy_ecs::system::assert_is_system(gravity_system);
    /// ```
    #[inline]
    pub fn for_each_mut<'a, FN: FnMut(QueryItem<'a, Q>)>(&'a mut self, f: FN) {
        // SAFETY: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.for_each_unchecked_manual(
                self.world,
                f,
                self.last_change_tick,
                self.change_tick,
            );
        };
    }

    /// Runs `f` on each query result in parallel using the [`World`]'s [`ComputeTaskPool`].
    ///
    /// This can only be called for immutable data, see [`Self::par_for_each_mut`] for
    /// mutable access.
    ///
    /// # Tasks and batch size
    ///
    /// The items in the query get sorted into batches.
    /// Internally, this function spawns a group of futures that each take on a `batch_size` sized section of the items (or less if the division is not perfect).
    /// Then, the tasks in the [`ComputeTaskPool`] work through these futures.
    ///
    /// You can use this value to tune between maximum multithreading ability (many small batches) and minimum parallelization overhead (few big batches).
    /// Rule of thumb: If the function body is (mostly) computationally expensive but there are not many items, a small batch size (=more batches) may help to even out the load.
    /// If the body is computationally cheap and you have many items, a large batch size (=fewer batches) avoids spawning additional futures that don't help to even out the load.
    ///
    /// # Arguments
    ///
    ///* `batch_size` - The number of batches to spawn
    ///* `f` - The function to run on each item in the query
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::prelude::ComputeTaskPool
    #[inline]
    pub fn par_for_each<'this>(
        &'this self,
        batch_size: usize,
        f: impl Fn(ROQueryItem<'this, Q>) + Send + Sync + Clone,
    ) {
        // SAFETY: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.as_readonly().par_for_each_unchecked_manual(
                self.world,
                batch_size,
                f,
                self.last_change_tick,
                self.change_tick,
            );
        };
    }

    /// Runs `f` on each query result in parallel using the [`World`]'s [`ComputeTaskPool`].
    /// See [`Self::par_for_each`] for more details.
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::prelude::ComputeTaskPool
    #[inline]
    pub fn par_for_each_mut<'a, FN: Fn(QueryItem<'a, Q>) + Send + Sync + Clone>(
        &'a mut self,
        batch_size: usize,
        f: FN,
    ) {
        // SAFETY: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.par_for_each_unchecked_manual(
                self.world,
                batch_size,
                f,
                self.last_change_tick,
                self.change_tick,
            );
        };
    }

    /// Calls a closure on each result of [`Query`] where the entities match.
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct Counter {
    ///     value: i32
    /// }
    ///
    /// #[derive(Component)]
    /// struct Friends {
    ///     list: Vec<Entity>,
    /// }
    ///
    /// fn system(
    ///     friends_query: Query<&Friends>,
    ///     mut counter_query: Query<&mut Counter>,
    /// ) {
    ///     for friends in &friends_query {
    ///         counter_query.many_for_each_mut(&friends.list, |mut counter| {
    ///             println!("Friend's counter: {:?}", counter.value);
    ///             counter.value += 1;
    ///         });
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn many_for_each_mut<EntityList: IntoIterator>(
        &mut self,
        entities: EntityList,
        f: impl FnMut(QueryItem<'_, Q>),
    ) where
        EntityList::Item: Borrow<Entity>,
    {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.many_for_each_unchecked_manual(
                self.world,
                entities,
                f,
                self.last_change_tick,
                self.change_tick,
            );
        };
    }

    /// Returns the query result for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// This can only return immutable data (mutable data will be cast to an immutable form).
    /// See [`get_mut`](Self::get_mut) for queries that contain at least one mutable component.
    ///
    /// # Example
    ///
    /// Here, `get` is used to retrieve the exact query result of the entity specified by the
    /// `SelectedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct SelectedCharacter { entity: Entity }
    /// # #[derive(Component)]
    /// # struct Character { name: String }
    /// #
    /// fn print_selected_character_name_system(
    ///        query: Query<&Character>,
    ///        selection: Res<SelectedCharacter>
    /// )
    /// {
    ///     if let Ok(selected_character) = query.get(selection.entity) {
    ///         println!("{}", selected_character.name);
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(print_selected_character_name_system);
    /// ```
    #[inline]
    pub fn get(&self, entity: Entity) -> Result<ROQueryItem<'_, Q>, QueryEntityError> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.as_readonly().get_unchecked_manual(
                self.world,
                entity,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns the read-only query results for the given array of [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// Note that the unlike [`Query::get_many_mut`], the entities passed in do not need to be unique.
    ///
    /// See [`Query::many`] for the infallible equivalent.
    #[inline]
    pub fn get_many<const N: usize>(
        &self,
        entities: [Entity; N],
    ) -> Result<[ROQueryItem<'_, Q>; N], QueryEntityError> {
        // SAFETY: it is the scheduler's responsibility to ensure that `Query` is never handed out on the wrong `World`.
        unsafe {
            self.state.get_many_read_only_manual(
                self.world,
                entities,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns the read-only query items for the provided array of [`Entity`]
    ///
    /// See [`Query::get_many`] for the [`Result`]-returning equivalent.
    ///
    /// # Examples
    /// ```rust, no_run
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Targets([Entity; 3]);
    ///
    /// #[derive(Component)]
    /// struct Position{
    ///     x: i8,
    ///     y: i8
    /// };
    ///
    /// impl Position {
    ///     fn distance(&self, other: &Position) -> i8 {
    ///         // Manhattan distance is way easier to compute!
    ///         (self.x - other.x).abs() + (self.y - other.y).abs()
    ///     }
    /// }
    ///
    /// fn check_all_targets_in_range(targeting_query: Query<(Entity, &Targets, &Position)>, targets_query: Query<&Position>){
    ///     for (targeting_entity, targets, origin) in &targeting_query {
    ///         // We can use "destructuring" to unpack the results nicely
    ///         let [target_1, target_2, target_3] = targets_query.many(targets.0);
    ///
    ///         assert!(target_1.distance(origin) <= 5);
    ///         assert!(target_2.distance(origin) <= 5);
    ///         assert!(target_3.distance(origin) <= 5);
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn many<const N: usize>(&self, entities: [Entity; N]) -> [ROQueryItem<'_, Q>; N] {
        self.get_many(entities).unwrap()
    }

    /// Returns the query result for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// # Example
    ///
    /// Here, `get_mut` is used to retrieve the exact query result of the entity specified by the
    /// `PoisonedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct PoisonedCharacter { character_id: Entity }
    /// # #[derive(Component)]
    /// # struct Health(u32);
    /// #
    /// fn poison_system(mut query: Query<&mut Health>, poisoned: Res<PoisonedCharacter>) {
    ///     if let Ok(mut health) = query.get_mut(poisoned.character_id) {
    ///         health.0 -= 1;
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(poison_system);
    /// ```
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Result<QueryItem<'_, Q>, QueryEntityError> {
        // SAFETY: system runs without conflicts with other systems.
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

    /// Returns the query results for the given array of [`Entity`].
    ///
    /// In case of a nonexisting entity, duplicate entities or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// See [`Query::many_mut`] for the infallible equivalent.
    #[inline]
    pub fn get_many_mut<const N: usize>(
        &mut self,
        entities: [Entity; N],
    ) -> Result<[QueryItem<'_, Q>; N], QueryEntityError> {
        // SAFETY: scheduler ensures safe Query world access
        unsafe {
            self.state.get_many_unchecked_manual(
                self.world,
                entities,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns the query items for the provided array of [`Entity`]
    ///
    /// See [`Query::get_many_mut`] for the [`Result`]-returning equivalent.
    ///
    /// # Examples
    ///
    /// ```rust, no_run
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct Spring{
    ///     connected_entities: [Entity; 2],
    ///     strength: f32,
    /// }
    ///
    /// #[derive(Component)]
    /// struct Position {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// #[derive(Component)]
    /// struct Force {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// fn spring_forces(spring_query: Query<&Spring>, mut mass_query: Query<(&Position, &mut Force)>){
    ///     for spring in &spring_query {
    ///          // We can use "destructuring" to unpack our query items nicely
    ///          let [(position_1, mut force_1), (position_2, mut force_2)] = mass_query.many_mut(spring.connected_entities);
    ///
    ///          force_1.x += spring.strength * (position_1.x - position_2.x);
    ///          force_1.y += spring.strength * (position_1.y - position_2.y);
    ///
    ///          // Silence borrow-checker: I have split your mutable borrow!
    ///          force_2.x += spring.strength * (position_2.x - position_1.x);
    ///          force_2.y += spring.strength * (position_2.y - position_1.y);
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn many_mut<const N: usize>(&mut self, entities: [Entity; N]) -> [QueryItem<'_, Q>; N] {
        self.get_many_mut(entities).unwrap()
    }

    /// Returns the query result for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees. You must make sure
    /// this call does not result in multiple mutable references to the same component
    #[inline]
    pub unsafe fn get_unchecked(
        &'s self,
        entity: Entity,
    ) -> Result<QueryItem<'w, Q>, QueryEntityError> {
        // SEMI-SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state
            .get_unchecked_manual(self.world, entity, self.last_change_tick, self.change_tick)
    }

    /// Returns a reference to the [`Entity`]'s [`Component`] of the given type.
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// # Example
    ///
    /// Here, `get_component` is used to retrieve the `Character` component of the entity
    /// specified by the `SelectedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct SelectedCharacter { entity: Entity }
    /// # #[derive(Component)]
    /// # struct Character { name: String }
    /// #
    /// fn print_selected_character_name_system(
    ///        query: Query<&Character>,
    ///        selection: Res<SelectedCharacter>
    /// )
    /// {
    ///     if let Ok(selected_character) = query.get_component::<Character>(selection.entity) {
    ///         println!("{}", selected_character.name);
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(print_selected_character_name_system);
    /// ```
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

    /// Returns a mutable reference to the [`Entity`]'s [`Component`] of the given type.
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// # Example
    ///
    /// Here, `get_component_mut` is used to retrieve the `Health` component of the entity
    /// specified by the `PoisonedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct PoisonedCharacter { character_id: Entity }
    /// # #[derive(Component)]
    /// # struct Health(u32);
    /// #
    /// fn poison_system(mut query: Query<&mut Health>, poisoned: Res<PoisonedCharacter>) {
    ///     if let Ok(mut health) = query.get_component_mut::<Health>(poisoned.character_id) {
    ///         health.0 -= 1;
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(poison_system);
    /// ```
    #[inline]
    pub fn get_component_mut<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        // SAFETY: unique access to query (preventing aliased access)
        unsafe { self.get_component_unchecked_mut(entity) }
    }

    /// Returns a mutable reference to the [`Entity`]'s [`Component`] of the given type.
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
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

    /// Returns a single immutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// This can only return immutable data. Use [`single_mut`](Self::single_mut) for
    /// queries that contain at least one mutable component.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Player;
    /// # #[derive(Component)]
    /// # struct Position(f32, f32);
    /// fn player_system(query: Query<&Position, With<Player>>) {
    ///     let player_position = query.single();
    ///     // do something with player_position
    /// }
    /// # bevy_ecs::system::assert_is_system(player_system);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number of query results is not exactly one. Use
    /// [`get_single`](Self::get_single) to return a `Result` instead of panicking.
    #[track_caller]
    pub fn single(&self) -> ROQueryItem<'_, Q> {
        self.get_single().unwrap()
    }

    /// Returns a single immutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// This can only return immutable data. Use [`get_single_mut`](Self::get_single_mut)
    /// for queries that contain at least one mutable component.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::query::QuerySingleError;
    /// # #[derive(Component)]
    /// # struct PlayerScore(i32);
    /// fn player_scoring_system(query: Query<&PlayerScore>) {
    ///     match query.get_single() {
    ///         Ok(PlayerScore(score)) => {
    ///             println!("Score: {}", score);
    ///         }
    ///         Err(QuerySingleError::NoEntities(_)) => {
    ///             println!("Error: There is no player!");
    ///         }
    ///         Err(QuerySingleError::MultipleEntities(_)) => {
    ///             println!("Error: There is more than one player!");
    ///         }
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(player_scoring_system);
    /// ```
    #[inline]
    pub fn get_single(&self) -> Result<ROQueryItem<'_, Q>, QuerySingleError> {
        // SAFETY:
        // the query ensures that the components it accesses are not mutably accessible somewhere else
        // and the query is read only.
        unsafe {
            self.state.as_readonly().get_single_unchecked_manual(
                self.world,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns a single mutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player;
    /// # #[derive(Component)]
    /// # struct Health(u32);
    /// #
    /// fn regenerate_player_health_system(mut query: Query<&mut Health, With<Player>>) {
    ///     let mut health = query.single_mut();
    ///     health.0 += 1;
    /// }
    /// # bevy_ecs::system::assert_is_system(regenerate_player_health_system);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number of query results is not exactly one. Use
    /// [`get_single_mut`](Self::get_single_mut) to return a `Result` instead of panicking.
    #[track_caller]
    pub fn single_mut(&mut self) -> QueryItem<'_, Q> {
        self.get_single_mut().unwrap()
    }

    /// Returns a single mutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player;
    /// # #[derive(Component)]
    /// # struct Health(u32);
    /// #
    /// fn regenerate_player_health_system(mut query: Query<&mut Health, With<Player>>) {
    ///     let mut health = query.get_single_mut().expect("Error: Could not find a single player.");
    ///     health.0 += 1;
    /// }
    /// # bevy_ecs::system::assert_is_system(regenerate_player_health_system);
    /// ```
    #[inline]
    pub fn get_single_mut(&mut self) -> Result<QueryItem<'_, Q>, QuerySingleError> {
        // SAFETY:
        // the query ensures mutable access to the components it accesses, and the query
        // is uniquely borrowed
        unsafe {
            self.state.get_single_unchecked_manual(
                self.world,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns `true` if there are no query results.
    ///
    /// # Example
    ///
    /// Here, the score is increased only if an entity with a `Player` component is present
    /// in the world:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player;
    /// # #[derive(Component)]
    /// # struct Score(u32);
    /// fn update_score_system(query: Query<(), With<Player>>, mut score: ResMut<Score>) {
    ///     if !query.is_empty() {
    ///         score.0 += 1;
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(update_score_system);
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.state
            .is_empty(self.world, self.last_change_tick, self.change_tick)
    }

    /// Returns `true` if the given [`Entity`] matches the query.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct InRange;
    /// #
    /// # struct Target {
    /// #     entity: Entity,
    /// # }
    /// #
    /// fn targeting_system(in_range_query: Query<&InRange>, target: Res<Target>) {
    ///     if in_range_query.contains(target.entity) {
    ///         println!("Bam!")
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(targeting_system);
    /// ```
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        // SAFETY: NopFetch does not access any members while &self ensures no one has exclusive access
        unsafe {
            self.state
                .as_nop()
                .get_unchecked_manual(self.world, entity, self.last_change_tick, self.change_tick)
                .is_ok()
        }
    }
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> IntoIterator for &'w Query<'_, 's, Q, F> {
    type Item = ROQueryItem<'w, Q>;
    type IntoIter = QueryIter<'w, 's, Q::ReadOnly, F::ReadOnly>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> IntoIterator for &'w mut Query<'_, 's, Q, F> {
    type Item = QueryItem<'w, Q>;
    type IntoIter = QueryIter<'w, 's, Q, F>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// An error that occurs when retrieving a specific [`Entity`]'s component from a [`Query`]
#[derive(Debug)]
pub enum QueryComponentError {
    MissingReadAccess,
    MissingWriteAccess,
    MissingComponent,
    NoSuchEntity,
}

impl std::error::Error for QueryComponentError {}

impl std::fmt::Display for QueryComponentError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QueryComponentError::MissingReadAccess => {
                write!(
                    f,
                    "This query does not have read access to the requested component."
                )
            }
            QueryComponentError::MissingWriteAccess => {
                write!(
                    f,
                    "This query does not have write access to the requested component."
                )
            }
            QueryComponentError::MissingComponent => {
                write!(f, "The given entity does not have the requested component.")
            }
            QueryComponentError::NoSuchEntity => {
                write!(f, "The requested entity does not exist.")
            }
        }
    }
}

impl<'w, 's, Q: ReadOnlyWorldQuery, F: WorldQuery> Query<'w, 's, Q, F> {
    /// Returns the query result for the given [`Entity`], with the actual "inner" world lifetime.
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// This can only return immutable data (mutable data will be cast to an immutable form).
    /// See [`get_mut`](Self::get_mut) for queries that contain at least one mutable component.
    ///
    /// # Example
    ///
    /// Here, `get` is used to retrieve the exact query result of the entity specified by the
    /// `SelectedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct SelectedCharacter { entity: Entity }
    /// # #[derive(Component)]
    /// # struct Character { name: String }
    /// #
    /// fn print_selected_character_name_system(
    ///        query: Query<&Character>,
    ///        selection: Res<SelectedCharacter>
    /// )
    /// {
    ///     if let Ok(selected_character) = query.get(selection.entity) {
    ///         println!("{}", selected_character.name);
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(print_selected_character_name_system);
    /// ```
    #[inline]
    pub fn get_inner(&'s self, entity: Entity) -> Result<ROQueryItem<'w, Q>, QueryEntityError> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.as_readonly().get_unchecked_manual(
                self.world,
                entity,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns an [`Iterator`] over the query results, with the actual "inner" world lifetime.
    ///
    /// This can only return immutable data (mutable data will be cast to an immutable form).
    /// See [`Self::iter_mut`] for queries that contain at least one mutable component.
    ///
    /// # Example
    ///
    /// Here, the `report_names_system` iterates over the `Player` component of every entity
    /// that contains it:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player { name: String }
    /// #
    /// fn report_names_system(query: Query<&Player>) {
    ///     for player in &query {
    ///         println!("Say hello to {}!", player.name);
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(report_names_system);
    /// ```
    #[inline]
    pub fn iter_inner(&'s self) -> QueryIter<'w, 's, Q::ReadOnly, F::ReadOnly> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.as_readonly().iter_unchecked_manual(
                self.world,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }
}
