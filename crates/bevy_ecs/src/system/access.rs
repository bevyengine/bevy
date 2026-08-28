use alloc::borrow::Cow;
use bevy_utils::prelude::ShortName;

use crate::{
    query::{AccessConflicts, FilteredAccess, FilteredAccessSet},
    system::SystemMeta,
};

/// Represents the access a [`System`] requires to the [`World`].
///
/// [`System`]: crate::system::System
/// [`World`]: crate::world::World
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SystemAccess {
    /// The system does not access the world at all.
    #[default]
    None,
    /// The system requires shared access to the world, which means it can run
    /// in parallel with other systems that also require shared access, as long
    /// as they don't require exclusive access to the same components.
    Shared(FilteredAccessSet),
    /// The system requires exclusive access to the world, meaning no other
    /// [`Shared`] or [`Exclusive`] system can run in parallel with it.
    ///
    /// [`Shared`]: SystemAccess::Shared
    /// [`Exclusive`]: SystemAccess::Exclusive
    Exclusive,
}

impl SystemAccess {
    /// Returns true if the system does not access the world at all, so it can run
    /// in parallel with any other system.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns true if the system requires shared access to the world, which
    /// means it can run in parallel with other systems that also require shared
    /// access, as long as they don't require exclusive access to the same components.
    pub fn is_shared(&self) -> bool {
        matches!(self, Self::Shared(_))
    }

    /// Returns true if the system requires exclusive access to the world.
    pub fn is_exclusive(&self) -> bool {
        matches!(self, Self::Exclusive)
    }

    /// Returns true if this system's access is compatible with the other
    /// system's access, such that they can run in parallel without conflicting
    /// with each other.
    pub fn is_compatible(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, _) | (_, Self::None) => true,
            (Self::Shared(access), Self::Shared(other_access)) => {
                access.is_compatible(other_access)
            }
            (Self::Exclusive, _) | (_, Self::Exclusive) => false,
        }
    }

    /// Returns the conflicts between the system's current access and the
    /// provided access.
    ///
    /// - If the system currently has [`None`] access, it will return an empty
    ///   [`AccessConflicts`] (i.e. no conflicts).
    /// - If the system currently has [`Shared`] access, the conflicts between
    ///   individual components or resources will be returned.
    /// - If the system currently has [`Exclusive`] access, it will return
    ///   [`AccessConflicts::All`].
    ///
    /// [`None`]: SystemAccess::None
    /// [`Shared`]: SystemAccess::Shared
    /// [`Exclusive`]: SystemAccess::Exclusive
    pub fn get_conflicts_single(&self, filtered_access: &FilteredAccess) -> AccessConflicts {
        match self {
            SystemAccess::None => AccessConflicts::empty(),
            SystemAccess::Shared(access) => access.get_conflicts_single(filtered_access),
            SystemAccess::Exclusive => AccessConflicts::All,
        }
    }

    /// Returns the conflicts between the system's current access and another
    /// system's access.
    ///
    /// - If either system has [`None`] access, it will return an empty
    ///   [`AccessConflicts`] (i.e. no conflicts).
    /// - If both systems have [`Shared`] access, the conflicts between
    ///   individual components or resources will be returned.
    /// - If either system has [`Exclusive`] access, it will return
    ///   [`AccessConflicts::All`].
    ///
    /// [`None`]: SystemAccess::None
    /// [`Shared`]: SystemAccess::Shared
    /// [`Exclusive`]: SystemAccess::Exclusive
    pub fn get_conflicts(&self, other: &Self) -> AccessConflicts {
        match (self, other) {
            (SystemAccess::None, _) | (_, SystemAccess::None) => AccessConflicts::empty(),
            (SystemAccess::Shared(access), SystemAccess::Shared(other_access)) => {
                access.get_conflicts(other_access)
            }
            (SystemAccess::Exclusive, _) | (_, SystemAccess::Exclusive) => AccessConflicts::All,
        }
    }

    /// Converts the system's current access into a [`FilteredAccessSet`].
    pub fn to_filtered_access_set(&self) -> Cow<'_, FilteredAccessSet> {
        match self {
            Self::None => Cow::Owned(FilteredAccessSet::new()),
            Self::Shared(access) => Cow::Borrowed(access),
            Self::Exclusive => {
                let mut access_set = FilteredAccessSet::new();
                let mut access = FilteredAccess::default();
                access.write_all();
                access_set.add(access);
                Cow::Owned(access_set)
            }
        }
    }

    /// Marks the system as requiring shared access to the world, which means it
    /// can run in parallel with other systems that also require shared access,
    /// as long as they don't require exclusive access to the same components.
    ///
    /// The provided type `T` is used for error reporting in case of access conflicts;
    /// it should be the type of the system parameter that is requesting shared access.
    ///
    /// # Panics
    ///
    /// If the system already has exclusive access, this method will panic with
    /// an error message indicating the conflict.
    ///
    /// # Examples
    ///
    /// ## Only metadata access required
    ///
    /// If a system parameter requires access to world metadata but not any
    /// particular components, you may call this function without modifying the
    /// returned [`FilteredAccessSet`]:
    ///
    /// ```rust,no_run
    /// # use bevy_ecs::system::{SystemAccess, SystemMeta, SystemParam, SystemParamValidationError};
    /// # use bevy_ecs::world::{unsafe_world_cell::UnsafeWorldCell, World, WorldId};
    /// # use bevy_ecs::change_detection::Tick;
    /// pub struct MyWorldId(WorldId);
    ///
    /// // SAFETY: World metadata is registered and accessed.
    /// unsafe impl SystemParam for MyWorldId {
    ///     type State = ();
    ///     type Item<'world, 'state> = MyWorldId;
    ///
    ///     fn init_state(_: &mut World) -> Self::State {}
    ///
    ///     fn init_access(
    ///         _state: &Self::State,
    ///         system_meta: &mut SystemMeta,
    ///         system_access: &mut SystemAccess,
    ///         _world: &mut World,
    ///     ) {
    ///         system_access.require_shared_access::<Self>(system_meta);
    ///     }
    ///
    ///     unsafe fn get_param<'world, 'state>(
    ///         _: &'state mut Self::State,
    ///         _: &SystemMeta,
    ///         world: UnsafeWorldCell<'world>,
    ///         _: Tick,
    ///     ) -> Result<Self::Item<'world, 'state>, SystemParamValidationError> {
    ///         Ok(MyWorldId(world.id()))
    ///     }
    /// }
    /// ```
    pub fn require_shared_access<T>(&mut self, system_meta: &SystemMeta) -> &mut FilteredAccessSet {
        match self {
            this @ Self::None => {
                *this = Self::Shared(FilteredAccessSet::new());
                if let Self::Shared(access) = this {
                    access
                } else {
                    unreachable!()
                }
            }
            Self::Shared(access) => access,
            Self::Exclusive => panic!(
                "error[B0002]: {} in system {} conflicts with a previous system parameter.",
                ShortName::of::<T>(),
                system_meta.name()
            ),
        }
    }

    /// Marks the system as requiring exclusive access to the world, meaning no
    /// other [`Shared`] or [`Exclusive`] system can run in parallel with it.
    ///
    /// The provided type `T` is used for error reporting in case of access conflicts;
    /// it should be the type of the system parameter that is requesting exclusive access.
    ///
    /// # Panics
    ///
    /// If the system already has shared or exclusive access, this method will
    /// panic with an error message indicating the conflict.
    ///
    /// [`Shared`]: SystemAccess::Shared
    /// [`Exclusive`]: SystemAccess::Exclusive
    pub fn require_exclusive_access<T>(&mut self, system_meta: &SystemMeta) {
        if !matches!(self, Self::None) {
            panic!(
                "error[B0002]: {} in system {} conflicts with a previous system parameter.",
                ShortName::of::<T>(),
                system_meta.name()
            );
        }
        *self = Self::Exclusive;
    }

    /// Attempts to add the provided [`FilteredAccess`] to the system's current access.
    ///
    /// - If the system currently has [`None`] access, it will change this access
    ///   to [`Shared`] and add the provided [`FilteredAccess`].
    /// - If the system currently has [`Shared`] access, it will check for conflicts
    ///   with the provided [`FilteredAccess`]. If there are no conflicts, it will
    ///   add the provided [`FilteredAccess`]. If there are conflicts, it will return
    ///   an [`AccessConflicts`] error.
    /// - If the system currently has [`Exclusive`] access, it will error
    ///   with [`AccessConflicts::All`].
    ///
    /// # Errors
    ///
    /// If there are conflicts with the system's current access, an
    /// [`AccessConflicts`] error will be returned.
    ///
    /// [`None`]: SystemAccess::None
    /// [`Shared`]: SystemAccess::Shared
    /// [`Exclusive`]: SystemAccess::Exclusive
    pub fn try_add(&mut self, filtered_access: FilteredAccess) -> Result<(), AccessConflicts> {
        let conflicts = self.get_conflicts_single(&filtered_access);
        self.ensure_filtered_access(filtered_access);
        if conflicts.is_empty() {
            Ok(())
        } else {
            Err(conflicts)
        }
    }

    /// Marks the system as requiring access to certain or all components or
    /// resources in the [`World`].
    ///
    /// [`World`]: crate::world::World
    pub fn ensure_filtered_access(&mut self, filtered_access: FilteredAccess) {
        self.ensure_metadata_access();
        // If the system is `Exclusive` then we already have access to everything,
        // so we don't need to add anything.
        if let Self::Shared(access) = self {
            access.add(filtered_access);
        }
    }

    /// Marks the system as requiring access to [`World`] metadata
    /// (e.g. [`Archetypes`], [`Components`], etc.).
    ///
    /// [`World`]: crate::world::World
    /// [`Archetypes`]: crate::archetype::Archetypes
    /// [`Components`]: crate::component::Components
    pub fn ensure_metadata_access(&mut self) {
        // If the system is `Shared` or `Exclusive` then we already have access
        // to metadata, so we don't need to add anything.
        if let Self::None = self {
            *self = Self::Shared(FilteredAccessSet::new());
        }
    }

    /// Merges the provided [`SystemAccess`] into the system's current access.
    pub fn extend(&mut self, other: Self) {
        match (&mut *self, other) {
            (_, Self::None) | (Self::Exclusive, _) => {
                // Do nothing: nothing was added, or already at maximum access level
            }
            (Self::None, Self::Shared(other_access)) => {
                // Upgrade self to Shared with the other access
                *self = Self::Shared(other_access);
            }
            (Self::Shared(access), Self::Shared(other_access)) => {
                // Merge the other access into self
                access.extend(other_access);
            }
            (_, Self::Exclusive) => {
                // Upgrade self to Exclusive
                *self = Self::Exclusive;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        component::ComponentId,
        query::{FilteredAccess, FilteredAccessSet},
        system::{SystemAccess, SystemMeta},
    };

    #[test]
    fn check_default_access() {
        let mut access = SystemAccess::default();

        assert_eq!(access, SystemAccess::None);
        assert!(access.is_none());
        assert_ne!(access, SystemAccess::Shared(FilteredAccessSet::default()));
        assert!(!access.is_shared());
        assert_ne!(access, SystemAccess::Exclusive);
        assert!(!access.is_exclusive());

        access.ensure_metadata_access();

        assert!(access.is_shared());
        assert_eq!(access, SystemAccess::Shared(FilteredAccessSet::default()));
    }

    #[test]
    fn check_shared_access() {
        let mut access = SystemAccess::Shared(FilteredAccessSet::default());

        assert_ne!(access, SystemAccess::None);
        assert!(!access.is_none());
        assert!(access.is_shared());
        assert_ne!(access, SystemAccess::Exclusive);
        assert!(!access.is_exclusive());

        access.ensure_metadata_access();

        assert!(access.is_shared());

        access.ensure_filtered_access(FilteredAccess::default());

        assert!(access.is_shared());
    }

    #[test]
    fn check_exclusive_access() {
        let mut access = SystemAccess::Exclusive;

        assert_ne!(access, SystemAccess::None);
        assert!(!access.is_none());
        assert_ne!(access, SystemAccess::Shared(FilteredAccessSet::default()));
        assert!(!access.is_shared());
        assert!(access.is_exclusive());

        access.ensure_metadata_access();

        assert!(access.is_exclusive());

        access.ensure_filtered_access(FilteredAccess::default());

        assert!(access.is_exclusive());
    }

    #[test]
    fn check_compatibility() {
        let access_none = SystemAccess::None;
        let access_shared = SystemAccess::Shared({
            let mut set = FilteredAccessSet::default();
            set.add_unfiltered_component_read(ComponentId::new(1));
            set
        });
        let access_exclusive = SystemAccess::Exclusive;

        assert!(access_none.is_compatible(&access_none));
        assert!(access_none.is_compatible(&access_shared));
        assert!(access_none.is_compatible(&access_exclusive));

        assert!(access_shared.is_compatible(&access_none));
        assert!(access_shared.is_compatible(&access_shared));
        assert!(!access_shared.is_compatible(&access_exclusive));

        assert!(access_exclusive.is_compatible(&access_none));
        assert!(!access_exclusive.is_compatible(&access_shared));
        assert!(!access_exclusive.is_compatible(&access_exclusive));
    }

    #[test]
    fn conflict_reporting() {
        let access_none = SystemAccess::None;
        let access_shared = SystemAccess::Shared({
            let mut set = FilteredAccessSet::default();
            set.add_unfiltered_component_read(ComponentId::new(1));
            set
        });
        let access_exclusive = SystemAccess::Exclusive;

        assert!(access_none.get_conflicts(&access_none).is_empty());
        assert!(access_none.get_conflicts(&access_shared).is_empty());
        assert!(access_none.get_conflicts(&access_exclusive).is_empty());

        assert!(access_shared.get_conflicts(&access_none).is_empty());
        assert!(access_shared.get_conflicts(&access_shared).is_empty());
        assert_eq!(
            access_shared.get_conflicts(&access_exclusive),
            crate::query::AccessConflicts::All
        );

        assert!(access_exclusive.get_conflicts(&access_none).is_empty());
        assert_eq!(
            access_exclusive.get_conflicts(&access_shared),
            crate::query::AccessConflicts::All
        );
        assert_eq!(
            access_exclusive.get_conflicts(&access_exclusive),
            crate::query::AccessConflicts::All
        );
    }

    #[test]
    #[should_panic]
    fn require_shared_access_panics_on_exclusive() {
        let mut access = SystemAccess::Exclusive;
        access.require_shared_access::<()>(&SystemMeta::new::<()>());
    }

    #[test]
    #[should_panic]
    fn require_exclusive_access_panics_on_shared() {
        let mut access = SystemAccess::Shared(FilteredAccessSet::default());
        access.require_exclusive_access::<()>(&SystemMeta::new::<()>());
    }

    #[test]
    #[should_panic]
    fn require_exclusive_access_panics_on_exclusive() {
        let mut access = SystemAccess::Exclusive;
        access.require_exclusive_access::<()>(&SystemMeta::new::<()>());
    }

    #[test]
    fn try_add_returns_correctly() {
        let mut access = SystemAccess::None;
        let filtered_access = FilteredAccess::default();

        assert!(access.try_add(filtered_access.clone()).is_ok());
        assert!(access.is_shared());

        let mut access_shared = SystemAccess::Shared(FilteredAccessSet::default());
        assert!(access_shared.try_add(filtered_access.clone()).is_ok());

        let mut access_exclusive = SystemAccess::Exclusive;
        assert!(access_exclusive.try_add(filtered_access).is_err());
    }

    #[test]
    fn conversion_to_access_sets() {
        let access_none = SystemAccess::None;
        let access_shared = SystemAccess::Shared({
            let mut set = FilteredAccessSet::default();
            set.add_unfiltered_component_read(ComponentId::new(1));
            set
        });
        let access_exclusive = SystemAccess::Exclusive;

        assert_eq!(
            access_none.to_filtered_access_set().into_owned(),
            FilteredAccessSet::new()
        );
        assert_eq!(access_shared.to_filtered_access_set().into_owned(), {
            let mut set = FilteredAccessSet::default();
            set.add_unfiltered_component_read(ComponentId::new(1));
            set
        });
        assert_eq!(access_exclusive.to_filtered_access_set().into_owned(), {
            let mut set = FilteredAccessSet::new();
            let mut access = FilteredAccess::default();
            access.write_all();
            set.add(access);
            set
        });
    }

    #[test]
    fn extending_access() {
        let mut access = SystemAccess::default();

        let access_none = SystemAccess::None;
        let access_shared = SystemAccess::Shared({
            let mut set = FilteredAccessSet::default();
            set.add_unfiltered_component_read(ComponentId::new(1));
            set
        });
        let access_exclusive = SystemAccess::Exclusive;

        access.extend(access_none.clone());
        assert_eq!(access, SystemAccess::None);

        access.extend(access_shared.clone());
        assert_eq!(access, access_shared);

        access.extend(access_exclusive.clone());
        assert_eq!(access, access_exclusive);
    }
}
