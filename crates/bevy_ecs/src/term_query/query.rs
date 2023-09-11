use std::marker::PhantomData;

use crate::{
    component::Tick,
    entity::Entity,
    query::{QueryEntityError, QuerySingleError},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{QueryTermGroup, ROTermItem, TermQueryIter, TermQueryState};

pub struct TermQuery<'w, 's, Q: QueryTermGroup, F: QueryTermGroup = ()> {
    world: UnsafeWorldCell<'w>,
    state: &'s TermQueryState<Q, F>,
    last_run: Tick,
    this_run: Tick,
    _marker: PhantomData<Q>,
}

impl<'w, 's, Q: QueryTermGroup, F: QueryTermGroup> TermQuery<'w, 's, Q, F> {
    pub fn new(
        world: UnsafeWorldCell<'w>,
        state: &'s TermQueryState<Q, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        state.validate_world(world.id());

        Self {
            world,
            state,
            last_run,
            this_run,
            _marker: PhantomData::default(),
        }
    }

    #[inline]
    pub fn iter(&self) -> TermQueryIter<'_, 's, Q::ReadOnly> {
        // SAFETY:
        // - `self.world` has permission to access the required components.
        // - The query is read-only, so it can be aliased even if it was originally mutable.
        unsafe {
            self.state
                .as_readonly()
                .iter_unchecked_manual(self.world, self.last_run, self.this_run)
        }
    }

    #[inline]
    pub fn iter_mut(&mut self) -> TermQueryIter<'_, 's, Q> {
        // SAFETY: `self.world` has permission to access the required components.
        unsafe {
            self.state
                .iter_unchecked_manual(self.world, self.last_run, self.this_run)
        }
    }

    #[inline]
    pub fn get(&self, entity: Entity) -> Result<ROTermItem<'_, Q>, QueryEntityError> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state.as_readonly().get_unchecked_manual(
                self.world,
                entity,
                self.last_run,
                self.this_run,
            )
        }
    }

    #[inline]
    pub fn get_mut(&mut self, entity: Entity) -> Result<Q::Item<'_>, QueryEntityError> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe {
            self.state
                .get_unchecked_manual(self.world, entity, self.last_run, self.this_run)
        }
    }

    pub fn single(&self) -> ROTermItem<'_, Q> {
        self.get_single().unwrap()
    }

    #[inline]
    pub fn get_single(&self) -> Result<ROTermItem<'_, Q>, QuerySingleError> {
        // SAFETY:
        // the query ensures that the components it accesses are not mutably accessible somewhere else
        // and the query is read only.
        unsafe {
            self.state.as_readonly().get_single_unchecked_manual(
                self.world,
                self.last_run,
                self.this_run,
            )
        }
    }

    pub fn single_mut(&mut self) -> Q::Item<'_> {
        self.get_single_mut().unwrap()
    }

    pub fn get_single_mut(&mut self) -> Result<Q::Item<'_>, QuerySingleError> {
        // SAFETY:
        // the query ensures mutable access to the components it accesses, and the query
        // is uniquely borrowed
        unsafe {
            self.state
                .get_single_unchecked_manual(self.world, self.last_run, self.this_run)
        }
    }
}

impl<'w, 's, Q: QueryTermGroup, F: QueryTermGroup> IntoIterator for &TermQuery<'w, 's, Q, F> {
    type Item = <Q::ReadOnly as QueryTermGroup>::Item<'w>;
    type IntoIter = TermQueryIter<'w, 's, Q::ReadOnly>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            TermQueryIter::new(
                self.world,
                self.state.as_readonly(),
                self.last_run,
                self.this_run,
            )
        }
    }
}

impl<'w, 's, Q: QueryTermGroup, F: QueryTermGroup> IntoIterator for &mut TermQuery<'w, 's, Q, F> {
    type Item = Q::Item<'w>;
    type IntoIter = TermQueryIter<'w, 's, Q>;

    fn into_iter(self) -> Self::IntoIter {
        unsafe {
            TermQueryIter::new(
                self.world,
                self.state.filterless(),
                self.last_run,
                self.this_run,
            )
        }
    }
}
