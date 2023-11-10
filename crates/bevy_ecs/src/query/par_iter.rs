use crate::{component::Tick, world::unsafe_world_cell::UnsafeWorldCell};
use std::ops::Range;

use super::{QueryItem, QueryState, ReadOnlyWorldQuery, WorldQuery};

/// Dictates how a parallel query chunks up large tables/archetypes
/// during iteration.
///
/// A parallel query will chunk up large tables and archetypes into
/// chunks of at most a certain batch size.
///
/// By default, this batch size is automatically determined by dividing
/// the size of the largest matched archetype by the number
/// of threads (rounded up). This attempts to minimize the overhead of scheduling
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
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
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

    /// Configures the minimum allowed batch size of this instance.
    pub const fn min_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size_limits.start = batch_size;
        self
    }

    /// Configures the maximum allowed batch size of this instance.
    pub const fn max_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size_limits.end = batch_size;
        self
    }

    /// Configures the number of batches to assign to each thread for this instance.
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
/// This struct is created by the [`Query::par_iter`](crate::system::Query::par_iter) and
/// [`Query::par_iter_mut`](crate::system::Query::par_iter_mut) methods.
pub struct QueryParIter<'w, 's, Q: WorldQuery, F: ReadOnlyWorldQuery> {
    pub(crate) world: UnsafeWorldCell<'w>,
    pub(crate) state: &'s QueryState<Q, F>,
    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
    pub(crate) batching_strategy: BatchingStrategy,
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
    /// If the [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[inline]
    pub fn for_each<FN: Fn(QueryItem<'w, Q>) + Send + Sync + Clone>(self, func: FN) {
        #[cfg(any(target = "wasm32", not(feature = "multi-threaded")))]
        {
            // SAFETY:
            // This method can only be called once per instance of QueryParIter,
            // which ensures that mutable queries cannot be executed multiple times at once.
            // Mutable instances of QueryParIter can only be created via an exclusive borrow of a
            // Query or a World, which ensures that multiple aliasing QueryParIters cannot exist
            // at the same time.
            unsafe {
                self.state.for_each_unchecked_manual(
                    self.world,
                    func,
                    self.last_run,
                    self.this_run,
                );
            }
        }
        #[cfg(all(not(target = "wasm32"), feature = "multi-threaded"))]
        {
            let thread_count = bevy_tasks::ComputeTaskPool::get().thread_num();
            if thread_count <= 1 {
                // SAFETY: See the safety comment above.
                unsafe {
                    self.state.for_each_unchecked_manual(
                        self.world,
                        func,
                        self.last_run,
                        self.this_run,
                    );
                }
            } else {
                // Need a batch size of at least 1.
                let batch_size = self.get_batch_size(thread_count).max(1);
                // SAFETY: See the safety comment above.
                unsafe {
                    self.state.par_for_each_unchecked_manual(
                        self.world,
                        batch_size,
                        func,
                        self.last_run,
                        self.this_run,
                    );
                }
            }
        }
    }

    /// Runs `func` on each query result in parallel.
    ///
    /// # Panics
    /// If the [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[inline]
    #[deprecated = "use `.for_each(...)` instead."]
    pub fn for_each_mut<FN: Fn(QueryItem<'w, Q>) + Send + Sync + Clone>(self, func: FN) {
        self.for_each(func);
    }

    #[cfg(all(not(target = "wasm32"), feature = "multi-threaded"))]
    fn get_batch_size(&self, thread_count: usize) -> usize {
        if self.batching_strategy.batch_size_limits.is_empty() {
            return self.batching_strategy.batch_size_limits.start;
        }

        assert!(
            thread_count > 0,
            "Attempted to run parallel iteration over a query with an empty TaskPool"
        );
        let max_size = if Q::IS_DENSE && F::IS_DENSE {
            // SAFETY: We only access table metadata.
            let tables = unsafe { &self.world.world_metadata().storages().tables };
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

        let batches = thread_count * self.batching_strategy.batches_per_thread;
        // Round up to the nearest batch size.
        let batch_size = (max_size + batches - 1) / batches;
        batch_size.clamp(
            self.batching_strategy.batch_size_limits.start,
            self.batching_strategy.batch_size_limits.end,
        )
    }
}
