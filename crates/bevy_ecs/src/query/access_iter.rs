use crate::{
    component::{ComponentId, Components},
    query::QueryData,
};

/// The data storage type that is being accessed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EcsAccessType {
    /// Accesses [`Component`](crate::prelude::Component) data
    Component(EcsAccessLevel),
    /// Accesses [`Resource`](crate::prelude::Resource) data
    Resource(ResourceAccessLevel),
}

/// The way the data will be accessed and whether we take access on all the components on
/// an entity or just one component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EcsAccessLevel {
    /// Reads [`Component`](crate::prelude::Component) with [`ComponentId`]
    Read(ComponentId),
    /// Writes [`Component`](crate::prelude::Component) with [`ComponentId`]
    Write(ComponentId),
    /// Potentially reads all [`Component`](crate::prelude::Component)'s in the [`World`](crate::prelude::World)
    ReadAll,
    /// Potentially writes all [`Component`](crate::prelude::Component)'s in the [`World`](crate::prelude::World)
    WriteAll,
    /// [`FilteredEntityRef`](crate::world::FilteredEntityRef) captures it's access at the `SystemParam` level, so will
    /// not conflict with other [`QueryData`] in the same Query
    FilteredReadAll,
    /// [`FilteredEntityMut`](crate::world::FilteredEntityMut) captures it's access at the `SystemParam` level, so will
    /// not conflict with other [`QueryData`] in the same Query
    FilteredWriteAll,
    /// Potentially reads all [`Components`]'s except [`ComponentId`]
    ReadAllExcept {
        /// used to group excepts from the same [`QueryData`] together
        index: usize,
        /// read all except this id
        component_id: ComponentId,
    },
    /// Potentially writes all [`Components`]'s except [`ComponentId`]
    WriteAllExcept {
        /// used to group excepts from the same [`QueryData`] together
        index: usize,
        /// write all except this id
        component_id: ComponentId,
    },
}

/// Access level needed by [`QueryData`] fetch to the resource.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ResourceAccessLevel {
    /// Reads the resource with [`ComponentId`]
    Read(ComponentId),
    /// Writes the resource with [`ComponentId`]
    Write(ComponentId),
}

/// Return value of [`EcsAccessType::is_compatible`]
pub enum AccessCompatible {
    /// Access is compatible
    Compatible,
    /// Access conflicts
    Conflicts,
    /// Access is allowed by `EntityExcept*`. Returns index of the `Except` param.
    CompatibleExcept(usize),
    /// Access conflicts with the `Except` being the second param. Holds the index of the `Except` param
    /// which can be used to disambiguate between different `Except`'s
    ConflictsExcept(usize),
}

impl From<bool> for AccessCompatible {
    fn from(value: bool) -> Self {
        if value {
            AccessCompatible::Compatible
        } else {
            AccessCompatible::Conflicts
        }
    }
}

impl EcsAccessType {
    fn index(&self) -> Option<usize> {
        use EcsAccessLevel::*;
        use EcsAccessType::*;

        match self {
            Component(ReadAllExcept { index, .. }) | Component(WriteAllExcept { index, .. }) => {
                Some(*index)
            }
            _ => None,
        }
    }

    fn is_except(&self) -> bool {
        use EcsAccessLevel::*;
        use EcsAccessType::*;

        matches!(
            self,
            Component(ReadAllExcept { .. }) | Component(WriteAllExcept { .. })
        )
    }

    /// See [`AccessCompatible`] for more info
    #[inline(never)]
    pub fn is_compatible(&self, other: Self) -> AccessCompatible {
        use EcsAccessLevel::*;
        use EcsAccessType::*;

        match (*self, other) {
            (Component(ReadAll), Component(Write(_)))
            | (Component(Write(_)), Component(ReadAll))
            | (Component(_), Component(WriteAll))
            | (Component(WriteAll), Component(_))
            | (Component(WriteAllExcept { .. }), Component(ReadAllExcept { .. }))
            | (Component(ReadAllExcept { .. }), Component(WriteAllExcept { .. }))
            | (Component(WriteAllExcept { .. }), Component(ReadAll))
            | (Component(ReadAll), Component(WriteAllExcept { .. })) => AccessCompatible::Conflicts,

            // resources and components never conflict
            (Component(_), Resource(_))
            | (Resource(_), Component(_))
            // read only access doesn't conflict
            | (Component(Read(_)), Component(Read(_)))
            | (Component(ReadAll), Component(Read(_)))
            | (Component(Read(_)), Component(ReadAll))
            | (Component(ReadAll), Component(ReadAll))
            | (Resource(ResourceAccessLevel::Read(_)), Resource(ResourceAccessLevel::Read(_)))
            | (Component(ReadAllExcept { .. }), Component(Read(_)))
            | (Component(Read(_)), Component(ReadAllExcept { .. }))
            | (Component(ReadAllExcept { .. }), Component(ReadAll))
            | (Component(ReadAll), Component(ReadAllExcept { .. }))
            | (Component(ReadAllExcept { .. }), Component(ReadAllExcept { .. }))
            // filtered access takes it's access from what is remaining
            | (Component(FilteredReadAll), _)
            | (_, Component(FilteredReadAll))
            | (Component(FilteredWriteAll), _)
            | (_, Component(FilteredWriteAll)) => {
                AccessCompatible::Compatible
            }

            // single component checks
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
            ) => (id != id_other).into(),

            // Except* access in first parameter
            (
                Component(ReadAllExcept {
                    component_id: id,
                    index,
                }),
                Component(Write(id_other)),
            )
            | (
                Component(WriteAllExcept {
                    component_id: id,
                    index,
                }),
                Component(Read(id_other)),
            )
            | (
                Component(WriteAllExcept {
                    component_id: id,
                    index,
                }),
                Component(Write(id_other)),
            )
            // Except* access in second parameter
            | (
                Component(Write(id)),
                Component(ReadAllExcept {
                    component_id: id_other,
                    index,
                }),
            )
            | (
                Component(Read(id)),
                Component(WriteAllExcept {
                    component_id: id_other,
                    index,
                }),
            )
            | (
                Component(Write(id)),
                Component(WriteAllExcept {
                    component_id: id_other,
                    index,
                }),
            ) => {
                if id == id_other {
                    AccessCompatible::CompatibleExcept(index)
                } else {
                    AccessCompatible::ConflictsExcept(index)
                }
            }

            // WriteAll will always conflict if they have different indexes
            (
                Component(WriteAllExcept { index, .. }),
                Component(WriteAllExcept {
                    index: index_other, ..
                }),
            ) => (index == index_other).into(),
        }
    }
}

/// Error returned from [`has_conflicts`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QueryAccessError {
    /// Component was not registered on world
    ComponentNotRegistered,
    /// The [`EcsAccessType`]'s conflict with each other
    Conflict,
    /// Entity did not have the requested components
    EntityDoesNotMatch,
}

impl core::error::Error for QueryAccessError {}

impl core::fmt::Display for QueryAccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            QueryAccessError::ComponentNotRegistered => {
                write!(
                    f,
                    "At least one component in Q was not registered in world. 
                    Consider calling `World::register_component`"
                )
            }
            QueryAccessError::Conflict => {
                write!(f, "Access conflict in Q")
            }
            QueryAccessError::EntityDoesNotMatch => {
                write!(f, "Entity does not match Q")
            }
        }
    }
}

/// Check if `Q` has any internal conflicts.
#[inline(never)]
pub fn has_conflicts<Q: QueryData>(components: &Components) -> Result<(), QueryAccessError> {
    // increasing this too much may slow down smaller queries
    const MAX_SIZE: usize = 16;
    let mut index_outer = 0;
    let iter = Q::iter_access(components, &mut index_outer).enumerate();
    // Note: The two algorithms have very similar performance at small sizes. It's only around n = 10
    // where they start to diverge significantly.
    if iter
        .size_hint()
        .1
        .is_none_or(|max_size| max_size > MAX_SIZE)
    {
        // This algorithm will work for any size
        for (i, access) in iter {
            // only check except* conflicts in second iterator
            if access.as_ref().is_some_and(EcsAccessType::is_except) {
                continue;
            }

            let mut index_inner = 0;
            let mut except_index = None;
            let mut except_compatible = false;
            for (j, access_other) in Q::iter_access(components, &mut index_inner).enumerate() {
                // don't check for conflicts when the access is the same access
                if i == j {
                    continue;
                }
                let (Some(access), Some(access_other)) = (access, access_other) else {
                    return Err(QueryAccessError::ComponentNotRegistered);
                };

                // if we're in an except sequence, check if the sequence has ended
                if let Some(current_index) = except_index {
                    let sequence_ended = if let Some(index_other) = access_other.index() {
                        current_index != index_other
                    } else {
                        true
                    };

                    if sequence_ended {
                        if !except_compatible {
                            return Err(QueryAccessError::Conflict);
                        }
                        except_compatible = false;
                        except_index = None;
                    }
                }

                match access.is_compatible(access_other) {
                    AccessCompatible::Compatible => continue,
                    AccessCompatible::Conflicts => return Err(QueryAccessError::Conflict),
                    AccessCompatible::CompatibleExcept(index) => {
                        except_index = Some(index);
                        except_compatible = true;
                    }
                    AccessCompatible::ConflictsExcept(index) => {
                        except_index = Some(index);
                    }
                }
            }

            if except_index.is_some() && !except_compatible {
                return Err(QueryAccessError::Conflict);
            }
        }
        Ok(())
    } else {
        // we can use a faster algorithm by putting some fixed size
        // arrays onto the stack
        let mut compatibles = [[false; MAX_SIZE]; MAX_SIZE];
        let mut conflicts = [[false; MAX_SIZE]; MAX_SIZE];
        let size = iter.size_hint().1.unwrap_or(MAX_SIZE);
        for (i, access) in iter {
            let mut index_inner = 0;
            for (j, access_other) in Q::iter_access(components, &mut index_inner)
                .enumerate()
                .take(i)
            {
                if i == j {
                    continue;
                }
                let (Some(access), Some(access_other)) = (access, access_other) else {
                    return Err(QueryAccessError::ComponentNotRegistered);
                };

                match access.is_compatible(access_other) {
                    AccessCompatible::Compatible => continue,
                    AccessCompatible::Conflicts => return Err(QueryAccessError::Conflict),
                    AccessCompatible::CompatibleExcept(index) => {
                        let not_except = if access.is_except() { j } else { i };
                        compatibles[index][not_except] = true;
                    }
                    AccessCompatible::ConflictsExcept(index) => {
                        let not_except = if access.is_except() { j } else { i };
                        conflicts[index][not_except] = true;
                    }
                }
            }
        }

        for (compatible, conflict) in compatibles
            .iter()
            .flatten()
            .take(size * size)
            .zip(conflicts.iter().flatten().take(size * size))
        {
            if *conflict && !*compatible {
                return Err(QueryAccessError::Conflict);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        prelude::Component,
        world::{EntityMut, EntityMutExcept, EntityRef, EntityRefExcept, World},
    };

    #[derive(Component)]
    struct C1;

    #[derive(Component)]
    struct C2;

    #[test]
    fn simple_conflicts() {
        let mut world = World::new();
        world.register_component::<C1>();
        world.register_component::<C2>();
        let c = world.components();

        // Compatible
        assert!(has_conflicts::<&mut C1>(c).is_ok());
        assert!(has_conflicts::<&C1>(c).is_ok());
        assert!(has_conflicts::<(&C1, &C1)>(c).is_ok());

        // Conflicts
        assert!(matches!(
            has_conflicts::<(&C1, &mut C1)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(&mut C1, &C1)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(&mut C1, &mut C1)>(c),
            Err(QueryAccessError::Conflict)
        ));
    }

    #[test]
    fn entity_ref_mut_conflicts() {
        let mut world = World::new();
        world.register_component::<C1>();
        world.register_component::<C2>();
        let c = world.components();

        // Compatible
        assert!(has_conflicts::<(EntityRef, &C1)>(c).is_ok());
        assert!(has_conflicts::<(&C1, EntityRef)>(c).is_ok());
        assert!(has_conflicts::<(EntityRef, EntityRef)>(c).is_ok());

        // Conflicts
        assert!(matches!(
            has_conflicts::<(EntityRef, &mut C1)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(&mut C1, EntityRef)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(EntityMut, &C1)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(&C1, EntityMut)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(EntityMut, &mut C1)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(&mut C1, EntityMut)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(EntityMut, EntityRef)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(EntityRef, EntityMut)>(c),
            Err(QueryAccessError::Conflict)
        ));
    }

    #[test]
    fn entity_ref_except_conflicts() {
        let mut world = World::new();
        world.register_component::<C1>();
        world.register_component::<C2>();
        let c = world.components();

        // Compatible
        assert!(has_conflicts::<(EntityRefExcept<C1>, &mut C1)>(c).is_ok());
        assert!(has_conflicts::<(&mut C1, EntityRefExcept<C1>)>(c).is_ok());
        assert!(has_conflicts::<(&C2, EntityRefExcept<C1>)>(c).is_ok());
        assert!(has_conflicts::<(&mut C1, EntityRefExcept<(C1, C2)>,)>(c).is_ok());
        assert!(has_conflicts::<(EntityRefExcept<(C1, C2)>, &mut C1,)>(c).is_ok());
        assert!(has_conflicts::<(&mut C1, &mut C2, EntityRefExcept<(C1, C2)>,)>(c).is_ok());
        assert!(has_conflicts::<(&mut C1, EntityRefExcept<(C1, C2)>, &mut C2,)>(c).is_ok());
        assert!(has_conflicts::<(EntityRefExcept<(C1, C2)>, &mut C1, &mut C2,)>(c).is_ok());

        // Conflicts
        assert!(matches!(
            has_conflicts::<(EntityRefExcept<C1>, &mut C2)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(&mut C2, EntityRefExcept<C1>)>(c),
            Err(QueryAccessError::Conflict)
        ));
    }

    #[test]
    fn entity_mut_except_conflicts() {
        let mut world = World::new();
        world.register_component::<C1>();
        world.register_component::<C2>();
        let c = world.components();

        // Compatible
        assert!(has_conflicts::<(EntityMutExcept<C1>, &mut C1)>(c).is_ok());
        assert!(has_conflicts::<(&mut C1, EntityMutExcept<C1>)>(c).is_ok());
        assert!(has_conflicts::<(&mut C1, EntityMutExcept<(C1, C2)>,)>(c).is_ok());
        assert!(has_conflicts::<(EntityMutExcept<(C1, C2)>, &mut C1,)>(c).is_ok());
        assert!(has_conflicts::<(&mut C1, &mut C2, EntityMutExcept<(C1, C2)>,)>(c).is_ok());
        assert!(has_conflicts::<(&mut C1, EntityMutExcept<(C1, C2)>, &mut C2,)>(c).is_ok());
        assert!(has_conflicts::<(EntityMutExcept<(C1, C2)>, &mut C1, &mut C2,)>(c).is_ok());

        // Conflicts
        assert!(matches!(
            has_conflicts::<(&C2, EntityMutExcept<C1>)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(EntityMutExcept<C1>, &C2)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(EntityMutExcept<C1>, &mut C2)>(c),
            Err(QueryAccessError::Conflict)
        ));
        assert!(matches!(
            has_conflicts::<(&mut C2, EntityMutExcept<C1>)>(c),
            Err(QueryAccessError::Conflict)
        ));
    }
}
