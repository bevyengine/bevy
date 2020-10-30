mod query_set;

pub use query_set::*;

use bevy_hecs::{
    ArchetypeComponent, Batch, BatchedIter, Component, ComponentError, Entity, Fetch, Mut,
    Query as HecsQuery, QueryIter, ReadOnlyFetch, TypeAccess, World,
};
use bevy_tasks::ParallelIterator;
use std::marker::PhantomData;

/// Provides scoped access to a World according to a given [HecsQuery]
#[derive(Debug)]
pub struct Query<'a, Q: HecsQuery> {
    pub(crate) world: &'a World,
    pub(crate) component_access: &'a TypeAccess<ArchetypeComponent>,
    _marker: PhantomData<Q>,
}

/// An error that occurs when using a [Query]
#[derive(Debug)]
pub enum QueryError {
    CannotReadArchetype,
    CannotWriteArchetype,
    ComponentError(ComponentError),
    NoSuchEntity,
}

impl<'a, Q: HecsQuery> Query<'a, Q> {
    #[inline]
    pub fn new(world: &'a World, component_access: &'a TypeAccess<ArchetypeComponent>) -> Self {
        Self {
            world,
            component_access,
            _marker: PhantomData::default(),
        }
    }

    /// Iterates over the query results. This can only be called for read-only queries
    pub fn iter(&self) -> QueryIter<'_, Q>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe { self.world.query_unchecked() }
    }

    /// Iterates over the query results
    pub fn iter_mut(&mut self) -> QueryIter<'_, Q> {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe { self.world.query_unchecked() }
    }

    /// Iterates over the query results
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple mutable references to the same component
    pub unsafe fn iter_unsafe(&self) -> QueryIter<'_, Q> {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        self.world.query_unchecked()
    }

    #[inline]
    pub fn par_iter(&self, batch_size: usize) -> ParIter<'_, Q>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe { ParIter::new(self.world.query_batched_unchecked(batch_size)) }
    }

    #[inline]
    pub fn par_iter_mut(&mut self, batch_size: usize) -> ParIter<'_, Q> {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe { ParIter::new(self.world.query_batched_unchecked(batch_size)) }
    }

    /// Gets the query result for the given `entity`
    pub fn get(&self, entity: Entity) -> Result<<Q::Fetch as Fetch>::Item, QueryError>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.world
                .query_one_unchecked::<Q>(entity)
                .map_err(|_err| QueryError::NoSuchEntity)
        }
    }

    /// Gets the query result for the given `entity`
    pub fn get_mut(&mut self, entity: Entity) -> Result<<Q::Fetch as Fetch>::Item, QueryError> {
        // SAFE: system runs without conflicts with other systems. same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.world
                .query_one_unchecked::<Q>(entity)
                .map_err(|_err| QueryError::NoSuchEntity)
        }
    }

    /// Gets the query result for the given `entity`
    /// # Safety
    /// This allows aliased mutability. You must make sure this call does not result in multiple mutable references to the same component
    pub unsafe fn entity_unsafe(
        &self,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch>::Item, QueryError> {
        self.world
            .query_one_unchecked::<Q>(entity)
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
    pub unsafe fn get_unsafe<T: Component>(
        &self,
        entity: Entity,
    ) -> Result<Mut<'_, T>, QueryError> {
        self.world
            .get_mut_unchecked(entity)
            .map_err(QueryError::ComponentError)
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

/// Parallel version of QueryIter
pub struct ParIter<'w, Q: HecsQuery> {
    batched_iter: BatchedIter<'w, Q>,
}

impl<'w, Q: HecsQuery> ParIter<'w, Q> {
    pub fn new(batched_iter: BatchedIter<'w, Q>) -> Self {
        Self { batched_iter }
    }
}

unsafe impl<'w, Q: HecsQuery> Send for ParIter<'w, Q> {}

impl<'w, Q: HecsQuery> ParallelIterator<Batch<'w, Q>> for ParIter<'w, Q> {
    type Item = <Q::Fetch as Fetch<'w>>::Item;

    #[inline]
    fn next_batch(&mut self) -> Option<Batch<'w, Q>> {
        self.batched_iter.next()
    }
}
