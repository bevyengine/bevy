use bevy_utils::prelude::DebugName;
use thiserror::Error;

use crate::{
    archetype::ArchetypeId,
    entity::{Entity, EntityDoesNotExistError},
};

/// An error that occurs when retrieving a specific [`Entity`]'s query result from [`Query`](crate::system::Query) or [`QueryState`](crate::query::QueryState).
// TODO: return the type_name as part of this error
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryEntityError {
    /// The given [`Entity`]'s components do not match the query.
    ///
    /// Either it does not have a requested component, or it has a component which the query filters out.
    QueryDoesNotMatch(Entity, ArchetypeId),
    /// The given [`Entity`] does not exist.
    EntityDoesNotExist(EntityDoesNotExistError),
    /// The [`Entity`] was requested mutably more than once.
    ///
    /// See [`Query::get_many_mut`](crate::system::Query::get_many_mut) for an example.
    AliasedMutability(Entity),
}

impl From<EntityDoesNotExistError> for QueryEntityError {
    fn from(error: EntityDoesNotExistError) -> Self {
        QueryEntityError::EntityDoesNotExist(error)
    }
}

impl core::error::Error for QueryEntityError {}

impl core::fmt::Display for QueryEntityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::QueryDoesNotMatch(entity, _) => {
                write!(f, "The query does not match entity {entity}")
            }
            Self::EntityDoesNotExist(error) => {
                write!(f, "{error}")
            }
            Self::AliasedMutability(entity) => {
                write!(
                    f,
                    "The entity with ID {entity} was requested mutably more than once"
                )
            }
        }
    }
}

/// An error that occurs when evaluating a [`Query`](crate::system::Query) or [`QueryState`](crate::query::QueryState) as a single expected result via
/// [`single`](crate::system::Query::single) or [`single_mut`](crate::system::Query::single_mut).
#[derive(Debug, Error)]
pub enum QuerySingleError {
    /// No entity fits the query.
    #[error("No entities fit the query {0}")]
    NoEntities(DebugName),
    /// Multiple entities fit the query.
    #[error("Multiple entities fit the query {0}")]
    MultipleEntities(DebugName),
}

#[cfg(test)]
mod test {
    use crate::{prelude::World, query::QueryEntityError};
    use bevy_ecs_macros::Component;

    #[test]
    fn query_does_not_match() {
        let mut world = World::new();

        #[derive(Component)]
        struct Present1;
        #[derive(Component)]
        struct Present2;
        #[derive(Component, Debug, PartialEq)]
        struct NotPresent;

        let entity = world.spawn((Present1, Present2));

        let (entity, archetype_id) = (entity.id(), entity.archetype().id());

        let result = world.query::<&NotPresent>().get(&world, entity);

        assert_eq!(
            result,
            Err(QueryEntityError::QueryDoesNotMatch(entity, archetype_id))
        );
    }
}
