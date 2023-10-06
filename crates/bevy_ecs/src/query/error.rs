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

/// An error that occurs when retrieving a specific [`Entity`]'s component from a [`Query`](crate::system::Query).
#[derive(Debug, PartialEq, Eq)]
pub enum QueryComponentError {
    /// The [`Query`](crate::system::Query) does not have read access to the requested component.
    ///
    /// This error occurs when the requested component is not included in the original query.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, query::QueryComponentError};
    /// #
    /// # #[derive(Component)]
    /// # struct OtherComponent;
    /// #
    /// # #[derive(Component, PartialEq, Debug)]
    /// # struct RequestedComponent;
    /// #
    /// # #[derive(Resource)]
    /// # struct SpecificEntity {
    /// #     entity: Entity,
    /// # }
    /// #
    /// fn get_missing_read_access_error(query: Query<&OtherComponent>, res: Res<SpecificEntity>) {
    ///     assert_eq!(
    ///         query.get_component::<RequestedComponent>(res.entity),
    ///         Err(QueryComponentError::MissingReadAccess),
    ///     );
    ///     println!("query doesn't have read access to RequestedComponent because it does not appear in Query<&OtherComponent>");
    /// }
    /// # bevy_ecs::system::assert_is_system(get_missing_read_access_error);
    /// ```
    MissingReadAccess,
    /// The [`Query`](crate::system::Query) does not have write access to the requested component.
    ///
    /// This error occurs when the requested component is not included in the original query, or the mutability of the requested component is mismatched with the original query.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, query::QueryComponentError};
    /// #
    /// # #[derive(Component, PartialEq, Debug)]
    /// # struct RequestedComponent;
    /// #
    /// # #[derive(Resource)]
    /// # struct SpecificEntity {
    /// #     entity: Entity,
    /// # }
    /// #
    /// fn get_missing_write_access_error(mut query: Query<&RequestedComponent>, res: Res<SpecificEntity>) {
    ///     assert_eq!(
    ///         query.get_component::<RequestedComponent>(res.entity),
    ///         Err(QueryComponentError::MissingWriteAccess),
    ///     );
    ///     println!("query doesn't have write access to RequestedComponent because it doesn't have &mut in Query<&RequestedComponent>");
    /// }
    /// # bevy_ecs::system::assert_is_system(get_missing_write_access_error);
    /// ```
    MissingWriteAccess,
    /// The given [`Entity`] does not have the requested component.
    MissingComponent,
    /// The requested [`Entity`] does not exist.
    NoSuchEntity,
}

impl std::error::Error for QueryComponentError {}

impl std::fmt::Display for QueryComponentError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QueryComponentError::MissingReadAccess => {
                write!(
                    f,
                    "This query does not have read access to the requested component."
                )
            }
            QueryComponentError::MissingWriteAccess => {
                write!(
                    f,
                    "This query does not have write access to the requested component."
                )
            }
            QueryComponentError::MissingComponent => {
                write!(f, "The given entity does not have the requested component.")
            }
            QueryComponentError::NoSuchEntity => {
                write!(f, "The requested entity does not exist.")
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
