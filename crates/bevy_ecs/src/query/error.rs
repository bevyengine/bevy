use std::fmt;

use crate::entity::Entity;

/// An error that occurs when retrieving a specific [`Entity`]'s query result from [`Query`](crate::system::Query) or [`QueryState`](crate::query::QueryState).
// TODO: return the type_name as part of this error
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum QueryEntityError {
    /// The given [`Entity`]'s components do not match the query.
    ///
    /// Either it does not have a requested component, or it has a component which the query filters out.
    QueryDoesNotMatch(Entity),
    /// The given [`Entity`] does not exist.
    NoSuchEntity(Entity),
    /// The [`Entity`] was requested mutably more than once.
    ///
    /// See [`QueryState::get_many_mut`](crate::query::QueryState::get_many_mut) for an example.
    AliasedMutability(Entity),
}

impl std::error::Error for QueryEntityError {}

impl fmt::Display for QueryEntityError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QueryEntityError::QueryDoesNotMatch(_) => {
                write!(f, "The given entity's components do not match the query.")
            }
            QueryEntityError::NoSuchEntity(_) => write!(f, "The requested entity does not exist."),
            QueryEntityError::AliasedMutability(_) => {
                write!(f, "The entity was requested mutably more than once.")
            }
        }
    }
}

/// An error that occurs when evaluating a [`Query`](crate::system::Query) or [`QueryState`](crate::query::QueryState) as a single expected result via
/// [`get_single`](crate::system::Query::get_single) or [`get_single_mut`](crate::system::Query::get_single_mut).
#[derive(Debug)]
pub enum QuerySingleError {
    /// No entity fits the query.
    NoEntities(&'static str),
    /// Multiple entities fit the query.
    MultipleEntities(&'static str),
}

impl std::error::Error for QuerySingleError {}

impl std::fmt::Display for QuerySingleError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QuerySingleError::NoEntities(query) => write!(f, "No entities fit the query {query}"),
            QuerySingleError::MultipleEntities(query) => {
                write!(f, "Multiple entities fit the query {query}!")
            }
        }
    }
}
