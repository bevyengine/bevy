use derive_more::derive::{Display, Error};

use crate::{entity::Entity, world::unsafe_world_cell::UnsafeWorldCell};

/// An error that occurs when retrieving a specific [`Entity`]'s query result from [`Query`](crate::system::Query) or [`QueryState`](crate::query::QueryState).
// TODO: return the type_name as part of this error
#[derive(Clone, Copy)]
pub enum QueryEntityError<'w> {
    /// The given [`Entity`]'s components do not match the query.
    ///
    /// Either it does not have a requested component, or it has a component which the query filters out.
    QueryDoesNotMatch(Entity, UnsafeWorldCell<'w>),
    /// The given [`Entity`] does not exist.
    NoSuchEntity(Entity),
    /// The [`Entity`] was requested mutably more than once.
    ///
    /// See [`QueryState::get_many_mut`](crate::query::QueryState::get_many_mut) for an example.
    AliasedMutability(Entity),
}

impl<'w> core::error::Error for QueryEntityError<'w> {}

impl<'w> core::fmt::Display for QueryEntityError<'w> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::QueryDoesNotMatch(entity, world) => {
                write!(
                    f,
                    "The query does not match the entity {entity}, which has components "
                )?;
                format_archetype(f, world, entity)
            }
            Self::NoSuchEntity(entity) => write!(f, "The entity {entity} does not exist"),
            Self::AliasedMutability(entity) => write!(
                f,
                "The entity {entity} was requested mutably more than once"
            ),
        }
    }
}

impl<'w> core::fmt::Debug for QueryEntityError<'w> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::QueryDoesNotMatch(entity, world) => {
                write!(f, "QueryDoesNotMatch({entity} with components ")?;
                format_archetype(f, world, entity)?;
                write!(f, ")")
            }
            Self::NoSuchEntity(entity) => write!(f, "NoSuchEntity({entity})"),
            Self::AliasedMutability(entity) => write!(f, "AliasedMutability({entity})"),
        }
    }
}

fn format_archetype(
    f: &mut core::fmt::Formatter<'_>,
    world: UnsafeWorldCell<'_>,
    entity: Entity,
) -> core::fmt::Result {
    // We know entity is still alive
    let entity = world
        .get_entity(entity)
        .expect("entity does not belong to world");
    for (i, component_id) in entity.archetype().components().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        let name = world
            .components()
            .get_name(component_id)
            .expect("entity does not belong to world");
        write!(f, "{name}")?;
    }
    Ok(())
}

impl<'w> PartialEq for QueryEntityError<'w> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::QueryDoesNotMatch(e1, _), Self::QueryDoesNotMatch(e2, _)) if e1 == e2 => true,
            (Self::NoSuchEntity(e1), Self::NoSuchEntity(e2)) if e1 == e2 => true,
            (Self::AliasedMutability(e1), Self::AliasedMutability(e2)) if e1 == e2 => true,
            _ => false,
        }
    }
}

impl<'w> Eq for QueryEntityError<'w> {}

/// An error that occurs when evaluating a [`Query`](crate::system::Query) or [`QueryState`](crate::query::QueryState) as a single expected result via
/// [`get_single`](crate::system::Query::get_single) or [`get_single_mut`](crate::system::Query::get_single_mut).
#[derive(Debug, Error, Display)]
pub enum QuerySingleError {
    /// No entity fits the query.
    #[display("No entities fit the query {_0}")]
    #[error(ignore)]
    NoEntities(&'static str),
    /// Multiple entities fit the query.
    #[display("Multiple entities fit the query {_0}")]
    #[error(ignore)]
    MultipleEntities(&'static str),
}

#[cfg(test)]
mod test {
    use crate as bevy_ecs;
    use crate::prelude::World;
    use bevy_ecs_macros::Component;

    #[test]
    fn query_does_not_match() {
        let mut world = World::new();

        #[derive(Component)]
        struct Present1;
        #[derive(Component)]
        struct Present2;
        #[derive(Component, Debug)]
        struct NotPresent;

        let entity = world.spawn((Present1, Present2)).id();

        let err = world
            .query::<&NotPresent>()
            .get(&world, entity)
            .unwrap_err();

        assert_eq!(format!("{err:?}"), "QueryDoesNotMatch(0v1 with components bevy_ecs::query::error::test::query_does_not_match::Present1, bevy_ecs::query::error::test::query_does_not_match::Present2)");
    }
}
