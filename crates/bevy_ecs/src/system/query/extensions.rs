use std::fmt::Debug;

use thiserror::Error;

use crate::{Fetch, Query, QueryFilter, ReadOnlyFetch, WorldQuery};

impl<'a, Q: WorldQuery, F: QueryFilter> Query<'a, Q, F> {
    /// Takes exactly one result from the query. If there no results, or more than 1 result, this will return an error instead.
    pub fn get_unique(&self) -> Result<<Q::Fetch as Fetch<'_>>::Item, OnlyQueryError<'_, Q>>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        let mut query = self.iter();
        let first = query.next();
        let extra_count = query.count();

        match (first, extra_count) {
            (Some(r), 0) => Ok(r),
            (None, _) => Err(OnlyQueryError::NoEntities(std::any::type_name::<Self>())),
            (Some(r), extra) => Err(OnlyQueryError::MultipleEntities(
                r,
                extra,
                std::any::type_name::<Self>(),
            )),
        }
    }

    /// See [`Query::get_unique`]
    pub fn get_unique_mut(
        &mut self,
    ) -> Result<<Q::Fetch as Fetch<'_>>::Item, OnlyQueryError<'_, Q>> {
        let mut query = self.iter_mut();
        let first = query.next();
        let extra_count = query.count();

        match (first, extra_count) {
            (Some(r), 0) => Ok(r),
            (None, _) => Err(OnlyQueryError::NoEntities(std::any::type_name::<Self>())),
            (Some(r), extra) => Err(OnlyQueryError::MultipleEntities(
                r,
                extra,
                std::any::type_name::<Self>(),
            )),
        }
    }
}

#[derive(Error)]
pub enum OnlyQueryError<'a, Q: WorldQuery> {
    #[error("No entities fit the query {0}")]
    NoEntities(&'static str),
    #[error("Multiple entities ({1} extra) fit the query {2}!")]
    MultipleEntities(<Q::Fetch as Fetch<'a>>::Item, usize, &'static str),
}

impl<'a, Q: WorldQuery> Debug for OnlyQueryError<'a, Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnlyQueryError::NoEntities(_) => f.debug_tuple("NoEntities").finish(),
            OnlyQueryError::MultipleEntities(_, _, _) => f.debug_tuple("MultipleEntities").finish(),
        }
    }
}
