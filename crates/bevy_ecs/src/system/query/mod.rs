mod query_set;

pub use query_set::*;

use bevy_hecs::{
    ArchetypeComponent, Batch, BatchedIter, Component, ComponentError, Entity, Fetch, Mut,
    Query as HecsQuery, QueryIter, ReadOnlyFetch, TypeAccess, World,
};
use bevy_tasks::ParallelIterator;
use std::marker::PhantomData;

/// Provides scoped access to a World according to a given [`HecsQuery`]
#[derive(Debug)]
pub struct Query<'a, Q: HecsQuery> {
    pub(crate) query: StatefulQuery<'a, Q, ()>,
}

impl<'a, Q: HecsQuery> Query<'a, Q>
where
    Q::Fetch: for<'b> Fetch<'b, State = ()>,
{
    #[inline]
    pub fn new(world: &'a World, component_access: &'a TypeAccess<ArchetypeComponent>) -> Self {
        Self {
            query: StatefulQuery {
                world,
                component_access,
                state: (),
                _marker: PhantomData::default(),
            },
        }
    }

    /// Iterates over the query results. This can only be called for read-only queries
    pub fn iter(&self) -> QueryIter<'_, '_, Q, ()>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        self.query.iter()
    }

    /// Iterates over the query results
    pub fn iter_mut(&mut self) -> QueryIter<'_, '_, Q, ()> {
        self.query.iter_mut()
    }

    /// Iterates over the query results
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple mutable references to the same component
    pub unsafe fn iter_unsafe(&self) -> QueryIter<'_, '_, Q, ()> {
        self.query.iter_unsafe()
    }

    #[inline]
    pub fn par_iter(&self, batch_size: usize) -> ParIter<'_, '_, Q, ()>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        self.query.par_iter(batch_size)
    }

    #[inline]
    pub fn par_iter_mut(&mut self, batch_size: usize) -> ParIter<'_, '_, Q, ()> {
        self.query.par_iter_mut(batch_size)
    }

    /// Gets the query result for the given `entity`
    pub fn get(&self, entity: Entity) -> Result<<Q::Fetch as Fetch>::Item, QueryError>
    where
        Q::Fetch: ReadOnlyFetch + for<'b> Fetch<'b, State = ()>,
    {
        self.query.get(entity)
    }

    /// Gets the query result for the given `entity`
    pub fn get_mut(&mut self, entity: Entity) -> Result<<Q::Fetch as Fetch>::Item, QueryError>
    where
        Q::Fetch: for<'b> Fetch<'b, State = ()>,
    {
        self.query.get_mut(entity)
    }

    /// Gets the query result for the given `entity`
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple mutable references to the same component
    pub unsafe fn get_unsafe(&self, entity: Entity) -> Result<<Q::Fetch as Fetch>::Item, QueryError>
    where
        Q::Fetch: for<'b> Fetch<'b, State = ()>,
    {
        self.query.get_unsafe(entity)
    }

    /// Gets a reference to the entity's component of the given type. This will fail if the entity does not have
    /// the given component type or if the given component type does not match this query.
    pub fn get_component<T: Component>(&self, entity: Entity) -> Result<&T, QueryError> {
        self.query.get_component::<T>(entity)
    }

    /// Gets a mutable reference to the entity's component of the given type. This will fail if the entity does not have
    /// the given component type or if the given component type does not match this query.
    pub fn get_component_mut<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryError> {
        self.query.get_component_mut::<T>(entity)
    }

    /// Gets a mutable reference to the entity's component of the given type. This will fail if the entity does not have
    /// the given component type
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple mutable references to the same component
    pub unsafe fn get_component_unsafe<T: Component>(
        &self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryError> {
        self.query.get_component_unsafe(entity)
    }

    pub fn removed<C: Component>(&self) -> &[Entity] {
        self.query.removed::<C>()
    }

    /// Sets the entity's component to the given value. This will fail if the entity does not already have
    /// the given component type or if the given component type does not match this query.
    pub fn set<T: Component>(&mut self, entity: Entity, component: T) -> Result<(), QueryError> {
        self.query.set(entity, component)
    }
}

/// Provides scoped access to a World according to a given [`HecsQuery`]. Essentially identical to
/// [`Query`] except that it supports passing extra state into the query, useful for special kinds
/// of queries such as [`DynamicQuery`].
#[derive(Debug)]
pub struct StatefulQuery<'a, Q: HecsQuery, S> {
    pub(crate) world: &'a World,
    pub(crate) component_access: &'a TypeAccess<ArchetypeComponent>,
    pub(crate) state: S,
    _marker: PhantomData<Q>,
}

impl<'a, Q: HecsQuery, S> StatefulQuery<'a, Q, S>
where
    Q::Fetch: for<'b> Fetch<'b, State = S>,
{
    #[inline]
    pub fn new(
        world: &'a World,
        component_access: &'a TypeAccess<ArchetypeComponent>,
        state: S,
    ) -> Self {
        Self {
            world,
            component_access,
            state,
            _marker: PhantomData::default(),
        }
    }

    /// Iterates over the query results. This can only be called for read-only queries
    pub fn iter(&self) -> QueryIter<'_, '_, Q, S>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe { self.world.query_unchecked_stateful(&self.state) }
    }

    /// Iterates over the query results
    pub fn iter_mut(&mut self) -> QueryIter<'_, '_, Q, S> {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe { self.world.query_unchecked_stateful(&self.state) }
    }

    /// Iterates over the query results
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple mutable references to the same component
    pub unsafe fn iter_unsafe(&self) -> QueryIter<'_, '_, Q, S> {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        self.world.query_unchecked_stateful(&self.state)
    }

    #[inline]
    pub fn par_iter(&self, batch_size: usize) -> ParIter<'_, '_, Q, S>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe {
            ParIter::new(
                self.world
                    .query_batched_unchecked_stateful(batch_size, &self.state),
            )
        }
    }

    #[inline]
    pub fn par_iter_mut(&mut self, batch_size: usize) -> ParIter<'_, '_, Q, S> {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe {
            ParIter::new(
                self.world
                    .query_batched_unchecked_stateful(batch_size, &self.state),
            )
        }
    }

    /// Gets the query result for the given `entity`
    pub fn get(&self, entity: Entity) -> Result<<Q::Fetch as Fetch>::Item, QueryError>
    where
        Q::Fetch: ReadOnlyFetch + for<'b> Fetch<'b, State = S>,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.world
                .query_one_unchecked_stateful::<Q, S>(entity, &self.state)
                .map_err(|_err| QueryError::NoSuchEntity)
        }
    }

    /// Gets the query result for the given `entity`
    pub fn get_mut(&mut self, entity: Entity) -> Result<<Q::Fetch as Fetch>::Item, QueryError>
    where
        Q::Fetch: for<'b> Fetch<'b, State = S>,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.world
                .query_one_unchecked_stateful::<Q, S>(entity, &self.state)
                .map_err(|_err| QueryError::NoSuchEntity)
        }
    }

    /// Gets the query result for the given `entity`
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple mutable references to the same component
    pub unsafe fn get_unsafe(
        &self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, QueryError> {
        self.world
            .query_one_unchecked_stateful::<Q, S>(entity, &self.state)
            .map_err(|_err| QueryError::NoSuchEntity)
    }

    /// Gets a reference to the entity's component of the given type. This will fail if the entity does not have
    /// the given component type or if the given component type does not match this query.
    pub fn get_component<T: Component>(&self, entity: Entity) -> Result<&T, QueryError> {
        if let Some(location) = self.world.get_entity_location(entity) {
            if self
                .component_access
                .is_read_or_write(&ArchetypeComponent::new::<T>(location.archetype))
            {
                // SAFE: we have already checked that the entity/component matches our archetype access. and systems are scheduled to run with safe archetype access
                unsafe {
                    self.world
                        .get_at_location_unchecked(location)
                        .map_err(QueryError::ComponentError)
                }
            } else {
                Err(QueryError::CannotReadArchetype)
            }
        } else {
            Err(QueryError::ComponentError(ComponentError::NoSuchEntity))
        }
    }

    /// Gets a mutable reference to the entity's component of the given type. This will fail if the entity does not have
    /// the given component type or if the given component type does not match this query.
    pub fn get_component_mut<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryError> {
        let location = match self.world.get_entity_location(entity) {
            None => return Err(QueryError::ComponentError(ComponentError::NoSuchEntity)),
            Some(location) => location,
        };

        if self
            .component_access
            .is_write(&ArchetypeComponent::new::<T>(location.archetype))
        {
            // SAFE: RefMut does exclusivity checks and we have already validated the entity
            unsafe {
                self.world
                    .get_mut_at_location_unchecked(location)
                    .map_err(QueryError::ComponentError)
            }
        } else {
            Err(QueryError::CannotWriteArchetype)
        }
    }

    /// Gets a mutable reference to the entity's component of the given type. This will fail if the entity does not have
    /// the given component type
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple mutable references to the same component
    pub unsafe fn get_component_unsafe<T: Component>(
        &self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryError>
    where
        Q::Fetch: for<'b> Fetch<'b, State = S>,
    {
        self.world
            .get_mut_unchecked(entity)
            .map_err(|_err| QueryError::NoSuchEntity)
    }

    pub fn removed<C: Component>(&self) -> &[Entity] {
        self.world.removed::<C>()
    }

    /// Sets the entity's component to the given value. This will fail if the entity does not already have
    /// the given component type or if the given component type does not match this query.
    pub fn set<T: Component>(&mut self, entity: Entity, component: T) -> Result<(), QueryError> {
        let mut current = self.get_component_mut::<T>(entity)?;
        *current = component;
        Ok(())
    }
}

/// An error that occurs when using a [Query]
#[derive(Debug)]
pub enum QueryError {
    CannotReadArchetype,
    CannotWriteArchetype,
    ComponentError(ComponentError),
    NoSuchEntity,
}

/// Parallel version of QueryIter
pub struct ParIter<'s, 'w, Q: HecsQuery, S> {
    batched_iter: BatchedIter<'s, 'w, Q, S>,
}

impl<'s, 'w, Q: HecsQuery, S> ParIter<'s, 'w, Q, S>
where
    Q::Fetch: for<'a> Fetch<'a, State = S>,
{
    pub fn new(batched_iter: BatchedIter<'s, 'w, Q, S>) -> Self {
        Self { batched_iter }
    }
}

unsafe impl<'s, 'w, Q: HecsQuery, S> Send for ParIter<'s, 'w, Q, S> where
    Q::Fetch: for<'a> Fetch<'a, State = S>
{
}

impl<'s, 'w, Q: HecsQuery, S> ParallelIterator<Batch<'s, 'w, Q, S>> for ParIter<'s, 'w, Q, S>
where
    Q::Fetch: for<'a> Fetch<'a, State = S>,
{
    type Item = <Q::Fetch as Fetch<'w>>::Item;

    #[inline]
    fn next_batch(&mut self) -> Option<Batch<'s, 'w, Q, S>> {
        self.batched_iter.next()
    }
}
