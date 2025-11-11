use crate::{
    component::{ComponentId, Components},
    query::QueryData,
};

/// The data storage type that is being accessed.
#[derive(Clone, Copy)]
pub enum EcsAccessType {
    /// Accesses [`Component`] data
    Component(EcsAccessLevel),
    /// Accesses [`Resource`] data
    Resource(ResourceAccessLevel),
}

/// The way the data will be accessed and whether we take access on all the components on
/// an entity or just one component.
#[derive(Clone, Copy)]
pub enum EcsAccessLevel {
    /// Reads [`Component`] with [`ComponentId`]
    Read(ComponentId),
    /// Writes [`Component`] with [`ComponentId`]
    Write(ComponentId),
    /// Potentially reads all [`Component`]'s in the [`World`]
    ReadAll,
    /// Potentially writes all [`Component`]'s in the [`World`]
    WriteAll,
    /// [`FilteredEntityRef`] captures it's access at the `SystemParam` level, so will
    /// not conflict with other `QueryData` in the same Query
    FilteredReadAll,
    /// [`FilteredEntityMut`] captures it's access at the `SystemParam` level, so will
    /// not conflict with other `QueryData` in the same Query
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
#[derive(Copy, Clone)]
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
    /// Access conflicts, but is the first param, so we ignore it and check when the order is reversed.
    ConflictsExceptFirst,
    /// Access conflicts with the `Except` being the second param. Holds the index of the `Except` param
    /// which can be used to disambiguate between different `Except`'s
    ConflictsExceptSecond(usize),
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

    /// Returns true if the access between `self` and `other` do not conflict.
    pub fn is_compatible(&self, other: Self) -> AccessCompatible {
        use EcsAccessLevel::*;
        use EcsAccessType::*;

        match (self, other) {
            (Component(ReadAll), Component(Write(_)))
            | (Component(Write(_)), Component(ReadAll))
            | (Component(_), Component(WriteAll))
            | (Component(WriteAll), Component(_))
            | (Component(WriteAllExcept { .. }), Component(ReadAllExcept { .. }))
            | (Component(ReadAllExcept { .. }), Component(WriteAllExcept { .. }))
            | (Component(WriteAllExcept { .. }), Component(ReadAll))
            | (Component(ReadAll), Component(WriteAllExcept { .. })) => AccessCompatible::Conflicts,

            (Component(_), Resource(_))
            | (Resource(_), Component(_))
            | (Component(Read(_)), Component(Read(_)))
            | (Component(ReadAll), Component(Read(_)))
            | (Component(Read(_)), Component(ReadAll))
            | (Component(ReadAll), Component(ReadAll))
            | (Resource(ResourceAccessLevel::Read(_)), Resource(ResourceAccessLevel::Read(_)))
            // TODO: I think (FilterdReadAll, FilteredWriteAll) should probably conflict, but should
            // double check with the normal conflict check
            | (Component(FilteredReadAll), _)
            | (_, Component(FilteredReadAll))
            | (Component(FilteredWriteAll), _)
            | (_, Component(FilteredWriteAll))
            | (Component(ReadAllExcept { .. }), Component(Read(_)))
            | (Component(Read(_)), Component(ReadAllExcept { .. }))
            | (Component(ReadAllExcept { .. }), Component(ReadAll))
            | (Component(ReadAll), Component(ReadAllExcept { .. }))
            | (Component(ReadAllExcept { .. }), Component(ReadAllExcept { .. })) => AccessCompatible::Compatible,

            (Component(Read(id)), Component(Write(id_other)))
            | (Component(Write(id)), Component(Read(id_other)))
            | (Component(Write(id)), Component(Write(id_other)))
            | (Resource(ResourceAccessLevel::Read(id)), Resource(ResourceAccessLevel::Write(id_other)))
            | (Resource(ResourceAccessLevel::Write(id)), Resource(ResourceAccessLevel::Read(id_other)))
            | (Resource(ResourceAccessLevel::Write(id)), Resource(ResourceAccessLevel::Write(id_other))) => (*id != id_other).into(),

            (Component(ReadAllExcept { component_id: id, .. }), Component(Write(id_other)))
            | (Component(WriteAllExcept { component_id: id, .. }), Component(Read(id_other)))
            | (Component(WriteAllExcept { component_id: id, .. }), Component(Write(id_other))) => {
                if *id == id_other {
                    AccessCompatible::Compatible
                } else {
                    AccessCompatible::ConflictsExceptFirst
                }
            },

            (Component(Write(id)), Component(ReadAllExcept { component_id: id_other, index }))
            | (Component(Read(id)), Component(WriteAllExcept { component_id: id_other, index }))
            | (Component(Write(id)), Component(WriteAllExcept { component_id: id_other, index })) => {
                if *id == id_other {
                    AccessCompatible::CompatibleExcept(index)
                } else {
                    AccessCompatible::ConflictsExceptSecond(index)
                }
            },

            (Component(WriteAllExcept { index, .. }), Component(WriteAllExcept { index: index_other, .. })) => (*index == index_other).into(),
        }
    }
}

/// Check if `Q` has any internal conflicts.
pub fn has_conflicts<Q: QueryData>(components: &Components) -> bool {
    let mut index_outer = 0;
    for (i, access) in Q::iter_access(components, &mut index_outer).enumerate() {
        let mut index_inner = 0;
        let mut except_index = None;
        let mut except_compatible = false;
        for (j, access_other) in Q::iter_access(components, &mut index_inner).enumerate() {
            // don't check for conflicts when the access is the same access
            if i == j {
                continue;
            }
            let (Some(access), Some(access_other)) = (access, access_other) else {
                // A component wasn't registered
                return true;
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
                        return true;
                    }
                    except_compatible = false;
                    except_index = None;
                }
            }

            match access.is_compatible(access_other) {
                AccessCompatible::Compatible
                    // ignore *Except conflicts if they're in the outer loop and only check them in the inner loop
                    | AccessCompatible::ConflictsExceptFirst => continue,
                AccessCompatible::CompatibleExcept(index) => {
                    except_index = Some(index);
                    except_compatible = true;
                },
                AccessCompatible::Conflicts => return true,
                AccessCompatible::ConflictsExceptSecond(index) => {
                    except_index = Some(index);
                }
            }
        }

        if except_index.is_some() && !except_compatible {
            return true;
        }
    }
    false
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
        assert!(!has_conflicts::<&mut C1>(c));
        assert!(!has_conflicts::<&C1>(c));
        assert!(!has_conflicts::<(&C1, &C1)>(c));

        // Conflicts
        assert!(has_conflicts::<(&C1, &mut C1)>(c));
        assert!(has_conflicts::<(&mut C1, &C1)>(c));
        assert!(has_conflicts::<(&mut C1, &mut C1)>(c));
    }

    #[test]
    fn entity_ref_mut_conflicts() {
        let mut world = World::new();
        world.register_component::<C1>();
        world.register_component::<C2>();
        let c = world.components();

        // Compatible
        assert!(!has_conflicts::<(EntityRef, &C1)>(c));
        assert!(!has_conflicts::<(&C1, EntityRef)>(c));
        assert!(!has_conflicts::<(EntityRef, EntityRef)>(c));

        // Conflicts
        assert!(has_conflicts::<(EntityRef, &mut C1)>(c));
        assert!(has_conflicts::<(&mut C1, EntityRef)>(c));
        assert!(has_conflicts::<(EntityMut, &C1)>(c));
        assert!(has_conflicts::<(&C1, EntityMut)>(c));
        assert!(has_conflicts::<(EntityMut, &mut C1)>(c));
        assert!(has_conflicts::<(&mut C1, EntityMut)>(c));
        assert!(has_conflicts::<(EntityMut, EntityRef)>(c));
        assert!(has_conflicts::<(EntityRef, EntityMut)>(c));
    }

    #[test]
    fn entity_ref_except_conflicts() {
        let mut world = World::new();
        world.register_component::<C1>();
        world.register_component::<C2>();
        let c = world.components();

        // Compatible
        assert!(!has_conflicts::<(EntityRefExcept<C1>, &mut C1)>(c));
        assert!(!has_conflicts::<(&mut C1, EntityRefExcept<C1>)>(c));
        assert!(!has_conflicts::<(&C2, EntityRefExcept<C1>)>(c));
        assert!(!has_conflicts::<(&mut C1, EntityRefExcept<(C1, C2)>,)>(c));
        assert!(!has_conflicts::<(EntityRefExcept<(C1, C2)>, &mut C1,)>(c));
        assert!(!has_conflicts::<(
            &mut C1,
            &mut C2,
            EntityRefExcept<(C1, C2)>,
        )>(c));
        assert!(!has_conflicts::<(
            &mut C1,
            EntityRefExcept<(C1, C2)>,
            &mut C2,
        )>(c));
        assert!(!has_conflicts::<(
            EntityRefExcept<(C1, C2)>,
            &mut C1,
            &mut C2,
        )>(c));

        // Conflicts
        assert!(has_conflicts::<(EntityRefExcept<C1>, &mut C2)>(c));
        assert!(has_conflicts::<(&mut C2, EntityRefExcept<C1>)>(c));
    }

    #[test]
    fn entity_mut_except_conflicts() {
        let mut world = World::new();
        world.register_component::<C1>();
        world.register_component::<C2>();
        let c = world.components();

        // Compatible
        assert!(!has_conflicts::<(EntityMutExcept<C1>, &mut C1)>(c));
        assert!(!has_conflicts::<(&mut C1, EntityMutExcept<C1>)>(c));
        assert!(!has_conflicts::<(&mut C1, EntityMutExcept<(C1, C2)>,)>(c));
        assert!(!has_conflicts::<(EntityMutExcept<(C1, C2)>, &mut C1,)>(c));
        assert!(!has_conflicts::<(
            &mut C1,
            &mut C2,
            EntityMutExcept<(C1, C2)>,
        )>(c));
        assert!(!has_conflicts::<(
            &mut C1,
            EntityMutExcept<(C1, C2)>,
            &mut C2,
        )>(c));
        assert!(!has_conflicts::<(
            EntityMutExcept<(C1, C2)>,
            &mut C1,
            &mut C2,
        )>(c));

        // Conflicts
        assert!(has_conflicts::<(&C2, EntityMutExcept<C1>)>(c));
        assert!(has_conflicts::<(EntityMutExcept<C1>, &C2)>(c));
        assert!(has_conflicts::<(EntityMutExcept<C1>, &mut C2)>(c));
        assert!(has_conflicts::<(&mut C2, EntityMutExcept<C1>)>(c));
    }
}
