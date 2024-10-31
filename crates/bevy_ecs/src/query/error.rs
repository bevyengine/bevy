use thiserror::Error;

use crate::entity::Entity;

/// An error that occurs when retrieving a specific [`Entity`]'s query result from [`Query`](crate::system::Query) or [`QueryState`](crate::query::QueryState).
// TODO: return the type_name as part of this error
#[derive(Debug, PartialEq, Eq, Clone, Copy, Error)]
pub enum QueryEntityError {
    /// The given [`Entity`]'s components do not match the query.
    ///
    /// Either it does not have a requested component, or it has a component which the query filters out.
    #[error("The components of entity {0:?} do not match the query")]
    QueryDoesNotMatch(Entity),
    /// The given [`Entity`] does not exist.
    #[error("The entity {0:?} does not exist")]
    NoSuchEntity(Entity),
    /// The [`Entity`] was requested mutably more than once.
    ///
    /// See [`QueryState::get_many_mut`](crate::query::QueryState::get_many_mut) for an example.
    #[error("The entity {0:?} was requested mutably more than once")]
    AliasedMutability(Entity),
}

/// An error that occurs when evaluating a [`Query`](crate::system::Query) or [`QueryState`](crate::query::QueryState) as a single expected result via
/// [`get_single`](crate::system::Query::get_single) or [`get_single_mut`](crate::system::Query::get_single_mut).
#[derive(Debug, Error)]
pub enum QuerySingleError {
    /// No entity fits the query.
    #[error("No entities fit the query {0}")]
    NoEntities(&'static str),
    /// Multiple entities fit the query.
    #[error("Multiple entities fit the query {0}")]
    MultipleEntities(&'static str),
}
