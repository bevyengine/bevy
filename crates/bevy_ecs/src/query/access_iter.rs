use core::fmt::Display;

use crate::{
    component::{ComponentId, Components},
    query::{Access, QueryData},
};
use bevy_utils::BloomFilter;

// found by benchmarking
// too low, and smaller queries do unnecessary work
// maintaining the bloom filter for a handful of checks
// too high, and the benefit of a simpler loop
// is outweighed by the n^2 check
const USE_FILTER_THRESHOLD: usize = 4;

/// Check if `Q` has any internal conflicts.
#[inline(never)]
pub fn has_conflicts<Q: QueryData>(components: &Components) -> Result<(), QueryAccessError> {
    let Some(state) = Q::get_state(components) else {
        return Err(QueryAccessError::ComponentNotRegistered);
    };
    let size = Q::iter_access(&state)
        .size_hint()
        .1
        .unwrap_or(USE_FILTER_THRESHOLD);

    let result = if size > USE_FILTER_THRESHOLD {
        has_conflicts_large::<Q>(&state)
    } else {
        has_conflicts_small::<Q>(&state)
    };
    if let Err(e) = result {
        panic!("{e}");
    }

    Ok(())
}

/// Check if `Q` has any internal conflicts by checking all pairs of accesses.
///
/// This is intended for queries with fewer components than [`USE_FILTER_THRESHOLD`].
/// Split from [`has_conflicts`] for easier testing.
fn has_conflicts_small<'a, Q: QueryData>(
    state: &'a Q::State,
) -> Result<(), AccessConflictError<'a>> {
    // we can optimize small sizes by caching the iteration result in an array on the stack
    let mut inner_access = [EcsAccessType::Empty; USE_FILTER_THRESHOLD];
    for (i, access) in Q::iter_access(state).enumerate() {
        for access_other in inner_access.iter().take(i) {
            if access.is_compatible(*access_other).is_err() {
                return Err(AccessConflictError(access, *access_other));
            }
        }
        if i < USE_FILTER_THRESHOLD {
            inner_access[i] = access;
        } else {
            for access_other in Q::iter_access(state).take(i) {
                if access.is_compatible(access_other).is_err() {
                    return Err(AccessConflictError(access, access_other));
                }
            }
        }
    }

    Ok(())
}

/// Check if `Q` has any internal conflicts using a bloom filter for efficiency.
///
/// This is intended for queries with more components than [`USE_FILTER_THRESHOLD`].
/// Split from [`has_conflicts`] for easier testing.
fn has_conflicts_large<'a, Q: QueryData>(
    state: &'a Q::State,
) -> Result<(), AccessConflictError<'a>> {
    // use a bloom filter as a linear time check if we need to run the longer, exact check
    let mut filter = BloomFilter::<8, 2>::new();
    for (i, access) in Q::iter_access(state).enumerate() {
        let needs_check = match access {
            EcsAccessType::Component(EcsAccessLevel::Read(component_id))
            | EcsAccessType::Component(EcsAccessLevel::Write(component_id)) => {
                filter.check_insert(&component_id.index())
            }
            EcsAccessType::Component(EcsAccessLevel::ReadAll)
            | EcsAccessType::Component(EcsAccessLevel::WriteAll) => true,
            EcsAccessType::Resource(ResourceAccessLevel::Read(resource_id))
            | EcsAccessType::Resource(ResourceAccessLevel::Write(resource_id)) => {
                filter.check_insert(&resource_id.index())
            }
            EcsAccessType::Access(access) => {
                if access.has_read_all_resources() || access.has_write_all_resources() {
                    true
                } else if let Ok(component_iter) = access.try_iter_component_access() {
                    let mut needs_check = false;
                    for kind in component_iter {
                        let index = match kind {
                            crate::query::ComponentAccessKind::Shared(id)
                            | crate::query::ComponentAccessKind::Exclusive(id)
                            | crate::query::ComponentAccessKind::Archetypal(id) => id.index(),
                        };
                        if filter.check_insert(&index) {
                            needs_check = true;
                        }
                    }
                    for resource_id in access.resource_reads_and_writes() {
                        if filter.check_insert(&resource_id.index()) {
                            needs_check = true;
                        }
                    }
                    needs_check
                } else {
                    true
                }
            }
            EcsAccessType::Empty => continue,
        };
        if needs_check {
            // we MIGHT have a conflict, fallback to slow check
            for (j, access_other) in Q::iter_access(state).enumerate() {
                if i == j {
                    continue;
                }
                if access.is_compatible(access_other).is_err() {
                    return Err(AccessConflictError(access, access_other));
                }
            }
        }
    }
    Ok(())
}

/// The data storage type that is being accessed.
#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub enum EcsAccessType<'a> {
    /// Accesses [`Component`](crate::prelude::Component) data
    Component(EcsAccessLevel),
    /// Accesses [`Resource`](crate::prelude::Resource) data
    Resource(ResourceAccessLevel),
    /// borrowed access from [`WorldQuery::State`](crate::query::WorldQuery)
    Access(&'a Access),
    /// Does not access any data that can conflict.
    Empty,
}

impl<'a> EcsAccessType<'a> {
    /// Returns `Ok(())` if `self` and `other` are compatible. Returns a [`AccessConflictError`] otherwise.
    #[inline(never)]
    pub fn is_compatible(&self, other: Self) -> Result<(), AccessConflictError<'_>> {
        use EcsAccessLevel::*;
        use EcsAccessType::*;

        match (*self, other) {
            (Component(ReadAll), Component(Write(_)))
            | (Component(Write(_)), Component(ReadAll))
            | (Component(_), Component(WriteAll))
            | (Component(WriteAll), Component(_)) => Err(AccessConflictError(*self, other)),

            (Empty, _)
            | (_, Empty)
            | (Component(_), Resource(_))
            | (Resource(_), Component(_))
            // read only access doesn't conflict
            | (Component(Read(_)), Component(Read(_)))
            | (Component(ReadAll), Component(Read(_)))
            | (Component(Read(_)), Component(ReadAll))
            | (Component(ReadAll), Component(ReadAll))
            | (Resource(ResourceAccessLevel::Read(_)), Resource(ResourceAccessLevel::Read(_))) => {
                Ok(())
            }

            (Component(Read(id)), Component(Write(id_other)))
            | (Component(Write(id)), Component(Read(id_other)))
            | (Component(Write(id)), Component(Write(id_other)))
            | (
                Resource(ResourceAccessLevel::Read(id)),
                Resource(ResourceAccessLevel::Write(id_other)),
            )
            | (
                Resource(ResourceAccessLevel::Write(id)),
                Resource(ResourceAccessLevel::Read(id_other)),
            )
            | (
                Resource(ResourceAccessLevel::Write(id)),
                Resource(ResourceAccessLevel::Write(id_other)),
            ) => if id == id_other {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },

            // Borrowed Access
            (Component(Read(component_id)), Access(access))
            | (Access(access), Component(Read(component_id))) => if access.has_component_write(component_id) {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },

            (Component(Write(component_id)), Access(access))
            | (Access(access), Component(Write(component_id))) => if access.has_component_read(component_id) {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },

            (Component(ReadAll), Access(access))
            | (Access(access), Component(ReadAll)) => if access.has_any_component_write() {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },

            (Component(WriteAll), Access(access))
            | (Access(access), Component(WriteAll))=> if access.has_any_component_read() {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },

            (Resource(ResourceAccessLevel::Read(component_id)), Access(access))
            | (Access(access), Resource(ResourceAccessLevel::Read(component_id))) => if access.has_resource_write(component_id) {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },
            (Resource(ResourceAccessLevel::Write(component_id)), Access(access))
            | (Access(access), Resource(ResourceAccessLevel::Write(component_id))) => if access.has_resource_read(component_id) {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },

            (Access(access), Access(other_access)) => if access.is_compatible(other_access) {
                Ok(())
            } else {
                Err(AccessConflictError(*self, other))
            },
        }
    }
}

/// The way the data will be accessed and whether we take access on all the components on
/// an entity or just one component.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub enum EcsAccessLevel {
    /// Reads [`Component`](crate::prelude::Component) with [`ComponentId`]
    Read(ComponentId),
    /// Writes [`Component`](crate::prelude::Component) with [`ComponentId`]
    Write(ComponentId),
    /// Potentially reads all [`Component`](crate::prelude::Component)'s in the [`World`](crate::prelude::World)
    ReadAll,
    /// Potentially writes all [`Component`](crate::prelude::Component)'s in the [`World`](crate::prelude::World)
    WriteAll,
}

/// Access level needed by [`QueryData`] fetch to the resource.
#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub enum ResourceAccessLevel {
    /// Reads the resource with [`ComponentId`]
    Read(ComponentId),
    /// Writes the resource with [`ComponentId`]
    Write(ComponentId),
}

/// Error returned from [`EcsAccessType::is_compatible`]
pub struct AccessConflictError<'a>(EcsAccessType<'a>, EcsAccessType<'a>);

impl Display for AccessConflictError<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use EcsAccessLevel::*;
        use EcsAccessType::*;

        let AccessConflictError(a, b) = self;
        match (a, b) {
            // ReadAll/WriteAll + Component conflicts
            (Component(ReadAll), Component(Write(id)))
            | (Component(Write(id)), Component(ReadAll)) => {
                write!(
                    f,
                    "Component read all access conflicts with component {id:?} write."
                )
            }
            (Component(WriteAll), Component(Write(id)))
            | (Component(Write(id)), Component(WriteAll)) => {
                write!(
                    f,
                    "Component write all access conflicts with component {id:?} write."
                )
            }
            (Component(WriteAll), Component(Read(id)))
            | (Component(Read(id)), Component(WriteAll)) => {
                write!(
                    f,
                    "Component write all access conflicts with component {id:?} read."
                )
            }
            (Component(WriteAll), Component(ReadAll))
            | (Component(ReadAll), Component(WriteAll)) => {
                write!(f, "Component write all conflicts with component read all.")
            }
            (Component(WriteAll), Component(WriteAll)) => {
                write!(f, "Component write all conflicts with component write all.")
            }

            // Component + Component conflicts
            (Component(Read(id)), Component(Write(id_other)))
            | (Component(Write(id_other)), Component(Read(id))) => write!(
                f,
                "Component {id:?} read conflicts with component {id_other:?} write."
            ),
            (Component(Write(id)), Component(Write(id_other))) => write!(
                f,
                "Component {id:?} write conflicts with component {id_other:?} write."
            ),

            // Borrowed Access conflicts
            (Access(_), Component(Read(id))) | (Component(Read(id)), Access(_)) => write!(
                f,
                "Access has a write that conflicts with component {id:?} read."
            ),
            (Access(_), Component(Write(id))) | (Component(Write(id)), Access(_)) => write!(
                f,
                "Access has a read that conflicts with component {id:?} write."
            ),
            (Access(_), Component(ReadAll)) | (Component(ReadAll), Access(_)) => write!(
                f,
                "Access has a write that conflicts with component read all"
            ),
            (Access(_), Component(WriteAll)) | (Component(WriteAll), Access(_)) => write!(
                f,
                "Access has a read that conflicts with component write all"
            ),
            (Access(_), Resource(ResourceAccessLevel::Read(id)))
            | (Resource(ResourceAccessLevel::Read(id)), Access(_)) => write!(
                f,
                "Access has a write that conflicts with resource {id:?} read."
            ),
            (Access(_), Resource(ResourceAccessLevel::Write(id)))
            | (Resource(ResourceAccessLevel::Write(id)), Access(_)) => write!(
                f,
                "Access has a read that conflicts with resource {id:?} write."
            ),
            (Access(_), Access(_)) => write!(f, "Access conflicts with other Access"),

            _ => {
                unreachable!("Other accesses should be compatible");
            }
        }
    }
}

/// Error returned from [`has_conflicts`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QueryAccessError {
    /// Component was not registered on world
    ComponentNotRegistered,
    /// Entity did not have the requested components
    EntityDoesNotMatch,
}

impl core::error::Error for QueryAccessError {}

impl Display for QueryAccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            QueryAccessError::ComponentNotRegistered => {
                write!(
                    f,
                    "At least one component in Q was not registered in world. 
                    Consider calling `World::register_component`"
                )
            }
            QueryAccessError::EntityDoesNotMatch => {
                write!(f, "Entity does not match Q")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        prelude::Component,
        query::WorldQuery,
        world::{EntityMut, EntityMutExcept, EntityRef, EntityRefExcept, World},
    };

    #[derive(Component)]
    struct C1;

    #[derive(Component)]
    struct C2;

    fn setup_world() -> World {
        let world = World::new();
        let mut world = world;
        world.register_component::<C1>();
        world.register_component::<C2>();
        world
    }

    #[test]
    fn simple_compatible() {
        let world = setup_world();
        let c = world.components();

        // Compatible
        let state = <&mut C1 as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<&mut C1>(&state).is_ok());
        assert!(has_conflicts_large::<&mut C1>(&state).is_ok());
        assert!(has_conflicts::<&mut C1>(c).is_ok());

        let state = <&C1 as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<&C1>(&state).is_ok());
        assert!(has_conflicts_large::<&C1>(&state).is_ok());
        assert!(has_conflicts::<&C1>(c).is_ok());

        let state = <(&C1, &C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&C1, &C1)>(&state).is_ok());
        assert!(has_conflicts_large::<(&C1, &C1)>(&state).is_ok());
        assert!(has_conflicts::<(&C1, &C1)>(c).is_ok());
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn conflict_component_read_conflicts_write() {
        let world = setup_world();
        let c = world.components();
        let state = <(&C1, &mut C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&C1, &mut C1)>(&state).is_err());
        assert!(has_conflicts_large::<(&C1, &mut C1)>(&state).is_err());
        let _ = has_conflicts::<(&C1, &mut C1)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn conflict_component_write_conflicts_read() {
        let world = setup_world();
        let c = world.components();
        let state = <(&mut C1, &C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&mut C1, &C1)>(&state).is_err());
        assert!(has_conflicts_large::<(&mut C1, &C1)>(&state).is_err());
        let _ = has_conflicts::<(&mut C1, &C1)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn conflict_component_write_conflicts_write() {
        let world = setup_world();
        let c = world.components();
        let state = <(&mut C1, &mut C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&mut C1, &mut C1)>(&state).is_err());
        assert!(has_conflicts_large::<(&mut C1, &mut C1)>(&state).is_err());
        let _ = has_conflicts::<(&mut C1, &mut C1)>(c);
    }

    #[test]
    fn entity_ref_compatible() {
        let world = setup_world();
        let c = world.components();

        // Compatible
        let state = <(EntityRef, &C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityRef, &C1)>(&state).is_ok());
        assert!(has_conflicts_large::<(EntityRef, &C1)>(&state).is_ok());
        assert!(has_conflicts::<(EntityRef, &C1)>(c).is_ok());

        let state = <(&C1, EntityRef) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&C1, EntityRef)>(&state).is_ok());
        assert!(has_conflicts_large::<(&C1, EntityRef)>(&state).is_ok());
        assert!(has_conflicts::<(&C1, EntityRef)>(c).is_ok());

        let state = <(EntityRef, EntityRef) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityRef, EntityRef)>(&state).is_ok());
        assert!(has_conflicts_large::<(EntityRef, EntityRef)>(&state).is_ok());
        assert!(has_conflicts::<(EntityRef, EntityRef)>(c).is_ok());
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_ref_conflicts_component_write() {
        let world = setup_world();
        let c = world.components();
        let state = <(EntityRef, &mut C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityRef, &mut C1)>(&state).is_err());
        assert!(has_conflicts_large::<(EntityRef, &mut C1)>(&state).is_err());
        let _ = has_conflicts::<(EntityRef, &mut C1)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_write_conflicts_entity_ref() {
        let world = setup_world();
        let c = world.components();
        let state = <(&mut C1, EntityRef) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&mut C1, EntityRef)>(&state).is_err());
        assert!(has_conflicts_large::<(&mut C1, EntityRef)>(&state).is_err());
        let _ = has_conflicts::<(&mut C1, EntityRef)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_mut_conflicts_component_read() {
        let world = setup_world();
        let c = world.components();
        let state = <(EntityMut, &C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityMut, &C1)>(&state).is_err());
        assert!(has_conflicts_large::<(EntityMut, &C1)>(&state).is_err());
        let _ = has_conflicts::<(EntityMut, &C1)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_read_conflicts_entity_mut() {
        let world = setup_world();
        let c = world.components();
        let state = <(&C1, EntityMut) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&C1, EntityMut)>(&state).is_err());
        assert!(has_conflicts_large::<(&C1, EntityMut)>(&state).is_err());
        let _ = has_conflicts::<(&C1, EntityMut)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_mut_conflicts_component_write() {
        let world = setup_world();
        let c = world.components();
        let state = <(EntityMut, &mut C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityMut, &mut C1)>(&state).is_err());
        assert!(has_conflicts_large::<(EntityMut, &mut C1)>(&state).is_err());
        let _ = has_conflicts::<(EntityMut, &mut C1)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_write_conflicts_entity_mut() {
        let world = setup_world();
        let c = world.components();
        let state = <(&mut C1, EntityMut) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&mut C1, EntityMut)>(&state).is_err());
        assert!(has_conflicts_large::<(&mut C1, EntityMut)>(&state).is_err());
        let _ = has_conflicts::<(&mut C1, EntityMut)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_mut_conflicts_entity_ref() {
        let world = setup_world();
        let c = world.components();
        let state = <(EntityMut, EntityRef) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityMut, EntityRef)>(&state).is_err());
        assert!(has_conflicts_large::<(EntityMut, EntityRef)>(&state).is_err());
        let _ = has_conflicts::<(EntityMut, EntityRef)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_ref_conflicts_entity_mut() {
        let world = setup_world();
        let c = world.components();
        let state = <(EntityRef, EntityMut) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityRef, EntityMut)>(&state).is_err());
        assert!(has_conflicts_large::<(EntityRef, EntityMut)>(&state).is_err());
        let _ = has_conflicts::<(EntityRef, EntityMut)>(c);
    }

    #[test]
    fn entity_ref_except_compatible() {
        let world = setup_world();
        let c = world.components();

        // Compatible
        let state = <(EntityRefExcept<C1>, &mut C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityRefExcept<C1>, &mut C1)>(&state).is_ok());
        assert!(has_conflicts_large::<(EntityRefExcept<C1>, &mut C1)>(&state).is_ok());
        assert!(has_conflicts::<(EntityRefExcept<C1>, &mut C1)>(c).is_ok());

        let state = <(&mut C1, EntityRefExcept<C1>) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&mut C1, EntityRefExcept<C1>)>(&state).is_ok());
        assert!(has_conflicts_large::<(&mut C1, EntityRefExcept<C1>)>(&state).is_ok());
        assert!(has_conflicts::<(&mut C1, EntityRefExcept<C1>)>(c).is_ok());

        let state = <(&C2, EntityRefExcept<C1>) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&C2, EntityRefExcept<C1>)>(&state).is_ok());
        assert!(has_conflicts_large::<(&C2, EntityRefExcept<C1>)>(&state).is_ok());
        assert!(has_conflicts::<(&C2, EntityRefExcept<C1>)>(c).is_ok());

        let state = <(&mut C1, EntityRefExcept<(C1, C2)>) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&mut C1, EntityRefExcept<(C1, C2)>)>(&state).is_ok());
        assert!(has_conflicts_large::<(&mut C1, EntityRefExcept<(C1, C2)>)>(&state).is_ok());
        assert!(has_conflicts::<(&mut C1, EntityRefExcept<(C1, C2)>)>(c).is_ok());

        let state = <(EntityRefExcept<(C1, C2)>, &mut C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityRefExcept<(C1, C2)>, &mut C1)>(&state).is_ok());
        assert!(has_conflicts_large::<(EntityRefExcept<(C1, C2)>, &mut C1)>(&state).is_ok());
        assert!(has_conflicts::<(EntityRefExcept<(C1, C2)>, &mut C1)>(c).is_ok());

        let state =
            <(&mut C1, &mut C2, EntityRefExcept<(C1, C2)>) as WorldQuery>::get_state(c).unwrap();
        assert!(
            has_conflicts_small::<(&mut C1, &mut C2, EntityRefExcept<(C1, C2)>)>(&state).is_ok()
        );
        assert!(
            has_conflicts_large::<(&mut C1, &mut C2, EntityRefExcept<(C1, C2)>)>(&state).is_ok()
        );
        assert!(has_conflicts::<(&mut C1, &mut C2, EntityRefExcept<(C1, C2)>)>(c).is_ok());

        let state =
            <(&mut C1, EntityRefExcept<(C1, C2)>, &mut C2) as WorldQuery>::get_state(c).unwrap();
        assert!(
            has_conflicts_small::<(&mut C1, EntityRefExcept<(C1, C2)>, &mut C2)>(&state).is_ok()
        );
        assert!(
            has_conflicts_large::<(&mut C1, EntityRefExcept<(C1, C2)>, &mut C2)>(&state).is_ok()
        );
        assert!(has_conflicts::<(&mut C1, EntityRefExcept<(C1, C2)>, &mut C2)>(c).is_ok());

        let state =
            <(EntityRefExcept<(C1, C2)>, &mut C1, &mut C2) as WorldQuery>::get_state(c).unwrap();
        assert!(
            has_conflicts_small::<(EntityRefExcept<(C1, C2)>, &mut C1, &mut C2)>(&state).is_ok()
        );
        assert!(
            has_conflicts_large::<(EntityRefExcept<(C1, C2)>, &mut C1, &mut C2)>(&state).is_ok()
        );
        assert!(has_conflicts::<(EntityRefExcept<(C1, C2)>, &mut C1, &mut C2)>(c).is_ok());
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_ref_except_conflicts_component_write() {
        let world = setup_world();
        let c = world.components();
        let state = <(EntityRefExcept<C1>, &mut C2) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityRefExcept<C1>, &mut C2)>(&state).is_err());
        assert!(has_conflicts_large::<(EntityRefExcept<C1>, &mut C2)>(&state).is_err());
        let _ = has_conflicts::<(EntityRefExcept<C1>, &mut C2)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_write_conflicts_entity_ref_except() {
        let world = setup_world();
        let c = world.components();
        let state = <(&mut C2, EntityRefExcept<C1>) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&mut C2, EntityRefExcept<C1>)>(&state).is_err());
        assert!(has_conflicts_large::<(&mut C2, EntityRefExcept<C1>)>(&state).is_err());
        let _ = has_conflicts::<(&mut C2, EntityRefExcept<C1>)>(c);
    }

    #[test]
    fn entity_mut_except_compatible() {
        let world = setup_world();
        let c = world.components();

        // Compatible
        let state = <(EntityMutExcept<C1>, &mut C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityMutExcept<C1>, &mut C1)>(&state).is_ok());
        assert!(has_conflicts_large::<(EntityMutExcept<C1>, &mut C1)>(&state).is_ok());
        assert!(has_conflicts::<(EntityMutExcept<C1>, &mut C1)>(c).is_ok());

        let state = <(&mut C1, EntityMutExcept<C1>) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&mut C1, EntityMutExcept<C1>)>(&state).is_ok());
        assert!(has_conflicts_large::<(&mut C1, EntityMutExcept<C1>)>(&state).is_ok());
        assert!(has_conflicts::<(&mut C1, EntityMutExcept<C1>)>(c).is_ok());

        let state = <(&mut C1, EntityMutExcept<(C1, C2)>) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&mut C1, EntityMutExcept<(C1, C2)>)>(&state).is_ok());
        assert!(has_conflicts_large::<(&mut C1, EntityMutExcept<(C1, C2)>)>(&state).is_ok());
        assert!(has_conflicts::<(&mut C1, EntityMutExcept<(C1, C2)>)>(c).is_ok());

        let state = <(EntityMutExcept<(C1, C2)>, &mut C1) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityMutExcept<(C1, C2)>, &mut C1)>(&state).is_ok());
        assert!(has_conflicts_large::<(EntityMutExcept<(C1, C2)>, &mut C1)>(&state).is_ok());
        assert!(has_conflicts::<(EntityMutExcept<(C1, C2)>, &mut C1)>(c).is_ok());

        let state =
            <(&mut C1, &mut C2, EntityMutExcept<(C1, C2)>) as WorldQuery>::get_state(c).unwrap();
        assert!(
            has_conflicts_small::<(&mut C1, &mut C2, EntityMutExcept<(C1, C2)>)>(&state).is_ok()
        );
        assert!(
            has_conflicts_large::<(&mut C1, &mut C2, EntityMutExcept<(C1, C2)>)>(&state).is_ok()
        );
        assert!(has_conflicts::<(&mut C1, &mut C2, EntityMutExcept<(C1, C2)>)>(c).is_ok());

        let state =
            <(&mut C1, EntityMutExcept<(C1, C2)>, &mut C2) as WorldQuery>::get_state(c).unwrap();
        assert!(
            has_conflicts_small::<(&mut C1, EntityMutExcept<(C1, C2)>, &mut C2)>(&state).is_ok()
        );
        assert!(
            has_conflicts_large::<(&mut C1, EntityMutExcept<(C1, C2)>, &mut C2)>(&state).is_ok()
        );
        assert!(has_conflicts::<(&mut C1, EntityMutExcept<(C1, C2)>, &mut C2)>(c).is_ok());

        let state =
            <(EntityMutExcept<(C1, C2)>, &mut C1, &mut C2) as WorldQuery>::get_state(c).unwrap();
        assert!(
            has_conflicts_small::<(EntityMutExcept<(C1, C2)>, &mut C1, &mut C2)>(&state).is_ok()
        );
        assert!(
            has_conflicts_large::<(EntityMutExcept<(C1, C2)>, &mut C1, &mut C2)>(&state).is_ok()
        );
        assert!(has_conflicts::<(EntityMutExcept<(C1, C2)>, &mut C1, &mut C2)>(c).is_ok());
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_mut_except_conflicts_component_read() {
        let world = setup_world();
        let c = world.components();
        let state = <(EntityMutExcept<C1>, &C2) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityMutExcept<C1>, &C2)>(&state).is_err());
        assert!(has_conflicts_large::<(EntityMutExcept<C1>, &C2)>(&state).is_err());
        let _ = has_conflicts::<(EntityMutExcept<C1>, &C2)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_read_conflicts_entity_mut_except() {
        let world = setup_world();
        let c = world.components();
        let state = <(&C2, EntityMutExcept<C1>) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&C2, EntityMutExcept<C1>)>(&state).is_err());
        assert!(has_conflicts_large::<(&C2, EntityMutExcept<C1>)>(&state).is_err());
        let _ = has_conflicts::<(&C2, EntityMutExcept<C1>)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_mut_except_conflicts_component_write() {
        let world = setup_world();
        let c = world.components();
        let state = <(EntityMutExcept<C1>, &mut C2) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(EntityMutExcept<C1>, &mut C2)>(&state).is_err());
        assert!(has_conflicts_large::<(EntityMutExcept<C1>, &mut C2)>(&state).is_err());
        let _ = has_conflicts::<(EntityMutExcept<C1>, &mut C2)>(c);
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_write_conflicts_entity_mut_except() {
        let world = setup_world();
        let c = world.components();
        let state = <(&mut C2, EntityMutExcept<C1>) as WorldQuery>::get_state(c).unwrap();
        assert!(has_conflicts_small::<(&mut C2, EntityMutExcept<C1>)>(&state).is_err());
        assert!(has_conflicts_large::<(&mut C2, EntityMutExcept<C1>)>(&state).is_err());
        let _ = has_conflicts::<(&mut C2, EntityMutExcept<C1>)>(c);
    }
}
