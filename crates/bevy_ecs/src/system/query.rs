use crate::{
    component::Component,
    entity::Entity,
    query::{
        Fetch, FilterFetch, QueryCombinationIter, QueryEntityError, QueryIter, QueryState,
        WorldQuery,
    },
    world::{Mut, World},
};
use bevy_tasks::TaskPool;
use std::{any::TypeId, fmt::Debug};
use thiserror::Error;

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
/// # system.system();
/// ```
///
/// Two systems cannot be executed in parallel if both access a certain component and
/// at least one of the accesses is mutable, unless the schedule can verify that no entity
/// could be found in both queries, as otherwise Rusts mutability Rules would be broken.
///
/// Similarly, a system cannot contain two queries that would break Rust's mutability Rules.
/// If you need such Queries, you can use Filters to make the Queries disjoint or use a
/// [`QuerySet`](super::QuerySet).
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
/// # system.system();
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
/// # system.system();
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
/// # system.system();
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
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
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
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
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
pub struct Query<'world, 'state, Q: WorldQuery, F: WorldQuery = ()>
where
    F::Fetch: FilterFetch,
{
    pub(crate) world: &'world World,
    pub(crate) state: &'state QueryState<Q, F>,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> Query<'w, 's, Q, F>
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
    ///     for player in query.iter() {
    ///         println!("Say hello to {}!", player.name);
    ///     }
    /// }
    /// # report_names_system.system();
    /// ```
    #[inline]
    pub fn iter(&'s self) -> QueryIter<'w, 's, Q, Q::ReadOnlyFetch, F> {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .iter_unchecked_manual(self.world, self.last_change_tick, self.change_tick)
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
    ///     for mut velocity in query.iter_mut() {
    ///         velocity.y -= 9.8 * DELTA;
    ///     }
    /// }
    /// # gravity_system.system();
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> QueryIter<'_, '_, Q, Q::Fetch, F> {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .iter_unchecked_manual(self.world, self.last_change_tick, self.change_tick)
        }
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results without repetition.
    /// This can only return immutable data
    ///
    ///  For permutations of size K of query returning N results, you will get:
    /// - if K == N: one permutation of all query results
    /// - if K < N: all possible K-sized combinations of query results, without repetition
    /// - if K > N: empty set (no K-sized combinations exist)
    #[inline]
    pub fn iter_combinations<const K: usize>(
        &self,
    ) -> QueryCombinationIter<'_, '_, Q, Q::ReadOnlyFetch, F, K> {
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
    ) -> QueryCombinationIter<'_, '_, Q, Q::Fetch, F, K> {
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
    pub unsafe fn iter_unsafe(&'s self) -> QueryIter<'w, 's, Q, Q::Fetch, F> {
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
    ) -> QueryCombinationIter<'_, '_, Q, Q::Fetch, F, K> {
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
    /// # report_names_system.system();
    /// ```
    #[inline]
    pub fn for_each<FN: FnMut(<Q::ReadOnlyFetch as Fetch<'w, 's>>::Item)>(&'s self, f: FN) {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .for_each_unchecked_manual::<Q::ReadOnlyFetch, FN>(
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
    /// # gravity_system.system();
    /// ```
    #[inline]
    pub fn for_each_mut<'a, FN: FnMut(<Q::Fetch as Fetch<'a, 'a>>::Item)>(&'a mut self, f: FN) {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.for_each_unchecked_manual::<Q::Fetch, FN>(
                self.world,
                f,
                self.last_change_tick,
                self.change_tick,
            )
        };
    }

    /// Runs `f` on each query result in parallel using the given task pool.
    ///
    /// This can only be called for immutable data, see [`Self::par_for_each_mut`] for
    /// mutable access.
    #[inline]
    pub fn par_for_each<FN: Fn(<Q::ReadOnlyFetch as Fetch<'w, 's>>::Item) + Send + Sync + Clone>(
        &'s self,
        task_pool: &TaskPool,
        batch_size: usize,
        f: FN,
    ) {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state
                .par_for_each_unchecked_manual::<Q::ReadOnlyFetch, FN>(
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
    pub fn par_for_each_mut<'a, FN: Fn(<Q::Fetch as Fetch<'a, 'a>>::Item) + Send + Sync + Clone>(
        &'a mut self,
        task_pool: &TaskPool,
        batch_size: usize,
        f: FN,
    ) {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.par_for_each_unchecked_manual::<Q::Fetch, FN>(
                self.world,
                task_pool,
                batch_size,
                f,
                self.last_change_tick,
                self.change_tick,
            )
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
    /// # print_selected_character_name_system.system();
    /// ```
    #[inline]
    pub fn get(
        &'s self,
        entity: Entity,
    ) -> Result<<Q::ReadOnlyFetch as Fetch<'w, 's>>::Item, QueryEntityError> {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.get_unchecked_manual::<Q::ReadOnlyFetch>(
                self.world,
                entity,
                self.last_change_tick,
                self.change_tick,
            )
        }
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
    /// # poison_system.system();
    /// ```
    #[inline]
    pub fn get_mut(
        &mut self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, QueryEntityError> {
        // SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.get_unchecked_manual::<Q::Fetch>(
                self.world,
                entity,
                self.last_change_tick,
                self.change_tick,
            )
        }
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
    ) -> Result<<Q::Fetch as Fetch<'w, 's>>::Item, QueryEntityError> {
        // SEMI-SAFE: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state.get_unchecked_manual::<Q::Fetch>(
            self.world,
            entity,
            self.last_change_tick,
            self.change_tick,
        )
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
    /// # print_selected_character_name_system.system();
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
    /// # poison_system.system();
    /// ```
    #[inline]
    pub fn get_component_mut<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        // SAFE: unique access to query (preventing aliased access)
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
    /// # player_system.system();
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number of query results is not exactly one. Use
    /// [`get_single`](Self::get_single) to return a `Result` instead of panicking.
    #[track_caller]
    pub fn single(&'s self) -> <Q::ReadOnlyFetch as Fetch<'w, 's>>::Item {
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
    /// # use bevy_ecs::system::QuerySingleError;
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
    /// # player_scoring_system.system();
    /// ```
    pub fn get_single(
        &'s self,
    ) -> Result<<Q::ReadOnlyFetch as Fetch<'w, 's>>::Item, QuerySingleError> {
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
    /// # regenerate_player_health_system.system();
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the number of query results is not exactly one. Use
    /// [`get_single_mut`](Self::get_single_mut) to return a `Result` instead of panicking.
    #[track_caller]
    pub fn single_mut(&mut self) -> <Q::Fetch as Fetch<'_, '_>>::Item {
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
    /// # regenerate_player_health_system.system();
    /// ```
    pub fn get_single_mut(
        &mut self,
    ) -> Result<<Q::Fetch as Fetch<'_, '_>>::Item, QuerySingleError> {
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
    /// # update_score_system.system();
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.state
            .is_empty(self.world, self.last_change_tick, self.change_tick)
    }
}

/// An error that occurs when retrieving a specific [`Entity`]'s component from a [`Query`]
#[derive(Error, Debug)]
pub enum QueryComponentError {
    #[error("This query does not have read access to the requested component.")]
    MissingReadAccess,
    #[error("This query does not have write access to the requested component.")]
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
