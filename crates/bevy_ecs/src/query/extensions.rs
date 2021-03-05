use std::fmt::Debug;

use thiserror::Error;

use crate::prelude::Query;

use super::{Fetch, FilterFetch, ReadOnlyFetch, WorldQuery};

impl<'w, Q: WorldQuery, F: WorldQuery> Query<'w, Q, F>
where
    F::Fetch: FilterFetch,
{
    /// Takes exactly one result from the query. If there are no results, or more than 1 result, this will return an error instead.
    pub fn get_unique(&self) -> Result<<Q::Fetch as Fetch<'_>>::Item, UniqueQueryError<'_, Q>>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        let mut query = self.iter();
        let first = query.next();
        let extra = query.next().is_some();

        match (first, extra) {
            (Some(r), false) => Ok(r),
            (None, _) => Err(UniqueQueryError::NoEntities(std::any::type_name::<Self>())),
            (Some(r), _) => Err(UniqueQueryError::MultipleEntities {
                result: r,
                query_name: std::any::type_name::<Self>(),
            }),
        }
    }

    /// See [`Query::get_unique`]
    pub fn get_unique_mut(
        &mut self,
    ) -> Result<<Q::Fetch as Fetch<'_>>::Item, UniqueQueryError<'_, Q>> {
        let mut query = self.iter_mut();
        let first = query.next();
        let extra = query.next().is_some();

        match (first, extra) {
            (Some(r), false) => Ok(r),
            (None, _) => Err(UniqueQueryError::NoEntities(std::any::type_name::<Self>())),
            (Some(r), _) => Err(UniqueQueryError::MultipleEntities {
                result: r,
                query_name: std::any::type_name::<Self>(),
            }),
        }
    }
}

#[derive(Error)]
pub enum UniqueQueryError<'a, Q: WorldQuery> {
    #[error("No entities fit the query {0}")]
    NoEntities(&'static str),
    #[error("Multiple entities fit the query {query_name}!")]
    MultipleEntities {
        result: <Q::Fetch as Fetch<'a>>::Item,
        query_name: &'static str,
    },
}

impl<'a, Q: WorldQuery> Debug for UniqueQueryError<'a, Q> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEntities(_) => f.debug_tuple("NoEntities").finish(),
            Self::MultipleEntities { .. } => f.debug_tuple("MultipleEntities").finish(),
        }
    }
}
