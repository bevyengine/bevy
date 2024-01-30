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

/// An error that occurs when retrieving a specific [`Entity`]'s component from a [`Query`](crate::system::Query).
#[derive(Debug, PartialEq, Eq, Error)]
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
    #[error("This query does not have read access to the requested component")]
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
    #[error("This query does not have write access to the requested component")]
    MissingWriteAccess,
    /// The given [`Entity`] does not have the requested component.
    #[error("The given entity does not have the requested component")]
    MissingComponent,
    /// The requested [`Entity`] does not exist.
    #[error("The requested entity does not exist")]
    NoSuchEntity,
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
