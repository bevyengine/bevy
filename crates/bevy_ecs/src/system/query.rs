use crate::ArchetypeAccess;
use bevy_hecs::{
    Archetype, Component, ComponentError, Entity, Fetch, Query as HecsQuery, QueryOne, Ref, RefMut,
    World,
};
use bevy_tasks::ParallelIterator;
use std::marker::PhantomData;

/// Provides scoped access to a World according to a given [HecsQuery]
pub struct Query<'a, Q: HecsQuery> {
    pub(crate) world: &'a World,
    pub(crate) archetype_access: &'a ArchetypeAccess,
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
    pub fn new(world: &'a World, archetype_access: &'a ArchetypeAccess) -> Self {
        Self {
            world,
            archetype_access,
            _marker: PhantomData::default(),
        }
    }

    #[inline]
    pub fn iter(&mut self) -> QueryBorrow<'_, Q> {
        QueryBorrow::new(&self.world.archetypes, self.archetype_access)
    }

    /// Gets a reference to the entity's component of the given type. This will fail if the entity does not have
    /// the given component type or if the given component type does not match this query.
    pub fn get<T: Component>(&self, entity: Entity) -> Result<Ref<'_, T>, QueryError> {
        if let Some(location) = self.world.get_entity_location(entity) {
            if self
                .archetype_access
                .immutable
                .contains(location.archetype as usize)
                || self
                    .archetype_access
                    .mutable
                    .contains(location.archetype as usize)
            {
                self.world.get(entity).map_err(QueryError::ComponentError)
            } else {
                Err(QueryError::CannotReadArchetype)
            }
        } else {
            Err(QueryError::ComponentError(ComponentError::NoSuchEntity))
        }
    }

    pub fn entity(&self, entity: Entity) -> Result<QueryOne<'_, Q>, QueryError> {
        if let Some(location) = self.world.get_entity_location(entity) {
            if self
                .archetype_access
                .immutable
                .contains(location.archetype as usize)
                || self
                    .archetype_access
                    .mutable
                    .contains(location.archetype as usize)
            {
                Ok(self.world.query_one(entity).unwrap())
            } else {
                Err(QueryError::CannotReadArchetype)
            }
        } else {
            Err(QueryError::NoSuchEntity)
        }
    }

    /// Gets a mutable reference to the entity's component of the given type. This will fail if the entity does not have
    /// the given component type or if the given component type does not match this query.
    pub fn get_mut<T: Component>(&self, entity: Entity) -> Result<RefMut<'_, T>, QueryError> {
        let location = match self.world.get_entity_location(entity) {
            None => return Err(QueryError::ComponentError(ComponentError::NoSuchEntity)),
            Some(location) => location,
        };

        if self
            .archetype_access
            .mutable
            .contains(location.archetype as usize)
        {
            self.world
                .get_mut(entity)
                .map_err(QueryError::ComponentError)
        } else {
            Err(QueryError::CannotWriteArchetype)
        }
    }

    pub fn removed<C: Component>(&self) -> &[Entity] {
        self.world.removed::<C>()
    }

    /// Sets the entity's component to the given value. This will fail if the entity does not already have
    /// the given component type or if the given component type does not match this query.
    pub fn set<T: Component>(&self, entity: Entity, component: T) -> Result<(), QueryError> {
        let mut current = self.get_mut::<T>(entity)?;
        *current = component;
        Ok(())
    }
}

/// A borrow of a `World` sufficient to execute the query `Q`
///
/// Note that borrows are not released until this object is dropped.
pub struct QueryBorrow<'w, Q: HecsQuery> {
    archetypes: &'w [Archetype],
    archetype_access: &'w ArchetypeAccess,
    _marker: PhantomData<Q>,
}

impl<'w, Q: HecsQuery> QueryBorrow<'w, Q> {
    pub(crate) fn new(archetypes: &'w [Archetype], archetype_access: &'w ArchetypeAccess) -> Self {
        for index in archetype_access.immutable.ones() {
            Q::Fetch::borrow(&archetypes[index]);
        }

        for index in archetype_access.mutable.ones() {
            Q::Fetch::borrow(&archetypes[index]);
        }
        Self {
            archetypes,
            archetype_access,
            _marker: PhantomData,
        }
    }

    /// Execute the query
    ///
    /// Must be called only once per query.
    #[inline]
    pub fn iter<'q>(&'q mut self) -> QueryIter<'q, 'w, Q> {
        QueryIter {
            borrow: self,
            archetype_index: 0,
            iter: None,
        }
    }

    /// Like `iter`, but returns child iterators of at most `batch_size`
    /// elements
    ///
    /// Useful for distributing work over a threadpool using the
    /// ParallelIterator interface.
    ///
    /// Batch size needs to be chosen based on the task being done in
    /// parallel. The elements in each batch are computed serially, while
    /// the batches themselves are computed in parallel.
    ///
    /// A too small batch size can cause too much overhead, since scheduling
    /// each batch could take longer than running the batch. On the other
    /// hand, a too large batch size risks that one batch is still running
    /// long after the rest have finished.
    pub fn par_iter<'q>(&'q mut self, batch_size: u32) -> ParIter<'q, 'w, Q> {
        ParIter {
            borrow: self,
            archetype_index: 0,
            batch_size,
            batch: 0,
        }
    }
}

unsafe impl<'w, Q: HecsQuery> Send for QueryBorrow<'w, Q> {}
unsafe impl<'w, Q: HecsQuery> Sync for QueryBorrow<'w, Q> {}

impl<'w, Q: HecsQuery> Drop for QueryBorrow<'w, Q> {
    #[inline]
    fn drop(&mut self) {
        for index in self.archetype_access.immutable.ones() {
            Q::Fetch::release(&self.archetypes[index]);
        }

        for index in self.archetype_access.mutable.ones() {
            Q::Fetch::release(&self.archetypes[index]);
        }
    }
}

impl<'q, 'w, Q: HecsQuery> IntoIterator for &'q mut QueryBorrow<'w, Q> {
    type IntoIter = QueryIter<'q, 'w, Q>;
    type Item = <Q::Fetch as Fetch<'q>>::Item;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over the set of entities with the components in `Q`
pub struct QueryIter<'q, 'w, Q: HecsQuery> {
    borrow: &'q mut QueryBorrow<'w, Q>,
    archetype_index: u32,
    iter: Option<ChunkIter<Q>>,
}

unsafe impl<'q, 'w, Q: HecsQuery> Send for QueryIter<'q, 'w, Q> {}
unsafe impl<'q, 'w, Q: HecsQuery> Sync for QueryIter<'q, 'w, Q> {}

impl<'q, 'w, Q: HecsQuery> Iterator for QueryIter<'q, 'w, Q> {
    type Item = <Q::Fetch as Fetch<'q>>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iter {
                None => {
                    let archetype = self.borrow.archetypes.get(self.archetype_index as usize)?;
                    self.archetype_index += 1;
                    unsafe {
                        self.iter = Q::Fetch::get(archetype, 0).map(|fetch| ChunkIter {
                            fetch,
                            len: archetype.len(),
                        });
                    }
                }
                Some(ref mut iter) => match unsafe { iter.next() } {
                    None => {
                        self.iter = None;
                        continue;
                    }
                    Some(components) => {
                        return Some(components);
                    }
                },
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.len();
        (n, Some(n))
    }
}

impl<'q, 'w, Q: HecsQuery> ExactSizeIterator for QueryIter<'q, 'w, Q> {
    fn len(&self) -> usize {
        self.borrow
            .archetypes
            .iter()
            .filter(|&x| Q::Fetch::access(x).is_some())
            .map(|x| x.len() as usize)
            .sum()
    }
}

struct ChunkIter<Q: HecsQuery> {
    fetch: Q::Fetch,
    len: u32,
}

impl<Q: HecsQuery> ChunkIter<Q> {
    #[inline]
    unsafe fn next<'a>(&mut self) -> Option<<Q::Fetch as Fetch<'a>>::Item> {
        loop {
            if self.len == 0 {
                return None;
            }

            self.len -= 1;
            if self.fetch.should_skip() {
                // we still need to progress the iterator
                let _ = self.fetch.next();
                continue;
            }

            break Some(self.fetch.next());
        }
    }
}

/// Batched version of `QueryIter`
pub struct ParIter<'q, 'w, Q: HecsQuery> {
    borrow: &'q mut QueryBorrow<'w, Q>,
    archetype_index: u32,
    batch_size: u32,
    batch: u32,
}

impl<'q, 'w, Q: HecsQuery> ParallelIterator<Batch<'q, Q>> for ParIter<'q, 'w, Q> {
    type Item = <Q::Fetch as Fetch<'q>>::Item;

    fn next_batch(&mut self) -> Option<Batch<'q, Q>> {
        loop {
            let archetype = self.borrow.archetypes.get(self.archetype_index as usize)?;
            let offset = self.batch_size * self.batch;
            if offset >= archetype.len() {
                self.archetype_index += 1;
                self.batch = 0;
                continue;
            }
            if let Some(fetch) = unsafe { Q::Fetch::get(archetype, offset as usize) } {
                self.batch += 1;
                return Some(Batch {
                    _marker: PhantomData,
                    state: ChunkIter {
                        fetch,
                        len: self.batch_size.min(archetype.len() - offset),
                    },
                });
            } else {
                self.archetype_index += 1;
                debug_assert_eq!(
                    self.batch, 0,
                    "query fetch should always reject at the first batch or not at all"
                );
                continue;
            }
        }
    }
}

/// A sequence of entities yielded by `ParIter`
pub struct Batch<'q, Q: HecsQuery> {
    _marker: PhantomData<&'q ()>,
    state: ChunkIter<Q>,
}

impl<'q, 'w, Q: HecsQuery> Iterator for Batch<'q, Q> {
    type Item = <Q::Fetch as Fetch<'q>>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let components = unsafe { self.state.next()? };
        Some(components)
    }
}

unsafe impl<'q, Q: HecsQuery> Send for Batch<'q, Q> {}
