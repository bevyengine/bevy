use crate::{
    component::{ComponentId, Components},
    query::{Access, QueryData},
};

/// The data storage type that is being accessed.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EcsAccessType<'a> {
    /// Accesses [`Component`](crate::prelude::Component) data
    Component(EcsAccessLevel),
    /// Accesses [`Resource`](crate::prelude::Resource) data
    Resource(ResourceAccessLevel),
    /// borrowed access from [`WorldQuery::State`]
    Access(&'a Access),
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

impl<'a> EcsAccessType<'a> {
    /// See [`AccessCompatible`] for more info
    #[inline(never)]
    pub fn is_compatible(&self, other: Self) -> AccessCompatible {
        use EcsAccessLevel::*;
        use EcsAccessType::*;

        match (*self, other) {
            (Component(ReadAll), Component(Write(_)))
            | (Component(Write(_)), Component(ReadAll))
            | (Component(_), Component(WriteAll))
            | (Component(WriteAll), Component(_)) => AccessCompatible::Conflicts,

            (Component(_), Resource(_))
            | (Resource(_), Component(_))
            // read only access doesn't conflict
            | (Component(Read(_)), Component(Read(_)))
            | (Component(ReadAll), Component(Read(_)))
            | (Component(Read(_)), Component(ReadAll))
            | (Component(ReadAll), Component(ReadAll))
            | (Resource(ResourceAccessLevel::Read(_)), Resource(ResourceAccessLevel::Read(_))) => {
                AccessCompatible::Compatible
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
            ) => (id != id_other).into(),

            // Borrowed Access
            (Component(Read(component_id)), Access(access))
            | (Access(access), Component(Read(component_id))) => {
                (!access.has_component_write(component_id)).into()
            },

            (Component(Write(component_id)), Access(access))
            | (Access(access), Component(Write(component_id))) =>
                (!access.has_component_read(component_id)).into(),

            (Component(ReadAll), Access(access))
            | (Access(access), Component(ReadAll))=> (!access.has_any_component_write()).into(),

            (Component(WriteAll), Access(access))
            | (Access(access), Component(WriteAll))=> (!access.has_any_component_read()).into(),
            
            (Resource(ResourceAccessLevel::Read(component_id)), Access(access))
            | (Access(access), Resource(ResourceAccessLevel::Read(component_id))) => (!access.has_resource_write(component_id)).into(),
            (Resource(ResourceAccessLevel::Write(component_id)), Access(access))
            | (Access(access), Resource(ResourceAccessLevel::Write(component_id))) => (!access.has_resource_read(component_id)).into(),

            (Access(access), Access(other_access)) => access.is_compatible(other_access).into(),
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
    let Some(state) = Q::get_state(components) else {
        return Err(QueryAccessError::ComponentNotRegistered);
    };
    for (i, access) in Q::iter_access(&state).enumerate() {
        for access_other in Q::iter_access(&state).take(i) {
            match access.is_compatible(access_other) {
                AccessCompatible::Compatible => continue,
                AccessCompatible::Conflicts => return Err(QueryAccessError::Conflict),
            }
        }
    }
    Ok(())
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
