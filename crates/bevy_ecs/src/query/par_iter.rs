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
        self.for_each_init(|| {}, |_, item| func(item));
    }

    /// Runs `func` on each query result in parallel on a value returned by `init`.
    ///
    /// `init` may be called multiple times per thread, and the values returned may be discarded between tasks on any given thread.
    /// Callers should avoid using this function as if it were a parallel version
    /// of [`Iterator::fold`].
    ///
    /// # Example
    ///
    /// ```
    /// use bevy_utils::Parallel;
    /// use crate::{bevy_ecs::prelude::Component, bevy_ecs::system::Query};
    /// #[derive(Component)]
    /// struct T;
    /// fn system(query: Query<&T>){
    ///     let mut queue: Parallel<usize> = Parallel::default();
    ///     // queue.borrow_local_mut() will get or create a thread_local queue for each task/thread;
    ///     query.par_iter().for_each_init(|| queue.borrow_local_mut(),|local_queue,item| {
    ///         **local_queue += 1;
    ///      });
    ///     
    ///     // collect value from every thread
    ///     let entity_count: usize = queue.iter_mut().map(|v| *v).sum();
    /// }
    /// ```
    ///
    /// # Panics
    /// If the [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[inline]
    pub fn for_each_init<FN, INIT, T>(self, init: INIT, func: FN)
    where
        FN: Fn(&mut T, QueryItem<'w, D>) + Send + Sync + Clone,
        INIT: Fn() -> T + Sync + Send + Clone,
    {
        let func = |mut init, item| {
            func(&mut init, item);
            init
        };
        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        {
            let init = init();
            // SAFETY:
            // This method can only be called once per instance of QueryParIter,
            // which ensures that mutable queries cannot be executed multiple times at once.
            // Mutable instances of QueryParIter can only be created via an exclusive borrow of a
            // Query or a World, which ensures that multiple aliasing QueryParIters cannot exist
            // at the same time.
            unsafe {
                self.state
                    .iter_unchecked_manual(self.world, self.last_run, self.this_run)
                    .fold(init, func);
            }
        }
        #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
        {
            let thread_count = bevy_tasks::ComputeTaskPool::get().thread_num();
            if thread_count <= 1 {
                let init = init();
                // SAFETY: See the safety comment above.
                unsafe {
                    self.state
                        .iter_unchecked_manual(self.world, self.last_run, self.this_run)
                        .fold(init, func);
                }
            } else {
                // Need a batch size of at least 1.
                let batch_size = self.get_batch_size(thread_count).max(1);
                // SAFETY: See the safety comment above.
                unsafe {
                    self.state.par_fold_init_unchecked_manual(
                        init,
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

    #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
    fn get_batch_size(&self, thread_count: usize) -> usize {
        let max_items = || {
            let id_iter = self.state.matched_storage_ids.iter();
            if D::IS_DENSE && F::IS_DENSE {
                // SAFETY: We only access table metadata.
                let tables = unsafe { &self.world.world_metadata().storages().tables };
                id_iter
                    // SAFETY: The if check ensures that matched_storage_ids stores TableIds
                    .map(|id| unsafe { tables[id.table_id].entity_count() })
                    .max()
            } else {
                let archetypes = &self.world.archetypes();
                id_iter
                    // SAFETY: The if check ensures that matched_storage_ids stores ArchetypeIds
                    .map(|id| unsafe { archetypes[id.archetype_id].len() })
                    .max()
            }
            .unwrap_or(0)
        };
        self.batching_strategy
            .calc_batch_size(max_items, thread_count)
    }
}
