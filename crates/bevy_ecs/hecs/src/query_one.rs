// modified by Bevy contributors

use core::marker::PhantomData;

use crate::{
    query::{Fetch, ReadOnlyFetch, With, Without},
    Archetype, Component, Query,
};

/// A borrow of a `World` sufficient to execute the query `Q` on a single entity
pub struct QueryOne<'a, Q: Query> {
    archetype: &'a Archetype,
    index: usize,
    _marker: PhantomData<Q>,
}

impl<'a, Q: Query> QueryOne<'a, Q> {
    /// Construct a query accessing the entity in `archetype` at `index`
    ///
    /// # Safety
    ///
    /// `index` must be in-bounds for `archetype`
    pub(crate) unsafe fn new(archetype: &'a Archetype, index: usize) -> Self {
        Self {
            archetype,
            index,
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
        unsafe {
            let mut fetch = Q::Fetch::get(self.archetype, self.index)?;
            if fetch.should_skip() {
                None
            } else {
                Some(fetch.next())
            }
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
    fn transform<R: Query>(self) -> QueryOne<'a, R> {
        QueryOne {
            archetype: self.archetype,
            index: self.index,
            _marker: PhantomData,
        }
    }
}

unsafe impl<Q: Query> Send for QueryOne<'_, Q> {}
unsafe impl<Q: Query> Sync for QueryOne<'_, Q> {}

/// A read only borrow of a `World` sufficient to execute the query `Q` on a single entity
pub struct ReadOnlyQueryOne<'a, Q: Query> {
    archetype: &'a Archetype,
    index: usize,
    _marker: PhantomData<Q>,
}

impl<'a, Q: Query> ReadOnlyQueryOne<'a, Q>
where
    Q::Fetch: ReadOnlyFetch,
{
    /// Construct a query accessing the entity in `archetype` at `index`
    ///
    /// # Safety
    ///
    /// `index` must be in-bounds for `archetype`
    pub(crate) unsafe fn new(archetype: &'a Archetype, index: usize) -> Self {
        Self {
            archetype,
            index,
            _marker: PhantomData,
        }
    }

    /// Get the query result, or `None` if the entity does not satisfy the query
    ///
    /// Must be called at most once.
    ///
    /// Panics if called more than once or if it would construct a borrow that clashes with another
    /// pre-existing borrow.
    pub fn get(&self) -> Option<<Q::Fetch as Fetch<'_>>::Item>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        unsafe {
            let mut fetch = Q::Fetch::get(self.archetype, self.index)?;
            if fetch.should_skip() {
                None
            } else {
                Some(fetch.next())
            }
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
    fn transform<R: Query>(self) -> QueryOne<'a, R> {
        QueryOne {
            archetype: self.archetype,
            index: self.index,
            _marker: PhantomData,
        }
    }
}

unsafe impl<Q: Query> Send for ReadOnlyQueryOne<'_, Q> {}
unsafe impl<Q: Query> Sync for ReadOnlyQueryOne<'_, Q> {}
