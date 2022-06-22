use crate::world::World;
use bevy_tasks::ComputeTaskPool;
use std::ops::Range;

use super::{Fetch, QueryFetch, QueryItem, QueryState, ROQueryFetch, ROQueryItem, WorldQuery};

#[derive(Clone)]
pub struct BatchingStrategy {
    pub batch_size_limits: Range<usize>,
    pub batches_per_thread: usize,
}

impl BatchingStrategy {
    pub const fn fixed(batch_size: usize) -> Self {
        Self {
            batch_size_limits: batch_size..batch_size,
            batches_per_thread: 1,
        }
    }

    pub const fn new() -> Self {
        Self {
            batch_size_limits: 0..usize::MAX,
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

pub struct QueryParIter<'w, 's, Q: WorldQuery, QF: Fetch<'w, State = Q::State>, F: WorldQuery> {
    pub(crate) world: &'w World,
    pub(crate) state: &'s QueryState<Q, F>,
    pub(crate) batching_strategy: BatchingStrategy,
    pub(crate) marker_: std::marker::PhantomData<fn() -> QF>,
}

impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery> QueryParIter<'w, 's, Q, QF, F>
where
    QF: Fetch<'w, State = Q::State>,
{
    pub fn batching_strategy(mut self, strategy: BatchingStrategy) -> Self {
        self.batching_strategy = strategy;
        self
    }

    /// Runs `func` on each query result in parallel.
    ///
    /// This can only be called for read-only queries, see [`Self::for_each_mut`] for
    /// write-queries.
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] resource must be added to the `World` before using this method. If using this from a query
    /// that is being initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[inline]
    pub fn for_each<FN: Fn(ROQueryItem<'w, Q>) + Send + Sync + Clone>(&self, func: FN) {
        // Need a batch size of at least 1.
        let batch_size = self.get_batch_size().max(1);
        // SAFETY: query is read only
        unsafe {
            self.state
                .par_for_each_unchecked_manual::<ROQueryFetch<Q>, FN>(
                    self.world,
                    batch_size,
                    func,
                    self.world.last_change_tick(),
                    self.world.read_change_tick(),
                );
        }
    }

    /// Runs `func` on each query result in parallel.
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] resource must be added to the `World` before using this method. If using this from a query
    /// that is being initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[inline]
    pub fn for_each_mut<FN: Fn(QueryItem<'w, Q>) + Send + Sync + Clone>(&mut self, func: FN) {
        // Need a batch size of at least 1.
        let batch_size = self.get_batch_size().max(1);
        // SAFETY: query has unique world access
        unsafe {
            self.state
                .par_for_each_unchecked_manual::<QueryFetch<Q>, FN>(
                    self.world,
                    batch_size,
                    func,
                    self.world.last_change_tick(),
                    self.world.read_change_tick(),
                );
        }
    }

    /// Runs `func` on each query result in parallel.
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] resource must be added to the `World` before using this method. If using this from a query
    /// that is being initialized and run from the ECS scheduler, this should never panic.
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
        // Need a batch size of at least 1.
        let batch_size = self.get_batch_size().max(1);
        self.state
            .par_for_each_unchecked_manual::<QueryFetch<Q>, FN>(
                self.world,
                batch_size,
                func,
                self.world.last_change_tick(),
                self.world.read_change_tick(),
            );
    }

    fn get_batch_size(&self) -> usize {
        if self.batching_strategy.batch_size_limits.is_empty() {
            return self.batching_strategy.batch_size_limits.start;
        }

        let thread_count = ComputeTaskPool::get().thread_num();
        assert!(
            thread_count > 0,
            "Attempted to run parallel iteration over a query with an empty TaskPool"
        );
        let max_size = if QF::IS_DENSE && <QueryFetch<'static, F>>::IS_DENSE {
            let tables = &self.world.storages().tables;
            self.state
                .matched_table_ids
                .iter()
                .map(|id| tables[*id].len())
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
        batch_size
            .min(self.batching_strategy.batch_size_limits.end)
            .max(self.batching_strategy.batch_size_limits.start)
    }
}
