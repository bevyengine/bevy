// modified by Bevy contributors 

use core::marker::PhantomData;

use crate::query::{Fetch, With, Without};
use crate::{Archetype, Component, Query};

/// A borrow of a `World` sufficient to execute the query `Q` on a single entity
pub struct QueryOne<'a, Q: Query> {
    archetype: &'a Archetype,
    index: u32,
    borrowed: bool,
    _marker: PhantomData<Q>,
}

impl<'a, Q: Query> QueryOne<'a, Q> {
    /// Construct a query accessing the entity in `archetype` at `index`
    ///
    /// # Safety
    ///
    /// `index` must be in-bounds for `archetype`
    pub(crate) unsafe fn new(archetype: &'a Archetype, index: u32) -> Self {
        Self { 
            archetype,
            index,
            borrowed: false,
            _marker: PhantomData,
        }
    }

    /// Get the query result, or `None` if the entity does not satisfy the query
    ///
    /// Must be called at most once.
    ///
    /// Panics if called more than once or if it would construct a borrow that clashes with another
    /// pre-existing borrow.
    pub fn get(&mut self) -> Option<<Q::Fetch as Fetch<'_>>::Item> {
        if self.borrowed {
            panic!("called QueryOnce::get twice; construct a new query instead");
        }
        unsafe {
            let mut fetch = Q::Fetch::get(self.archetype, self.index as usize)?;
            self.borrowed = true;
            Q::Fetch::borrow(self.archetype);
            Some(fetch.next())
        }
    }

    /// Transform the query into one that requires a certain component without borrowing it
    ///
    /// See `QueryBorrow::with` for details.
    pub fn with<T: Component>(self) -> QueryOne<'a, With<T, Q>> {
        self.transform()
    }

    /// Transform the query into one that skips entities having a certain component
    ///
    /// See `QueryBorrow::without` for details.
    pub fn without<T: Component>(self) -> QueryOne<'a, Without<T, Q>> {
        self.transform()
    }

    /// Helper to change the type of the query
    fn transform<R: Query>(mut self) -> QueryOne<'a, R> {
        let x = QueryOne {
            archetype: self.archetype,
            index: self.index,
            borrowed: self.borrowed,
            _marker: PhantomData,
        };
        // Ensure `Drop` won't fire redundantly
        self.borrowed = false;
        x
    }
}

impl<Q: Query> Drop for QueryOne<'_, Q> {
    fn drop(&mut self) {
        if self.borrowed {
            Q::Fetch::release(self.archetype);
        }
    }
}

unsafe impl<Q: Query> Send for QueryOne<'_, Q> {}
unsafe impl<Q: Query> Sync for QueryOne<'_, Q> {}
