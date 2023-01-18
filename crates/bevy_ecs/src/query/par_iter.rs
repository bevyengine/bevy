use crate::world::World;
use bevy_tasks::ComputeTaskPool;
use std::ops::Range;

use super::{QueryItem, QueryState, ROQueryItem, ReadOnlyWorldQuery, WorldQuery};

/// Dictates how a parallel query chunks up large tables/archetypes
/// during iteration.
///
/// A parallel query will chunk up large tables and archetypes into
/// chunks of at most a certain batch size.
///
/// By default, this batch size is automatically determined by dividing
/// the size of the largest matched archetype by the number
/// of threads. This attempts to minimize the overhead of scheduling
/// tasks onto multiple threads, but assumes each entity has roughly the
/// same amount of work to be done, which may not hold true in every
/// workload.
///
/// See [`Query::par_iter`] for more information.
///
/// [`Query::par_iter`]: crate::system::Query::par_iter
#[derive(Clone)]
pub struct BatchingStrategy {
    /// The upper and lower limits for how large a batch of entities.
    ///
    /// Setting the bounds to the same value will result in a fixed
    /// batch size.
    ///
    /// Defaults to `[1, usize::MAX]`.
    pub batch_size_limits: Range<usize>,
    /// The number of batches per thread in the [`ComputeTaskPool`].
    /// Increasing this value will decrease the batch size, which may
    /// increase the scheduling overhead for the iteration.
    ///
    /// Defaults to 1.
    pub batches_per_thread: usize,
}

impl BatchingStrategy {
    /// Creates a new unconstrained default batching strategy.
    pub const fn new() -> Self {
        Self {
            batch_size_limits: 1..usize::MAX,
            batches_per_thread: 1,
        }
    }

    /// Declares a batching strategy with a fixed batch size.
    pub const fn fixed(batch_size: usize) -> Self {
        Self {
            batch_size_limits: batch_size..batch_size,
            batches_per_thread: 1,
        }
    }

    pub const fn min_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size_limits.start = batch_size;
        self
    }

    pub const fn max_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size_limits.end = batch_size;
        self
    }

    pub fn batches_per_thread(mut self, batches_per_thread: usize) -> Self {
        assert!(
            batches_per_thread > 0,
            "The number of batches per thread must be non-zero."
        );
        self.batches_per_thread = batches_per_thread;
        self
    }
}

/// A parallel iterator over query results of a [`Query`](crate::system::Query).
///
/// This struct is created by the [`Query::par_iter`](crate::system::Query::iter) and
/// [`Query::par_iter_mut`](crate::system::Query::iter_mut) methods.
pub struct QueryParIter<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> {
    pub(crate) world: &'w World,
    pub(crate) state: &'s QueryState<Q, F>,
    pub(crate) batching_strategy: BatchingStrategy,
}

impl<'w, 's, Q: ReadOnlyWorldQuery, F: ReadOnlyWorldQuery> QueryParIter<'w, 's, Q, F> {
    /// Runs `func` on each query result in parallel.
    ///
    /// This can only be called for read-only queries, see [`Self::for_each_mut`] for
    /// write-queries.
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[inline]
    pub fn for_each<FN: Fn(ROQueryItem<'w, Q>) + Send + Sync + Clone>(&self, func: FN) {
        // SAFETY: query is read only
        unsafe {
            self.for_each_unchecked(func);
        }
    }
}

impl<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> QueryParIter<'w, 's, Q, F> {
    /// Changes the batching strategy used when iterating.
    ///
    /// For more information on how this affects the resultant iteration, see
    /// [`BatchingStrategy`].
    pub fn batching_strategy(mut self, strategy: BatchingStrategy) -> Self {
        self.batching_strategy = strategy;
        self
    }

    /// Runs `func` on each query result in parallel.
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[inline]
    pub fn for_each_mut<FN: Fn(QueryItem<'w, Q>) + Send + Sync + Clone>(&mut self, func: FN) {
        // SAFETY: query has unique world access
        unsafe {
            self.for_each_unchecked(func);
        }
    }

    /// Runs `func` on each query result in parallel.
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[inline]
    pub unsafe fn for_each_unchecked<FN: Fn(QueryItem<'w, Q>) + Send + Sync + Clone>(
        &self,
        func: FN,
    ) {
        let thread_count = ComputeTaskPool::get().thread_num();
        if thread_count <= 1 {
            self.state.for_each_unchecked_manual(
                self.world,
                func,
                self.world.last_change_tick(),
                self.world.read_change_tick(),
            );
        } else {
            // Need a batch size of at least 1.
            let batch_size = self.get_batch_size(thread_count).max(1);
            self.state.par_for_each_unchecked_manual(
                self.world,
                batch_size,
                func,
                self.world.last_change_tick(),
                self.world.read_change_tick(),
            );
        }
    }

    fn get_batch_size(&self, thread_count: usize) -> usize {
        if self.batching_strategy.batch_size_limits.is_empty() {
            return self.batching_strategy.batch_size_limits.start;
        }

        assert!(
            thread_count > 0,
            "Attempted to run parallel iteration over a query with an empty TaskPool"
        );
        let max_size = if Q::IS_DENSE && F::IS_DENSE {
            let tables = &self.world.storages().tables;
            self.state
                .matched_table_ids
                .iter()
                .map(|id| tables[*id].entity_count())
                .max()
                .unwrap_or(0)
        } else {
            let archetypes = &self.world.archetypes();
            self.state
                .matched_archetype_ids
                .iter()
                .map(|id| archetypes[*id].len())
                .max()
                .unwrap_or(0)
        };
        let batch_size = max_size / (thread_count * self.batching_strategy.batches_per_thread);
        batch_size.clamp(
            self.batching_strategy.batch_size_limits.start,
            self.batching_strategy.batch_size_limits.end,
        )
    }
}
