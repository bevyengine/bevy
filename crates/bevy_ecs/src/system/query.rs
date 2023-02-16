use crate::{
    component::Component,
    entity::Entity,
    query::{
        BatchingStrategy, QueryBatch, QueryCombinationIter, QueryEntityError, QueryItem, QueryIter,
        QueryManyIter, QueryParIter, QuerySingleError, QueryState, ROQueryBatch, ROQueryItem,
        ReadOnlyWorldQuery, WorldQuery, WorldQueryBatch,
    },
    world::{Mut, World},
};
use std::{any::TypeId, borrow::Borrow, fmt::Debug};

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
/// # Accessing query items
///
/// The following table summarizes the behavior of the safe methods that can be used to get query items.
///
/// |Query methods|Effect|
/// |:---:|---|
/// |[`iter`]\([`_mut`][`iter_mut`])|Returns an iterator over all query items.|
/// |[`for_each`]\([`_mut`][`for_each_mut`]),<br>[`par_iter`]\([`_mut`][`par_iter_mut`])|Runs a specified function for each query item.|
/// |[`iter_many`]\([`_mut`][`iter_many_mut`])|Iterates or runs a specified function over query items generated by a list of entities.|
/// |[`iter_combinations`]\([`_mut`][`iter_combinations_mut`])|Returns an iterator over all combinations of a specified number of query items.|
/// |[`get`]\([`_mut`][`get_mut`])|Returns the query item for the specified entity.|
/// |[`many`]\([`_mut`][`many_mut`]),<br>[`get_many`]\([`_mut`][`get_many_mut`])|Returns the query items for the specified entities.|
/// |[`single`]\([`_mut`][`single_mut`]),<br>[`get_single`]\([`_mut`][`get_single_mut`])|Returns the query item while verifying that there aren't others.|
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
/// |[`iter`]\([`_mut`][`iter_mut`])|O(n)|
/// |[`for_each`]\([`_mut`][`for_each_mut`]),<br>[`par_iter`]\([`_mut`][`par_iter_mut`])|O(n)|
/// |[`iter_many`]\([`_mut`][`iter_many_mut`])|O(k)|
/// |[`iter_combinations`]\([`_mut`][`iter_combinations_mut`])|O(<sub>n</sub>C<sub>r</sub>)|
/// |[`get`]\([`_mut`][`get_mut`])|O(1)|
/// |([`get_`][`get_many`])[`many`]|O(k)|
/// |([`get_`][`get_many_mut`])[`many_mut`]|O(k<sup>2</sup>)|
/// |[`single`]\([`_mut`][`single_mut`]),<br>[`get_single`]\([`_mut`][`get_single_mut`])|O(a)|
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
    world: &'world World,
    state: &'state QueryState<Q, F>,
    last_change_tick: u32,
    change_tick: u32,
    // SAFETY: This is used to ensure that `get_component_mut::<C>` properly fails when a Query writes C
    // and gets converted to a read-only query using `to_readonly`. Without checking this, `get_component_mut` relies on
    // QueryState's archetype_component_access, which will continue allowing write access to C after being cast to
    // the read-only variant. This whole situation is confusing and error prone. Ideally this is a temporary hack
    // until we sort out a cleaner alternative.
    force_read_only_component_access: bool,
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> std::fmt::Debug for Query<'w, 's, Q, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Query {{ matched entities: {}, world: {:?}, state: {:?}, last_change_tick: {}, change_tick: {} }}", self.iter().count(), self.world, self.state, self.last_change_tick, self.change_tick)
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
        world: &'w World,
        state: &'s QueryState<Q, F>,
        last_change_tick: u32,
        change_tick: u32,
        force_read_only_component_access: bool,
    ) -> Self {
        state.validate_world(world);

        Self {
            force_read_only_component_access,
            world,
            state,
            last_change_tick,
            change_tick,
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
                self.last_change_tick,
                self.change_tick,
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
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .iter_unchecked_manual(self.world, self.last_change_tick, self.change_tick)
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
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.iter_many_unchecked_manual(
                entities,
                self.world,
                self.last_change_tick,
                self.change_tick,
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
        // SEMI-SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state
            .iter_unchecked_manual(self.world, self.last_change_tick, self.change_tick)
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
        // SEMI-SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        self.state.iter_combinations_unchecked_manual(
            self.world,
            self.last_change_tick,
            self.change_tick,
        )
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
        self.state.iter_many_unchecked_manual(
            entities,
            self.world,
            self.last_change_tick,
            self.change_tick,
        )
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

    /// See [`QueryState<Q,F>::for_each_batched`](QueryState<Q,F>::for_each_batched) for how to use this function.
    #[inline]
    pub fn for_each_batched<'a, const N: usize>(
        &'a mut self,
        func: impl FnMut(ROQueryItem<'a, Q>),
        func_batch: impl FnMut(ROQueryBatch<'a, Q, N>),
    ) where
        <Q as WorldQuery>::ReadOnly: WorldQueryBatch<N>,
    {
        // SAFETY: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.as_readonly().for_each_unchecked_manual_batched(
                self.world,
                func,
                func_batch,
                self.last_change_tick,
                self.change_tick,
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

    /// This is a "batched" version of [`for_each_mut`](Self::for_each_mut) that accepts a batch size `N`, which should be a power of two.
    /// The advantage of using batching in queries is that it enables SIMD acceleration (vectorization) of your code to help you meet your performance goals.
    /// This function accepts two arguments, `func`, and `func_batch` which represent the "scalar" and "vector" (or "batched") paths of your code respectively.
    /// Each "batch" contains `N` query results, in order.  **Consider enabling AVX if you are on x86 when using this API**.
    ///
    /// # A very brief introduction to SIMD
    ///
    /// SIMD, or Single Instruction, Multiple Data, is a paradigm that allows a single instruction to operate on multiple datums in parallel.
    /// It is most commonly seen in "vector" instruction set extensions such as AVX and NEON, where it is possible to, for example, add
    /// two arrays of `[f32; 4]` together in a single instruction.  When used appropriately, SIMD is a very powerful tool that can greatly accelerate certain types of workloads.
    /// An introductory treatment of SIMD can be found [on Wikipedia](https://en.wikipedia.org/wiki/Single_instruction,_multiple_data) for interested readers.
    ///
    /// [Vectorization](https://stackoverflow.com/questions/1422149/what-is-vectorization) is an informal term to describe optimizing code to leverage these SIMD instruction sets.
    ///
    /// # When should I consider batching for my query?
    ///
    /// The first thing you should consider is if you are meeting your performance goals.  Batching a query is fundamentally an optimization, and if your application is meeting performance requirements
    /// already, then (other than for your own entertainment) you won't get much benefit out of batching.  If you are having performance problems though, the next step is to
    /// use a [profiler](https://nnethercote.github.io/perf-book/profiling.html) to determine the running characteristics of your code.
    /// If, after profiling your code, you have determined that a substantial amount of time is being processing a query, and it's hindering your performance goals,
    /// then it might be worth it to consider batching to meet them.
    ///
    /// One of the main tradeoffs with batching your queries is that there will be an increased complexity from maintaining both code paths: `func` and `func_batch`
    /// semantically should be doing the same thing, and it should always be possible to interchange them without visible program effects.
    ///
    /// # Getting maximum performance for your application
    ///
    /// Bevy aims to provide best-in-class performance for architectures that do not encode alignment into SIMD instructions.  This includes (but is not limited to) AVX and onwards for x86,
    /// ARM 64-bit, RISC-V, and `WebAssembly`g.  The majority of architectures created since 2010 have this property.  It is important to note that although these architectures
    /// do not encode alignment in the instruction set itself, they still benefit from memory operands being naturally aligned.
    ///
    /// On other instruction sets, code generation may be worse than it could be due to alignment information not being statically available in batches.
    /// For example, 32-bit ARM NEON instructions encode an alignment hint that is not present in the 64-bit ARM versions,
    /// and this hint will be set to "unaligned" even if the data itself is aligned.  Whether this incurs a performance penalty is implementation defined.
    ///
    /// **As a result, it is recommended, if you are on x86, to enable at minimum AVX support for maximum performance when executing batched queries**.
    /// It's a good idea to enable in general, too.  SSE4.2 and below will have slightly worse performance as more unaligned loads will be produced,
    /// with work being done in registers, since it requires memory operands to be aligned whereas AVX relaxes this restriction.
    ///
    /// To enable AVX support for your application, add "-C target-feature=+avx" to your `RUSTFLAGS`.  See the [Rust docs](https://doc.rust-lang.org/cargo/reference/config.html)
    /// for details on how to set this as a default for your project.
    ///
    /// When the `generic_const_exprs` feature of Rust is stable, Bevy will be able to encode the alignment of the batch into the batch itself and provide maximum performance
    /// on architectures that encode alignment into SIMD instruction opcodes as well.
    ///
    /// # What kinds of queries make sense to batch?
    ///
    /// Usually math related ones. Anything involving floats is a possible candidate.  Depending on your component layout, you may need to perform a data layout conversion
    /// to batch the query optimally.  This Wikipedia page on ["array of struct" and "struct of array" layouts](https://en.wikipedia.org/wiki/AoS_and_SoA) is a good starter on
    /// this topic, as is this [Intel blog post](https://www.intel.com/content/www/us/en/developer/articles/technical/memory-layout-transformations.html).
    ///
    /// Vectorizing code can be a very deep subject to get into.
    /// Sometimes it can be very straightfoward to accomplish what you want to do, and other times it takes a bit of playing around to make your problem fit the SIMD model.
    ///
    /// # Will batching always make my queries faster?
    ///
    /// Unfortunately it will not.  A suboptimally written batched query will probably perform worse than a straightforward `for_each_mut` query.  Data layout conversion,
    /// for example, carries overhead that may not always be worth it. Fortunately, your profiler can help you identify these situations.
    ///
    /// Think of batching as a tool in your performance toolbox rather than the preferred way of writing your queries.
    ///
    /// # What kinds of queries are batched right now?
    ///
    /// Currently, only "Dense" queries are actually batched; other queries will only use `func` and never call `func_batch`.  This will improve
    /// in the future.
    ///
    /// # Usage:
    ///
    /// * `N` should be a power of 2, and ideally be a multiple of your SIMD vector size.
    /// * `func_batch` receives [`Batch`](bevy_ptr::Batch)es of `N` components.
    /// * `func` functions exactly as does in [`for_each_mut`](Self::for_each_mut) -- it receives "scalar" (non-batched) components.
    ///
    /// In other words, `func_batch` composes the "fast path" of your query, and `func` is the "slow path".
    ///
    /// In general, when using this function, be mindful of the types of filters being used with your query, as these can fragment your batches
    /// and cause the scalar path to be taken more often.
    ///
    /// **Note**: It is well known that [`array::map`](https://doc.rust-lang.org/std/primitive.array.html#method.map) optimizes poorly at the moment.
    /// Avoid using it until the upstream issues are resolved: [#86912](https://github.com/rust-lang/rust/issues/86912) and [#102202](https://github.com/rust-lang/rust/issues/102202).
    /// Manually unpack your batches in the meantime for optimal codegen.
    ///
    /// **Note**: It is always valid for the implementation of this function to only call `func`.  Currently, batching is only supported for "Dense" queries.
    /// Calling this function on any other query type will result in only the slow path being executed (e.g., queries with Sparse components.)
    /// More query types may become batchable in the future.
    ///
    /// **Note**: Although this function provides the groundwork for writing performance-portable SIMD-accelerated queries, you will still need to take into account
    /// your target architecture's capabilities.  The batch size will likely need to be tuned for your application, for example.
    /// When SIMD becomes stabilized in Rust, it will be possible to write code that is generic over the batch width, but some degree of tuning will likely always be
    /// necessary.  Think of this as a tool at your disposal to meet your performance goals.
    ///
    /// The following is an example of using batching to accelerate a simplified collision detection system.  It is written using x86 AVX intrinsics, since `std::simd` is not stable
    /// yet.  You can, of course, use `std::simd` in your own code if you prefer, or adapt this example to other instruction sets.
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_math::Vec3;
    /// use core::arch::x86_64::*;
    ///
    /// #[derive(Component)]
    /// struct Position(Vec3);
    ///
    /// #[derive(Component)]
    /// #[repr(transparent)]
    /// struct Health(f32);
    ///
    /// // A plane describing solid geometry, (x,y,z) = n with d such that nx + d = 0
    /// #[derive(Component)]
    /// struct Wall(Vec3, f32);
    ///
    /// // AoS to SoA data layout conversion for x86 AVX.
    /// // This code has been adapted from:
    /// //   https://www.intel.com/content/dam/develop/external/us/en/documents/normvec-181650.pdf
    /// #[inline(always)]
    /// // This example is written in a way that benefits from inlined data layout conversion.
    /// fn aos_to_soa_83(aos_inner: &[Vec3; 8]) -> [__m256; 3] {
    ///    unsafe {
    ///        //# SAFETY: Vec3 is repr(C) for x86_64
    ///        let mx0 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(0));
    ///        let mx1 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(4));
    ///        let mx2 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(8));
    ///        let mx3 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(12));
    ///        let mx4 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(16));
    ///        let mx5 = _mm_loadu_ps((aos_inner as *const Vec3 as *const f32).offset(20));
    ///
    ///        let mut m03 = _mm256_castps128_ps256(mx0); // load lower halves
    ///        let mut m14 = _mm256_castps128_ps256(mx1);
    ///        let mut m25 = _mm256_castps128_ps256(mx2);
    ///        m03 = _mm256_insertf128_ps(m03, mx3, 1); // load upper halves
    ///        m14 = _mm256_insertf128_ps(m14, mx4, 1);
    ///        m25 = _mm256_insertf128_ps(m25, mx5, 1);
    ///
    ///        let xy = _mm256_shuffle_ps::<0b10011110>(m14, m25); // upper x's and y's
    ///        let yz = _mm256_shuffle_ps::<0b01001001>(m03, m14); // lower y's and z's
    ///        let x = _mm256_shuffle_ps::<0b10001100>(m03, xy);
    ///        let y = _mm256_shuffle_ps::<0b11011000>(yz, xy);
    ///        let z = _mm256_shuffle_ps::<0b11001101>(yz, m25);
    ///        [x, y, z]
    ///    }
    ///}
    ///
    /// // Perform collision detection against a set of Walls, forming a convex polygon.
    /// // Each entity has a Position and some Health (initialized to 100.0).
    /// // If the position of an entity is found to be outside of a Wall, decrement its "health" by 1.0.
    /// // The effect is cumulative based on the number of walls.  
    /// // An entity entirely inside the convex polygon will have its health remain unchanged.
    /// fn batched_collision_detection_system(mut pos_healths: Query<(&Position, &mut Health)>,
    ///     walls: Query<&Wall>) {
    ///
    ///    // Conceptually, this system is executed using two loops: the outer "batched" loop receiving
    ///    // batches of 8 Positions and Health components at a time, and the inner loop iterating over
    ///    // the Walls.
    ///
    ///    // There's more than one way to vectorize this system -- this example may not be optimal.
    ///    pos_healths.for_each_mut_batched::<8>(
    ///        |(position, mut health)| {
    ///            // This forms the scalar path: it behaves just like `for_each_mut`.
    ///
    ///            // Optional: disable change detection for more performance.
    ///            let health = &mut health.bypass_change_detection().0;
    ///
    ///            // Test each (Position,Health) against each Wall.
    ///            walls.for_each(|wall| {
    ///                let plane = wall.0;
    ///
    ///                // Test which side of the wall we are on
    ///                let dotproj = plane.dot(position.0);
    ///
    ///                // Test against the Wall's displacement/discriminant value
    ///                if dotproj < wall.1 {
    ///                    //Ouch! Take damage!
    ///                    *health -= 1.0;
    ///               }
    ///            });
    ///        },
    ///        |(positions, mut healths)| {
    ///            // This forms the vector path: the closure receives a batch of
    ///            // 8 Positions and 8 Healths as arrays.
    ///
    ///            // Optional: disable change detection for more performance.
    ///            let healths = healths.bypass_change_detection();
    ///
    ///            // Treat the Health batch as a batch of 8 f32s.
    ///            unsafe {
    ///            // # SAFETY: Health is repr(transprent)!
    ///            let healths_raw = healths as *mut Health as *mut f32;
    ///            let mut healths = _mm256_loadu_ps(healths_raw);
    ///
    ///            // NOTE: array::map optimizes poorly -- it is recommended to unpack your arrays
    ///            // manually as shown to avoid spurious copies which will impact your performance.
    ///            let [p0, p1, p2, p3, p4, p5, p6, p7] = positions;
    ///
    ///            // Perform data layout conversion from AoS to SoA.
    ///            // ps_x will receive all of the X components of the positions,
    ///            // ps_y will receive all of the Y components
    ///            // and ps_z will receive all of the Z's.
    ///            let [ps_x, ps_y, ps_z] =
    ///                aos_to_soa_83(&[p0.0, p1.0, p2.0, p3.0, p4.0, p5.0, p6.0, p7.0]);
    ///
    ///            // Iterate over each wall without batching.
    ///            walls.for_each(|wall| {
    ///                // Test each wall against all 8 positions at once.  The "broadcast" intrinsic
    ///                // helps us achieve this by duplicating the Wall's X coordinate over an entire
    ///                // vector register, e.g., [X X ... X]. The same goes for the Wall's Y and Z
    ///                // coordinates.
    ///
    ///                // This is the exact same formula as implemented in the scalar path, but
    ///                // modified to be calculated in parallel across each lane.
    ///
    ///                // Multiply all of the X coordinates of each Position against Wall's Normal X
    ///                let xs_dot = _mm256_mul_ps(ps_x, _mm256_broadcast_ss(&wall.0.x));
    ///                // Multiply all of the Y coordinates of each Position against Wall's Normal Y
    ///                let ys_dot = _mm256_mul_ps(ps_y, _mm256_broadcast_ss(&wall.0.y));
    ///                // Multiply all of the Z coordinates of each Position against Wall's Normal Z
    ///                let zs_dot = _mm256_mul_ps(ps_z, _mm256_broadcast_ss(&wall.0.z));
    ///
    ///                // Now add them together: the result is a vector register containing the dot
    ///                // product of each Position against the Wall's Normal vector.
    ///                let dotprojs = _mm256_add_ps(_mm256_add_ps(xs_dot, ys_dot), zs_dot);
    ///
    ///                // Take the Wall's discriminant/displacement value and broadcast it like before.
    ///                let wall_d = _mm256_broadcast_ss(&wall.1);
    ///
    ///                // Compare each dot product against the Wall's discriminant, using the
    ///                // "Less Than" relation as we did in the scalar code.
    ///                // The result will be be either -1 or zero *as an integer*.
    ///                let cmp = _mm256_cmp_ps::<_CMP_LT_OS>(dotprojs, wall_d);
    ///
    ///                // Convert the integer values back to f32 values (-1.0 or 0.0).
    ///                // These form the damage values for each entity.
    ///                let damages = _mm256_cvtepi32_ps(_mm256_castps_si256(cmp)); //-1.0 or 0.0
    ///
    ///                // Update the healths of each entity being processed with the results of the
    ///                // collision detection.
    ///                healths = _mm256_add_ps(healths, damages);
    ///            });
    ///            // Now that all Walls have been processed, write the final updated Health values
    ///            // for this batch of entities back to main memory.
    ///            _mm256_storeu_ps(healths_raw, healths);
    ///            }
    ///        },
    ///    );
    /// }
    ///
    /// # bevy_ecs::system::assert_is_system(batched_collision_detection_system);
    /// ```
    #[inline]
    pub fn for_each_mut_batched<'a, const N: usize>(
        &'a mut self,
        func: impl FnMut(QueryItem<'a, Q>),
        func_batch: impl FnMut(QueryBatch<'a, Q, N>),
    ) where
        Q: WorldQueryBatch<N>,
    {
        // SAFETY: system runs without conflicts with other systems. same-system queries have runtime
        // borrow checks when they conflict
        unsafe {
            self.state.for_each_unchecked_manual_batched(
                self.world,
                func,
                func_batch,
                self.last_change_tick,
                self.change_tick,
            );
        };
    }

    /// Returns a parallel iterator over the query results for the given [`World`].
    ///
    /// This can only be called for read-only queries, see [`par_iter_mut`] for write-queries.
    ///
    /// [`par_iter_mut`]: Self::par_iter_mut
    #[inline]
    pub fn par_iter(&mut self) -> QueryParIter<'_, '_, Q::ReadOnly, F::ReadOnly> {
        QueryParIter {
            world: self.world,
            state: self.state.as_readonly(),
            batching_strategy: BatchingStrategy::new(),
        }
    }

    /// Returns a parallel iterator over the query results for the given [`World`].
    ///
    /// This can only be called for mutable queries, see [`par_iter`] for read-only-queries.
    ///
    /// [`par_iter`]: Self::par_iter
    #[inline]
    pub fn par_iter_mut(&mut self) -> QueryParIter<'_, '_, Q, F> {
        QueryParIter {
            world: self.world,
            state: self.state,
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
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns the read-only query items for the given array of [`Entity`].
    ///
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
        self.get_many(entities).unwrap()
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
            self.state.get_unchecked_manual(
                self.world,
                entity,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }

    /// Returns the query items for the given array of [`Entity`].
    ///
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
            self.state.get_many_unchecked_manual(
                self.world,
                entities,
                self.last_change_tick,
                self.change_tick,
            )
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
        self.get_many_mut(entities).unwrap()
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
            .get_unchecked_manual(self.world, entity, self.last_change_tick, self.change_tick)
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
    /// - [`get_component_mut`](Self::get_component_mut) to get a mutable reference of a component.
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

    /// Returns a mutable reference to the component `T` of the given entity.
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
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
    /// - [`get_component`](Self::get_component) to get a shared reference of a component.
    #[inline]
    pub fn get_component_mut<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        // SAFETY: unique access to query (preventing aliased access)
        unsafe { self.get_component_unchecked_mut(entity) }
    }

    /// Returns a mutable reference to the component `T` of the given entity.
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
    /// - [`get_component_mut`](Self::get_component_mut) for the safe version.
    #[inline]
    pub unsafe fn get_component_unchecked_mut<T: Component>(
        &self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryComponentError> {
        // SAFETY: this check is required to ensure soundness in the case of `to_readonly().get_component_mut()`
        // See the comments on the `force_read_only_component_access` field for more info.
        if self.force_read_only_component_access {
            return Err(QueryComponentError::MissingWriteAccess);
        }
        let world = self.world;
        let entity_ref = world
            .as_unsafe_world_cell_migration_internal()
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
                .get_mut_using_ticks::<T>(self.last_change_tick, self.change_tick)
                .ok_or(QueryComponentError::MissingComponent)
        } else {
            Err(QueryComponentError::MissingWriteAccess)
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
                self.last_change_tick,
                self.change_tick,
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
            self.state.get_single_unchecked_manual(
                self.world,
                self.last_change_tick,
                self.change_tick,
            )
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
                .get_unchecked_manual(self.world, entity, self.last_change_tick, self.change_tick)
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

/// An error that occurs when retrieving a specific [`Entity`]'s component from a [`Query`]
#[derive(Debug, PartialEq, Eq)]
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
                self.last_change_tick,
                self.change_tick,
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
            self.state.as_readonly().iter_unchecked_manual(
                self.world,
                self.last_change_tick,
                self.change_tick,
            )
        }
    }
}
