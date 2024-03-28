use crate::{
    batching::BatchingStrategy, component::Tick, world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{QueryData, QueryFilter, QueryItem, QueryState};

/// A parallel iterator over query results of a [`Query`](crate::system::Query).
///
/// This struct is created by the [`Query::par_iter`](crate::system::Query::par_iter) and
/// [`Query::par_iter_mut`](crate::system::Query::par_iter_mut) methods.
pub struct QueryParIter<'w, 's, D: QueryData, F: QueryFilter> {
    pub(crate) world: UnsafeWorldCell<'w>,
    pub(crate) state: &'s QueryState<D, F>,
    pub(crate) last_run: Tick,
    pub(crate) this_run: Tick,
    pub(crate) batching_strategy: BatchingStrategy,
}

impl<'w, 's, D: QueryData, F: QueryFilter> QueryParIter<'w, 's, D, F> {
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
    pub fn for_each<FN: Fn(QueryItem<'w, D>) + Send + Sync + Clone>(self, func: FN) {
        #[cfg(any(target_arch = "wasm32", not(feature = "multi-threaded")))]
        {
            // SAFETY:
            // This method can only be called once per instance of QueryParIter,
            // which ensures that mutable queries cannot be executed multiple times at once.
            // Mutable instances of QueryParIter can only be created via an exclusive borrow of a
            // Query or a World, which ensures that multiple aliasing QueryParIters cannot exist
            // at the same time.
            unsafe {
                self.state
                    .iter_unchecked_manual(self.world, self.last_run, self.this_run)
                    .for_each(func);
            }
        }
        #[cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded"))]
        {
            let thread_count = bevy_tasks::ComputeTaskPool::get().thread_num();
            if thread_count <= 1 {
                // SAFETY: See the safety comment above.
                unsafe {
                    self.state
                        .iter_unchecked_manual(self.world, self.last_run, self.this_run)
                        .for_each(func);
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

    #[cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded"))]
    fn get_batch_size(&self, thread_count: usize) -> usize {
        let max_items = || {
            if D::IS_DENSE && F::IS_DENSE {
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
            }
        };
        self.batching_strategy
            .calc_batch_size(max_items, thread_count)
    }
}
