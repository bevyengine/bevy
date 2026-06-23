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

/// Check `Q` for internal conflicts, panicking if there are any.
///
/// Returns an error if not all components are registered.
#[inline(never)]
pub fn has_conflicts<Q: QueryData>(components: &Components) -> Result<(), QueryAccessError> {
    let Some(state) = Q::get_state(components) else {
        return Err(QueryAccessError::ComponentNotRegistered);
    };

    let result = if let Some(size) = Q::iter_access(&state).size_hint().1
        && size <= USE_FILTER_THRESHOLD
    {
        has_conflicts_small::<Q>(&state)
    } else {
        has_conflicts_large::<Q>(&state)
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
        inner_access[i] = access;
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
            EcsAccessType::Access(access) => {
                if let Ok(component_iter) = access.try_iter_access() {
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
            // read only access doesn't conflict
            | (Component(Read(_)), Component(Read(_)))
            | (Component(ReadAll), Component(Read(_)))
            | (Component(Read(_)), Component(ReadAll))
            | (Component(ReadAll), Component(ReadAll))
             => {
                Ok(())
            }

            (Component(Read(id)), Component(Write(id_other)))
            | (Component(Write(id)), Component(Read(id_other)))
            | (Component(Write(id)), Component(Write(id_other)))
 => if id == id_other {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },

            // Borrowed Access
            (Component(Read(component_id)), Access(access))
            | (Access(access), Component(Read(component_id))) => if access.has_write(component_id) {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },

            (Component(Write(component_id)), Access(access))
            | (Access(access), Component(Write(component_id))) => if access.has_read(component_id) {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },

            (Component(ReadAll), Access(access))
            | (Access(access), Component(ReadAll)) => if access.has_any_write() {
                Err(AccessConflictError(*self, other))
            } else {
                Ok(())
            },

            (Component(WriteAll), Access(access))
            | (Access(access), Component(WriteAll))=> if access.has_any_read() {
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
            (Access(_), Access(_)) => write!(f, "Access conflicts with other Access"),

            _ => {
                unreachable!("Other accesses should be compatible");
            }
        }
    }
}

/// Error indicating the entity does not have all requested component ids.
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

    fn assert_compatible<Q: WorldQuery + QueryData>() {
        let world = setup_world();
        let c = world.components();

        let state = <Q>::get_state(c).unwrap();
        assert!(has_conflicts_small::<Q>(&state).is_ok());
        assert!(has_conflicts_large::<Q>(&state).is_ok());
        assert!(has_conflicts::<Q>(c).is_ok());
    }

    fn assert_conflicted<Q: WorldQuery + QueryData>() {
        let world = setup_world();
        let c = world.components();

        let state = <Q>::get_state(c).unwrap();
        assert!(has_conflicts_small::<Q>(&state).is_err());
        assert!(has_conflicts_large::<Q>(&state).is_err());
        let _ = has_conflicts::<Q>(c);
    }

    #[test]
    fn simple_compatible() {
        assert_compatible::<&mut C1>();
        assert_compatible::<&C1>();
        assert_compatible::<(&C1, &C1)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn conflict_component_read_conflicts_write() {
        assert_conflicted::<(&C1, &mut C1)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn conflict_component_write_conflicts_read() {
        assert_conflicted::<(&mut C1, &C1)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn conflict_component_write_conflicts_write() {
        assert_conflicted::<(&mut C1, &mut C1)>();
    }

    #[test]
    fn entity_ref_compatible() {
        assert_compatible::<(EntityRef, &C1)>();
        assert_compatible::<(&C1, EntityRef)>();
        assert_compatible::<(EntityRef, EntityRef)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_ref_conflicts_component_write() {
        assert_conflicted::<(EntityRef, &mut C1)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_write_conflicts_entity_ref() {
        assert_conflicted::<(&mut C1, EntityRef)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_mut_conflicts_component_read() {
        assert_conflicted::<(EntityMut, &C1)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_read_conflicts_entity_mut() {
        assert_conflicted::<(&C1, EntityMut)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_mut_conflicts_component_write() {
        assert_conflicted::<(EntityMut, &mut C1)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_write_conflicts_entity_mut() {
        assert_conflicted::<(&mut C1, EntityMut)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_mut_conflicts_entity_ref() {
        assert_conflicted::<(EntityMut, EntityRef)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_ref_conflicts_entity_mut() {
        assert_conflicted::<(EntityRef, EntityMut)>();
    }

    #[test]
    fn entity_ref_except_compatible() {
        assert_compatible::<(EntityRefExcept<C1>, &mut C1)>();
        assert_compatible::<(&mut C1, EntityRefExcept<C1>)>();
        assert_compatible::<(&C2, EntityRefExcept<C1>)>();
        assert_compatible::<(&mut C1, EntityRefExcept<(C1, C2)>)>();
        assert_compatible::<(EntityRefExcept<(C1, C2)>, &mut C1)>();
        assert_compatible::<(&mut C1, &mut C2, EntityRefExcept<(C1, C2)>)>();
        assert_compatible::<(&mut C1, EntityRefExcept<(C1, C2)>, &mut C2)>();
        assert_compatible::<(EntityRefExcept<(C1, C2)>, &mut C1, &mut C2)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_ref_except_conflicts_component_write() {
        assert_conflicted::<(EntityRefExcept<C1>, &mut C2)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_write_conflicts_entity_ref_except() {
        assert_conflicted::<(&mut C2, EntityRefExcept<C1>)>();
    }

    #[test]
    fn entity_mut_except_compatible() {
        assert_compatible::<(EntityMutExcept<C1>, &mut C1)>();
        assert_compatible::<(&mut C1, EntityMutExcept<C1>)>();
        assert_compatible::<(&mut C1, EntityMutExcept<(C1, C2)>)>();
        assert_compatible::<(EntityMutExcept<(C1, C2)>, &mut C1)>();
        assert_compatible::<(&mut C1, &mut C2, EntityMutExcept<(C1, C2)>)>();
        assert_compatible::<(&mut C1, EntityMutExcept<(C1, C2)>, &mut C2)>();
        assert_compatible::<(EntityMutExcept<(C1, C2)>, &mut C1, &mut C2)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_mut_except_conflicts_component_read() {
        assert_conflicted::<(EntityMutExcept<C1>, &C2)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_read_conflicts_entity_mut_except() {
        assert_conflicted::<(&C2, EntityMutExcept<C1>)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn entity_mut_except_conflicts_component_write() {
        assert_conflicted::<(EntityMutExcept<C1>, &mut C2)>();
    }

    #[test]
    #[should_panic(expected = "conflicts")]
    fn component_write_conflicts_entity_mut_except() {
        assert_conflicted::<(&mut C2, EntityMutExcept<C1>)>();
    }
}
