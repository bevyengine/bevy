use crate::world::World;

use super::{Fetch, QueryFetch, QueryItem, QueryState, ROQueryFetch, ROQueryItem, WorldQuery};

pub struct QueryParIter<'w, 's, Q: WorldQuery, QF: Fetch<'w, State = Q::State>, F: WorldQuery> {
    pub(crate) world: &'w World,
    pub(crate) state: &'s QueryState<Q, F>,
    pub(crate) batch_size: Option<usize>,
    pub(crate) marker_: std::marker::PhantomData<fn() -> QF>,
}

impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery> QueryParIter<'w, 's, Q, QF, F>
where
    QF: Fetch<'w, State = Q::State>,
{
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = Some(batch_size);
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
    pub fn for_each<FN: Fn(ROQueryItem<'w, Q>) + Send + Sync + Clone>(self, func: FN) {
        let batch_size = match self.batch_size.or_else(|| self.get_default_batch_size()) {
            Some(batch_size) => batch_size.max(1),
            None => return,
        };
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
    pub fn for_each_mut<FN: Fn(QueryItem<'w, Q>) + Send + Sync + Clone>(self, func: FN) {
        let batch_size = match self.batch_size.or_else(|| self.get_default_batch_size()) {
            Some(batch_size) => batch_size.max(1),
            None => return,
        };
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
        self,
        func: FN,
    ) {
        let batch_size = match self.batch_size.or_else(|| self.get_default_batch_size()) {
            Some(batch_size) => batch_size.max(1),
            None => return,
        };
        self.state
            .par_for_each_unchecked_manual::<QueryFetch<Q>, FN>(
                self.world,
                batch_size,
                func,
                self.world.last_change_tick(),
                self.world.read_change_tick(),
            );
    }

    fn get_default_batch_size(&self) -> Option<usize> {
        let thread_count = self
            .state
            .task_pool
            .as_ref()
            .map(|pool| pool.thread_num())
            .unwrap_or(0);
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
        } else {
            let archetypes = &self.world.archetypes();
            self.state
                .matched_archetype_ids
                .iter()
                .map(|id| archetypes[*id].len())
                .max()
        };
        max_size.map(|max| max / thread_count)
    }
}
