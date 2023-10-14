use crate::{
    component::{Component, Tick},
    entity::Entity,
    query::{
        BatchingStrategy, QueryCombinationIter, QueryComponentError, QueryEntityError, QueryIter,
        QueryManyIter, QueryParIter, QuerySingleError, QueryState, ROQueryItem, ReadOnlyWorldQuery,
        WorldQuery,
    },
    world::{unsafe_world_cell::UnsafeWorldCell, Mut},
};
use std::{any::TypeId, borrow::Borrow};

/// [System parameter] that provides selective access to the [`Component`] data stored in a [`World`].
///
/// Enables access to [entity identifiers] and [components] from a system, without the need to directly access the world.
/// Its iterators and getter methods return *query items*.
/// Each query item is a type containing data relative to an entity.
///
/// `Query` is a generic data structure that accepts two type parameters, both of which must implement the [`WorldQuery`] trait:
///
/// - **`Q` (query fetch).**
///   The type of data contained in the query item.
///   Only entities that match the requested data will generate an item.
/// - **`F` (query filter).**
///   A set of conditions that determines whether query items should be kept or discarded.
///   This type parameter is optional.
///
/// [`World`]: crate::world::World
///
/// # System parameter declaration
///
/// A query should always be declared as a system parameter.
/// This section shows the most common idioms involving the declaration of `Query`, emerging by combining [`WorldQuery`] implementors.
///
/// ## Component access
///
/// A query defined with a reference to a component as the query fetch type parameter can be used to generate items that refer to the data of said component.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # fn immutable_ref(
/// // A component can be accessed by shared reference...
/// query: Query<&ComponentA>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(immutable_ref);
///
/// # fn mutable_ref(
/// // ... or by mutable reference.
/// query: Query<&mut ComponentA>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(mutable_ref);
/// ```
///
/// ## Query filtering
///
/// Setting the query filter type parameter will ensure that each query item satisfies the given condition.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # fn system(
/// // Just `ComponentA` data will be accessed, but only for entities that also contain
/// // `ComponentB`.
/// query: Query<&ComponentA, With<ComponentB>>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// ## `WorldQuery` tuples
///
/// Using tuples, each `Query` type parameter can contain multiple elements.
///
/// In the following example, two components are accessed simultaneously, and the query items are filtered on two conditions.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # #[derive(Component)]
/// # struct ComponentC;
/// # #[derive(Component)]
/// # struct ComponentD;
/// # fn immutable_ref(
/// query: Query<(&ComponentA, &ComponentB), (With<ComponentC>, Without<ComponentD>)>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(immutable_ref);
/// ```
///
/// ## Entity identifier access
///
/// The identifier of an entity can be made available inside the query item by including [`Entity`] in the query fetch type parameter.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # fn system(
/// query: Query<(Entity, &ComponentA)>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// ## Optional component access
///
/// A component can be made optional in a query by wrapping it into an [`Option`].
/// In this way, a query item can still be generated even if the queried entity does not contain the wrapped component.
/// In this case, its corresponding value will be `None`.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # fn system(
/// // Generates items for entities that contain `ComponentA`, and optionally `ComponentB`.
/// query: Query<(&ComponentA, Option<&ComponentB>)>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// See the documentation for [`AnyOf`] to idiomatically declare many optional components.
///
/// See the [performance] section to learn more about the impact of optional components.
///
/// ## Disjoint queries
///
/// A system cannot contain two queries that break Rust's mutability rules.
/// In this case, the [`Without`] filter can be used to disjoint them.
///
/// In the following example, two queries mutably access the same component.
/// Executing this system will panic, since an entity could potentially match the two queries at the same time by having both `Player` and `Enemy` components.
/// This would violate mutability rules.
///
/// ```should_panic
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct Health;
/// # #[derive(Component)]
/// # struct Player;
/// # #[derive(Component)]
/// # struct Enemy;
/// #
/// fn randomize_health(
///     player_query: Query<&mut Health, With<Player>>,
///     enemy_query: Query<&mut Health, With<Enemy>>,
/// )
/// # {}
/// # let mut randomize_health_system = bevy_ecs::system::IntoSystem::into_system(randomize_health);
/// # let mut world = World::new();
/// # randomize_health_system.initialize(&mut world);
/// # randomize_health_system.run((), &mut world);
/// ```
///
/// Adding a `Without` filter will disjoint the queries.
/// In this way, any entity that has both `Player` and `Enemy` components is excluded from both queries.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct Health;
/// # #[derive(Component)]
/// # struct Player;
/// # #[derive(Component)]
/// # struct Enemy;
/// #
/// fn randomize_health(
///     player_query: Query<&mut Health, (With<Player>, Without<Enemy>)>,
///     enemy_query: Query<&mut Health, (With<Enemy>, Without<Player>)>,
/// )
/// # {}
/// # let mut randomize_health_system = bevy_ecs::system::IntoSystem::into_system(randomize_health);
/// # let mut world = World::new();
/// # randomize_health_system.initialize(&mut world);
/// # randomize_health_system.run((), &mut world);
/// ```
///
/// An alternative to this idiom is to wrap the conflicting queries into a [`ParamSet`](super::ParamSet).
///
/// ## Whole Entity Access
///
/// [`EntityRef`]s can be fetched from a query. This will give read-only access to any component on the entity,
/// and can be use to dynamically fetch any component without baking it into the query type. Due to this global
/// access to the entity, this will block any other system from parallelizing with it. As such these queries
/// should be sparingly used.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # fn system(
/// query: Query<(EntityRef, &ComponentA)>
/// # ) {}
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// As `EntityRef` can read any component on an entity, a query using it will conflict with *any* mutable
/// access. It is strongly advised to couple `EntityRef` queries with the use of either `With`/`Without`
/// filters or `ParamSets`. This also limits the scope of the query, which will improve iteration performance
/// and also allows it to parallelize with other non-conflicting systems.
///
/// ```should_panic
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # fn system(
/// // This will panic!
/// query: Query<(EntityRef, &mut ComponentA)>
/// # ) {}
/// # bevy_ecs::system::assert_system_does_not_conflict(system);
/// ```
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # fn system(
/// // This will not panic.
/// query_a: Query<EntityRef, With<ComponentA>>,
/// query_b: Query<&mut ComponentB, Without<ComponentA>>,
/// # ) {}
/// # bevy_ecs::system::assert_system_does_not_conflict(system);
/// ```
///
/// # Accessing query items
///
/// The following table summarizes the behavior of the safe methods that can be used to get query items.
///
/// |Query methods|Effect|
/// |:---:|---|
/// |[`iter`]\[[`_mut`][`iter_mut`]]|Returns an iterator over all query items.|
/// |[`for_each`]\[[`_mut`][`for_each_mut`]],<br>[`par_iter`]\[[`_mut`][`par_iter_mut`]]|Runs a specified function for each query item.|
/// |[`iter_many`]\[[`_mut`][`iter_many_mut`]]|Iterates or runs a specified function over query items generated by a list of entities.|
/// |[`iter_combinations`]\[[`_mut`][`iter_combinations_mut`]]|Returns an iterator over all combinations of a specified number of query items.|
/// |[`get`]\[[`_mut`][`get_mut`]]|Returns the query item for the specified entity.|
/// |[`many`]\[[`_mut`][`many_mut`]],<br>[`get_many`]\[[`_mut`][`get_many_mut`]]|Returns the query items for the specified entities.|
/// |[`single`]\[[`_mut`][`single_mut`]],<br>[`get_single`]\[[`_mut`][`get_single_mut`]]|Returns the query item while verifying that there aren't others.|
///
/// There are two methods for each type of query operation: immutable and mutable (ending with `_mut`).
/// When using immutable methods, the query items returned are of type [`ROQueryItem`], a read-only version of the query item.
/// In this circumstance, every mutable reference in the query fetch type parameter is substituted by a shared reference.
///
/// # Performance
///
/// Creating a `Query` is a low-cost constant operation.
/// Iterating it, on the other hand, fetches data from the world and generates items, which can have a significant computational cost.
///
/// [`Table`] component storage type is much more optimized for query iteration than [`SparseSet`].
///
/// Two systems cannot be executed in parallel if both access the same component type where at least one of the accesses is mutable.
/// This happens unless the executor can verify that no entity could be found in both queries.
///
/// Optional components increase the number of entities a query has to match against.
/// This can hurt iteration performance, especially if the query solely consists of only optional components, since the query would iterate over each entity in the world.
///
/// The following table compares the computational complexity of the various methods and operations, where:
///
/// - **n** is the number of entities that match the query,
/// - **r** is the number of elements in a combination,
/// - **k** is the number of involved entities in the operation,
/// - **a** is the number of archetypes in the world,
/// - **C** is the [binomial coefficient], used to count combinations.
///   <sub>n</sub>C<sub>r</sub> is read as "*n* choose *r*" and is equivalent to the number of distinct unordered subsets of *r* elements that can be taken from a set of *n* elements.
///
/// |Query operation|Computational complexity|
/// |:---:|:---:|
/// |[`iter`]\[[`_mut`][`iter_mut`]]|O(n)|
/// |[`for_each`]\[[`_mut`][`for_each_mut`]],<br>[`par_iter`]\[[`_mut`][`par_iter_mut`]]|O(n)|
/// |[`iter_many`]\[[`_mut`][`iter_many_mut`]]|O(k)|
/// |[`iter_combinations`]\[[`_mut`][`iter_combinations_mut`]]|O(<sub>n</sub>C<sub>r</sub>)|
/// |[`get`]\[[`_mut`][`get_mut`]]|O(1)|
/// |([`get_`][`get_many`])[`many`]|O(k)|
/// |([`get_`][`get_many_mut`])[`many_mut`]|O(k<sup>2</sup>)|
/// |[`single`]\[[`_mut`][`single_mut`]],<br>[`get_single`]\[[`_mut`][`get_single_mut`]]|O(a)|
/// |Archetype based filtering ([`With`], [`Without`], [`Or`])|O(a)|
/// |Change detection filtering ([`Added`], [`Changed`])|O(a + n)|
///
/// `for_each` methods are seen to be generally faster than their `iter` version on worlds with high archetype fragmentation.
/// As iterators are in general more flexible and better integrated with the rest of the Rust ecosystem,
/// it is advised to use `iter` methods over `for_each`.
/// It is strongly advised to only use `for_each` if it tangibly improves performance:
/// be sure profile or benchmark both before and after the change.
///
/// [`Added`]: crate::query::Added
/// [`AnyOf`]: crate::query::AnyOf
/// [binomial coefficient]: https://en.wikipedia.org/wiki/Binomial_coefficient
/// [`Changed`]: crate::query::Changed
/// [components]: crate::component::Component
/// [entity identifiers]: crate::entity::Entity
/// [`EntityRef`]: crate::world::EntityRef
/// [`for_each`]: Self::for_each
/// [`for_each_mut`]: Self::for_each_mut
/// [`get`]: Self::get
/// [`get_many`]: Self::get_many
/// [`get_many_mut`]: Self::get_many_mut
/// [`get_mut`]: Self::get_mut
/// [`get_single`]: Self::get_single
/// [`get_single_mut`]: Self::get_single_mut
/// [`iter`]: Self::iter
/// [`iter_combinations`]: Self::iter_combinations
/// [`iter_combinations_mut`]: Self::iter_combinations_mut
/// [`iter_many`]: Self::iter_many
/// [`iter_many_mut`]: Self::iter_many_mut
/// [`iter_mut`]: Self::iter_mut
/// [`many`]: Self::many
/// [`many_mut`]: Self::many_mut
/// [`Or`]: crate::query::Or
/// [`par_iter`]: Self::par_iter
/// [`par_iter_mut`]: Self::par_iter_mut
/// [performance]: #performance
/// [`single`]: Self::single
/// [`single_mut`]: Self::single_mut
/// [`SparseSet`]: crate::storage::SparseSet
/// [System parameter]: crate::system::SystemParam
/// [`Table`]: crate::storage::Table
/// [`With`]: crate::query::With
/// [`Without`]: crate::query::Without
pub struct Query<'world, 'state, Q: WorldQuery, F: ReadOnlyWorldQuery = ()> {
    // SAFETY: Must have access to the components registered in `state`.
    world: UnsafeWorldCell<'world>,
    state: &'state QueryState<Q, F>,
    last_run: Tick,
    this_run: Tick,
    // SAFETY: This is used to ensure that `get_component_mut::<C>` properly fails when a Query writes C
    // and gets converted to a read-only query using `to_readonly`. Without checking this, `get_component_mut` relies on
    // QueryState's archetype_component_access, which will continue allowing write access to C after being cast to
    // the read-only variant. This whole situation is confusing and error prone. Ideally this is a temporary hack
    // until we sort out a cleaner alternative.
    force_read_only_component_access: bool,
}

impl<Q: WorldQuery, F: ReadOnlyWorldQuery> std::fmt::Debug for Query<'_, '_, Q, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Query")
            .field("matched_entities", &self.iter().count())
            .field("state", &self.state)
            .field("last_run", &self.last_run)
            .field("this_run", &self.this_run)
            .field("world", &self.world)
            .finish()
    }
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> Query<'w, 's, Q, F> {
    /// Creates a new query.
    ///
    /// # Panics
    ///
    /// This will panic if the world used to create `state` is not `world`.
    ///
    /// # Safety
    ///
    /// This will create a query that could violate memory safety rules. Make sure that this is only
    /// called in ways that ensure the queries have unique mutable access.
    #[inline]
    pub(crate) unsafe fn new(
        world: UnsafeWorldCell<'w>,
        state: &'s QueryState<Q, F>,
        last_run: Tick,
        this_run: Tick,
        force_read_only_component_access: bool,
    ) -> Self {
        state.validate_world(world.id());

        Self {
            force_read_only_component_access,
            world,
            state,
            last_run,
            this_run,
        }
    }

    /// Returns another `Query` from this that fetches the read-only version of the query items.
    ///
    /// For example, `Query<(&mut A, &B, &mut C), With<D>>` will become `Query<(&A, &B, &C), With<D>>`.
    /// This can be useful when working around the borrow checker,
    /// or reusing functionality between systems via functions that accept query types.
    pub fn to_readonly(&self) -> Query<'_, 's, Q::ReadOnly, F::ReadOnly> {
        let new_state = self.state.as_readonly();
        // SAFETY: This is memory safe because it turns the query immutable.
        unsafe {
            Query::new(
                self.world,
                new_state,
                self.last_run,
                self.this_run,
                // SAFETY: this must be set to true or `get_component_mut` will be unsound. See the comments
                // on this field for more details
                true,
            )
        }
    }

    /// Returns an [`Iterator`] over the read-only query items.
    ///
    /// # Example
    ///
    /// Here, the `report_names_system` iterates over the `Player` component of every entity that contains it:
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
    ///
    /// # See also
    ///
    /// - [`iter_mut`](Self::iter_mut) for mutable query items.
    /// - [`for_each`](Self::for_each) for the closure based alternative.
    #[inline]
    pub fn iter(&self) -> QueryIter<'_, 's, Q::ReadOnly, F::ReadOnly> {
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
    /// # Example
    ///
    /// Here, the `gravity_system` updates the `Velocity` component of every entity that contains it:
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
    ///
    /// # See also
    ///
    /// - [`iter`](Self::iter) for read-only query items.
    /// - [`for_each_mut`](Self::for_each_mut) for the closure based alternative.
    #[inline]
    pub fn iter_mut(&mut self) -> QueryIter<'_, 's, Q, F> {
        // SAFETY: `self.world` has permission to access the required components.
        unsafe {
            self.state
                .iter_unchecked_manual(self.world, self.last_run, self.this_run)
        }
    }

    /// Returns a [`QueryCombinationIter`] over all combinations of `K` read-only query items without repetition.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct ComponentA;
    /// #
    /// fn some_system(query: Query<&ComponentA>) {
    ///     for [a1, a2] in query.iter_combinations() {
    ///         // ...
    ///     }
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// - [`iter_combinations_mut`](Self::iter_combinations_mut) for mutable query item combinations.
    #[inline]
    pub fn iter_combinations<const K: usize>(
        &self,
    ) -> QueryCombinationIter<'_, 's, Q::ReadOnly, F::ReadOnly, K> {
        // SAFETY:
        // - `self.world` has permission to access the required components.
        // - The query is read-only, so it can be aliased even if it was originally mutable.
        unsafe {
            self.state.as_readonly().iter_combinations_unchecked_manual(
                self.world,
                self.last_run,
                self.this_run,
            )
        }
    }

    /// Returns a [`QueryCombinationIter`] over all combinations of `K` query items without repetition.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct ComponentA;
    /// fn some_system(mut query: Query<&mut ComponentA>) {
    ///     let mut combinations = query.iter_combinations_mut();
    ///     while let Some([mut a1, mut a2]) = combinations.fetch_next() {
    ///         // mutably access components data
    ///     }
    /// }
    /// ```
    ///
    /// # See also
    ///
    /// - [`iter_combinations`](Self::iter_combinations) for read-only query item combinations.
    #[inline]
    pub fn iter_combinations_mut<const K: usize>(
        &mut self,
    ) -> QueryCombinationIter<'_, 's, Q, F, K> {
        // SAFETY: `self.world` has permission to access the required components.
        unsafe {
            self.state
                .iter_combinations_unchecked_manual(self.world, self.last_run, self.this_run)
        }
    }

    /// Returns an [`Iterator`] over the read-only query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Component)]
    /// # struct Counter {
    /// #     value: i32
    /// # }
    /// #
    /// // A component containing an entity list.
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
    ///
    /// # See also
    ///
    /// - [`iter_many_mut`](Self::iter_many_mut) to get mutable query items.
    #[inline]
    pub fn iter_many<EntityList: IntoIterator>(
        &self,
        entities: EntityList,
    ) -> QueryManyIter<'_, 's, Q::ReadOnly, F::ReadOnly, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        // SAFETY:
        // - `self.world` has permission to access the required components.
        // - The query is read-only, so it can be aliased even if it was originally mutable.
        unsafe {
            self.state.as_readonly().iter_many_unchecked_manual(
                entities,
                self.world,
                self.last_run,
                self.this_run,
            )
        }
    }

    /// Returns an iterator over the query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    ///
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
    ///         let mut iter = counter_query.iter_many_mut(&friends.list);
    ///         while let Some(mut counter) = iter.fetch_next() {
    ///             println!("Friend's counter: {:?}", counter.value);
    ///             counter.value += 1;
    ///         }
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn iter_many_mut<EntityList: IntoIterator>(
        &mut self,
        entities: EntityList,
    ) -> QueryManyIter<'_, 's, Q, F, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        // SAFETY: `self.world` has permission to access the required components.
        unsafe {
            self.state.iter_many_unchecked_manual(
                entities,
                self.world,
                self.last_run,
                self.this_run,
            )
        }
    }

    /// Returns an [`Iterator`] over the query items.
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees.
    /// You must make sure this call does not result in multiple mutable references to the same component.
    ///
    /// # See also
    ///
    /// - [`iter`](Self::iter) and [`iter_mut`](Self::iter_mut) for the safe versions.
    #[inline]
    pub unsafe fn iter_unsafe(&self) -> QueryIter<'_, 's, Q, F> {
        // SAFETY:
        // - `self.world` has permission to access the required components.
        // - The caller ensures that this operation will not result in any aliased mutable accesses.
        self.state
            .iter_unchecked_manual(self.world, self.last_run, self.this_run)
    }

    /// Iterates over all possible combinations of `K` query items without repetition.
    ///
    /// # Safety
    ///
    /// This allows aliased mutability.
    /// You must make sure this call does not result in multiple mutable references to the same component.
    ///
    /// # See also
    ///
    /// - [`iter_combinations`](Self::iter_combinations) and [`iter_combinations_mut`](Self::iter_combinations_mut) for the safe versions.
    #[inline]
    pub unsafe fn iter_combinations_unsafe<const K: usize>(
        &self,
    ) -> QueryCombinationIter<'_, 's, Q, F, K> {
        // SAFETY:
        // - `self.world` has permission to access the required components.
        // - The caller ensures that this operation will not result in any aliased mutable accesses.
        self.state
            .iter_combinations_unchecked_manual(self.world, self.last_run, self.this_run)
    }

    /// Returns an [`Iterator`] over the query items generated from an [`Entity`] list.
    ///
    /// # Safety
    ///
    /// This allows aliased mutability and does not check for entity uniqueness.
    /// You must make sure this call does not result in multiple mutable references to the same component.
    /// Particular care must be taken when collecting the data (rather than iterating over it one item at a time) such as via [`Iterator::collect`].
    ///
    /// # See also
    ///
    /// - [`iter_many_mut`](Self::iter_many_mut) to safely access the query items.
    pub unsafe fn iter_many_unsafe<EntityList: IntoIterator>(
        &self,
        entities: EntityList,
    ) -> QueryManyIter<'_, 's, Q, F, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        // SAFETY:
        // - `self.world` has permission to access the required components.
        // - The caller ensures that this operation will not result in any aliased mutable accesses.
        self.state
            .iter_many_unchecked_manual(entities, self.world, self.last_run, self.this_run)
    }

    /// Runs `f` on each read-only query item.
    ///
    /// # Example
    ///
    /// Here, the `report_names_system` iterates over the `Player` component of every entity that contains it:
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
    ///
    /// # See also
    ///
    /// - [`for_each_mut`](Self::for_each_mut) to operate on mutable query items.
    /// - [`iter`](Self::iter) for the iterator based alternative.
    #[inline]
    pub fn for_each<'this>(&'this self, f: impl FnMut(ROQueryItem<'this, Q>)) {
        // SAFETY:
        // - `self.world` has permission to access the required components.
        // - The query is read-only, so it can be aliased even if it was originally mutable.
        unsafe {
            self.state.as_readonly().for_each_unchecked_manual(
                self.world,
                f,
                self.last_run,
                self.this_run,
            );
        };
    }

    /// Runs `f` on each query item.
    ///
    /// # Example
    ///
    /// Here, the `gravity_system` updates the `Velocity` component of every entity that contains it:
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
    ///
    /// # See also
    ///
    /// - [`for_each`](Self::for_each) to operate on read-only query items.
    /// - [`iter_mut`](Self::iter_mut) for the iterator based alternative.
    #[inline]
    pub fn for_each_mut<'a>(&'a mut self, f: impl FnMut(Q::Item<'a>)) {
        // SAFETY: `self.world` has permission to access the required components.
        unsafe {
            self.state
                .for_each_unchecked_manual(self.world, f, self.last_run, self.this_run);
        };
    }

    /// Returns a parallel iterator over the query results for the given [`World`].
    ///
    /// This can only be called for read-only queries, see [`par_iter_mut`] for write-queries.
    ///
    /// [`par_iter_mut`]: Self::par_iter_mut
    /// [`World`]: crate::world::World
    #[inline]
    pub fn par_iter(&self) -> QueryParIter<'_, '_, Q::ReadOnly, F::ReadOnly> {
        QueryParIter {
            world: self.world,
            state: self.state.as_readonly(),
            last_run: self.last_run,
            this_run: self.this_run,
            batching_strategy: BatchingStrategy::new(),
        }
    }

    /// Returns a parallel iterator over the query results for the given [`World`].
    ///
    /// This can only be called for mutable queries, see [`par_iter`] for read-only-queries.
    ///
    /// [`par_iter`]: Self::par_iter
    /// [`World`]: crate::world::World
    #[inline]
    pub fn par_iter_mut(&mut self) -> QueryParIter<'_, '_, Q, F> {
        QueryParIter {
            world: self.world,
            state: self.state,
            last_run: self.last_run,
            this_run: self.this_run,
            batching_strategy: BatchingStrategy::new(),
        }
    }

    /// Returns the read-only query item for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// # Example
    ///
    /// Here, `get` is used to retrieve the exact query item of the entity specified by the `SelectedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
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
    ///
    /// # See also
    ///
    /// - [`get_mut`](Self::get_mut) to get a mutable query item.
    #[inline]
    pub fn get(&self, entity: Entity) -> Result<ROQueryItem<'_, Q>, QueryEntityError> {
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

    /// Returns the read-only query items for the given array of [`Entity`].
    ///
    /// The returned query items are in the same order as the input.
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    /// The elements of the array do not need to be unique, unlike `get_many_mut`.
    ///
    /// # See also
    ///
    /// - [`get_many_mut`](Self::get_many_mut) to get mutable query items.
    /// - [`many`](Self::many) for the panicking version.
    #[inline]
    pub fn get_many<const N: usize>(
        &self,
        entities: [Entity; N],
    ) -> Result<[ROQueryItem<'_, Q>; N], QueryEntityError> {
        // SAFETY:
        // - `&self` ensures there is no mutable access to any components accessible to this query.
        // - `self.world` matches `self.state`.
        unsafe {
            self.state
                .get_many_read_only_manual(self.world, entities, self.last_run, self.this_run)
        }
    }

    /// Returns the read-only query items for the given array of [`Entity`].
    ///
    /// # Panics
    ///
    /// This method panics if there is a query mismatch or a non-existing entity.
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
    ///
    /// # See also
    ///
    /// - [`get_many`](Self::get_many) for the non-panicking version.
    #[inline]
    pub fn many<const N: usize>(&self, entities: [Entity; N]) -> [ROQueryItem<'_, Q>; N] {
        match self.get_many(entities) {
            Ok(items) => items,
            Err(error) => panic!("Cannot get query results: {error}"),
        }
    }

    /// Returns the query item for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// # Example
    ///
    /// Here, `get_mut` is used to retrieve the exact query item of the entity specified by the `PoisonedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
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
    ///
    /// # See also
    ///
    /// - [`get`](Self::get) to get a read-only query item.
    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Result<Q::Item<'_>, QueryEntityError> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .get_unchecked_manual(self.world, entity, self.last_run, self.this_run)
        }
    }

    /// Returns the query items for the given array of [`Entity`].
    ///
    /// The returned query items are in the same order as the input.
    /// In case of a nonexisting entity, duplicate entities or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// # See also
    ///
    /// - [`get_many`](Self::get_many) to get read-only query items.
    /// - [`many_mut`](Self::many_mut) for the panicking version.
    #[inline]
    pub fn get_many_mut<const N: usize>(
        &mut self,
        entities: [Entity; N],
    ) -> Result<[Q::Item<'_>; N], QueryEntityError> {
        // SAFETY: scheduler ensures safe Query world access
        unsafe {
            self.state
                .get_many_unchecked_manual(self.world, entities, self.last_run, self.this_run)
        }
    }

    /// Returns the query items for the given array of [`Entity`].
    ///
    /// # Panics
    ///
    /// This method panics if there is a query mismatch, a non-existing entity, or the same `Entity` is included more than once in the array.
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
    ///
    /// # See also
    ///
    /// - [`get_many_mut`](Self::get_many_mut) for the non panicking version.
    /// - [`many`](Self::many) to get read-only query items.
    #[inline]
    pub fn many_mut<const N: usize>(&mut self, entities: [Entity; N]) -> [Q::Item<'_>; N] {
        match self.get_many_mut(entities) {
            Ok(items) => items,
            Err(error) => panic!("Cannot get query result: {error}"),
        }
    }

    /// Returns the query item for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees.
    /// You must make sure this call does not result in multiple mutable references to the same component.
    ///
    /// # See also
    ///
    /// - [`get_mut`](Self::get_mut) for the safe version.
    #[inline]
    pub unsafe fn get_unchecked(&self, entity: Entity) -> Result<Q::Item<'_>, QueryEntityError> {
        // SEMI-SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state
            .get_unchecked_manual(self.world, entity, self.last_run, self.this_run)
    }

    /// Returns a shared reference to the component `T` of the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// # Example
    ///
    /// Here, `get_component` is used to retrieve the `Character` component of the entity specified by the `SelectedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
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
    ///
    /// # See also
    ///
    /// - [`component`](Self::component) a panicking version of this function.
    /// - [`get_component_mut`](Self::get_component_mut) to get a mutable reference of a component.
    #[inline]
    pub fn get_component<T: Component>(&self, entity: Entity) -> Result<&T, QueryComponentError> {
        self.state.get_component(self.world, entity)
    }

    /// Returns a mutable reference to the component `T` of the given entity.
    ///
    /// In case of a nonexisting entity, mismatched component or missing write access, a [`QueryComponentError`] is returned instead.
    ///
    /// # Example
    ///
    /// Here, `get_component_mut` is used to retrieve the `Health` component of the entity specified by the `PoisonedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
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
    ///
    /// # See also
    ///
    /// - [`component_mut`](Self::component_mut) a panicking version of this function.
    /// - [`get_component`](Self::get_component) to get a shared reference of a component.
    #[inline]
    pub fn get_component_mut<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        // SAFETY: unique access to query (preventing aliased access)
        unsafe { self.get_component_unchecked_mut(entity) }
    }

    /// Returns a shared reference to the component `T` of the given [`Entity`].
    ///
    /// # Panics
    ///
    /// Panics in case of a nonexisting entity or mismatched component.
    ///
    /// # See also
    ///
    /// - [`get_component`](Self::get_component) a non-panicking version of this function.
    /// - [`component_mut`](Self::component_mut) to get a mutable reference of a component.
    #[inline]
    #[track_caller]
    pub fn component<T: Component>(&self, entity: Entity) -> &T {
        self.state.component(self.world, entity)
    }

    /// Returns a mutable reference to the component `T` of the given entity.
    ///
    /// # Panics
    ///
    /// Panics in case of a nonexisting entity, mismatched component or missing write access.
    ///
    /// # See also
    ///
    /// - [`get_component_mut`](Self::get_component_mut) a non-panicking version of this function.
    /// - [`component`](Self::component) to get a shared reference of a component.
    #[inline]
    #[track_caller]
    pub fn component_mut<T: Component>(&mut self, entity: Entity) -> Mut<'_, T> {
        match self.get_component_mut(entity) {
            Ok(component) => component,
            Err(error) => {
                panic!(
                    "Cannot get component `{:?}` from {entity:?}: {error}",
                    TypeId::of::<T>()
                )
            }
        }
    }

    /// Returns a mutable reference to the component `T` of the given entity.
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryComponentError`] is returned instead.
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees.
    /// You must make sure this call does not result in multiple mutable references to the same component.
    ///
    /// # See also
    ///
    /// - [`get_component_mut`](Self::get_component_mut) for the safe version.
    #[inline]
    pub unsafe fn get_component_unchecked_mut<T: Component>(
        &self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        // This check is required to ensure soundness in the case of `to_readonly().get_component_mut()`
        // See the comments on the `force_read_only_component_access` field for more info.
        if self.force_read_only_component_access {
            return Err(QueryComponentError::MissingWriteAccess);
        }

        // SAFETY: The above check ensures we are not a readonly query.
        // It is the callers responsibility to ensure multiple mutable access is not provided.
        unsafe {
            self.state
                .get_component_unchecked_mut(self.world, entity, self.last_run, self.this_run)
        }
    }

    /// Returns a single read-only query item when there is exactly one entity matching the query.
    ///
    /// # Panics
    ///
    /// This method panics if the number of query items is **not** exactly one.
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
    /// # See also
    ///
    /// - [`get_single`](Self::get_single) for the non-panicking version.
    /// - [`single_mut`](Self::single_mut) to get the mutable query item.
    #[track_caller]
    pub fn single(&self) -> ROQueryItem<'_, Q> {
        self.get_single().unwrap()
    }

    /// Returns a single read-only query item when there is exactly one entity matching the query.
    ///
    /// If the number of query items is not exactly one, a [`QuerySingleError`] is returned instead.
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
    ///
    /// # See also
    ///
    /// - [`get_single_mut`](Self::get_single_mut) to get the mutable query item.
    /// - [`single`](Self::single) for the panicking version.
    #[inline]
    pub fn get_single(&self) -> Result<ROQueryItem<'_, Q>, QuerySingleError> {
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
    /// # Panics
    ///
    /// This method panics if the number of query item is **not** exactly one.
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
    /// # See also
    ///
    /// - [`get_single_mut`](Self::get_single_mut) for the non-panicking version.
    /// - [`single`](Self::single) to get the read-only query item.
    #[track_caller]
    pub fn single_mut(&mut self) -> Q::Item<'_> {
        self.get_single_mut().unwrap()
    }

    /// Returns a single query item when there is exactly one entity matching the query.
    ///
    /// If the number of query items is not exactly one, a [`QuerySingleError`] is returned instead.
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
    ///
    /// # See also
    ///
    /// - [`get_single`](Self::get_single) to get the read-only query item.
    /// - [`single_mut`](Self::single_mut) for the panicking version.
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

    /// Returns `true` if there are no query items.
    ///
    /// # Example
    ///
    /// Here, the score is increased only if an entity with a `Player` component is present in the world:
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct Player;
    /// # #[derive(Resource)]
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
        // SAFETY:
        // - `self.world` has permission to read any data required by the WorldQuery.
        // - `&self` ensures that no one currently has write access.
        // - `self.world` matches `self.state`.
        unsafe {
            self.state
                .is_empty_unsafe_world_cell(self.world, self.last_run, self.this_run)
        }
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
    /// # #[derive(Resource)]
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
                .get_unchecked_manual(self.world, entity, self.last_run, self.this_run)
                .is_ok()
        }
    }
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> IntoIterator for &'w Query<'_, 's, Q, F> {
    type Item = ROQueryItem<'w, Q>;
    type IntoIter = QueryIter<'w, 's, Q::ReadOnly, F::ReadOnly>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> IntoIterator for &'w mut Query<'_, 's, Q, F> {
    type Item = Q::Item<'w>;
    type IntoIter = QueryIter<'w, 's, Q, F>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<'w, 's, Q: ReadOnlyWorldQuery, F: ReadOnlyWorldQuery> Query<'w, 's, Q, F> {
    /// Returns the query item for the given [`Entity`], with the actual "inner" world lifetime.
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// This can only return immutable data (mutable data will be cast to an immutable form).
    /// See [`get_mut`](Self::get_mut) for queries that contain at least one mutable component.
    ///
    /// # Example
    ///
    /// Here, `get` is used to retrieve the exact query item of the entity specified by the
    /// `SelectedCharacter` resource.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Resource)]
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
    pub fn get_inner(&self, entity: Entity) -> Result<ROQueryItem<'w, Q>, QueryEntityError> {
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

    /// Returns an [`Iterator`] over the query items, with the actual "inner" world lifetime.
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
    pub fn iter_inner(&self) -> QueryIter<'w, 's, Q::ReadOnly, F::ReadOnly> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .as_readonly()
                .iter_unchecked_manual(self.world, self.last_run, self.this_run)
        }
    }
}
