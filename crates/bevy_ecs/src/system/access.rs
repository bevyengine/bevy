use alloc::borrow::Cow;

use crate::query::{AccessConflicts, FilteredAccess, FilteredAccessSet};

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

    /// Tries to add the provided [`SystemAccess`] to the current access.
    /// If the provided [`SystemAccess`] is not compatible,
    /// this will instead return an [`Err`] with the provided [`SystemAccess`].
    ///
    /// # Errors
    ///
    /// If `self` is not compatible with `other`, this will return an [`Err`] wrapping `other`.
    pub fn try_extend(&mut self, other: Self) -> Result<(), Self> {
        if !self.is_compatible(&other) {
            return Err(other);
        }
        self.extend(other);
        Ok(())
    }

    /// Tries to add set the current access to [`Exclusive`].
    /// If the provided [`SystemAccess`] is not [`None`],
    /// this will instead return an [`Err`] with [`Exclusive`].
    ///
    /// This is equivalent to `self.try_extend(SystemAccess::Exclusive)`.
    ///
    /// # Errors
    ///
    /// If `self` is not compatible with `other`, this will return an [`Err`] wrapping `other`.
    ///
    /// [`None`]: SystemAccess::None
    /// [`Shared`]: SystemAccess::Shared
    /// [`Exclusive`]: SystemAccess::Exclusive
    pub fn try_extend_exclusive(&mut self) -> Result<(), Self> {
        self.try_extend(Self::Exclusive)
    }

    /// Tries to add access to [`World`] metadata (e.g. [`Archetypes`], [`Components`]) to the current access.
    /// If the current access is [`Exclusive`],
    /// this will instead return an [`Err`] with an empty [`Shared`].
    ///
    /// On success, `self` is guaranteed to be `Shared`.
    ///
    /// This is equivalent to `self.try_extend(SystemAccess::Shared(FilteredAccessSet::new()))`.
    ///
    /// # Errors
    ///
    /// If `self` is [`Exclusive`], this will return an [`Err`] with an empty [`Shared`].
    ///
    /// [`World`]: crate::world::World
    /// [`Archetypes`]: crate::archetype::Archetypes
    /// [`Components`]: crate::component::Components
    /// [`Shared`]: SystemAccess::Shared
    /// [`Exclusive`]: SystemAccess::Exclusive
    pub fn try_extend_metadata(&mut self) -> Result<(), Self> {
        self.try_extend(Self::Shared(FilteredAccessSet::new()))
    }

    /// Tries to add the provided [`FilteredAccess`] to the current access.
    /// If the provided [`SystemAccess`] is not compatible,
    /// this will instead return an [`Err`] with a [`Shared`] that includes the provided [`FilteredAccess`].
    ///
    /// This is equivalent to `self.try_extend(SystemAccess::Shared(other.into()))`.
    ///
    /// # Errors
    ///
    /// If `self` is not compatible with `other`, this will return an [`Err`] with a [`Shared`] wrapping `other`.
    ///
    /// [`Shared`]: SystemAccess::Shared
    pub fn try_extend_single(&mut self, other: FilteredAccess) -> Result<(), Self> {
        if let Self::None = self {
            *self = Self::Shared(FilteredAccessSet::new());
        }
        if let Self::Shared(access) = self
            && access.is_compatible_single(&other)
        {
            access.add(other);
            Ok(())
        } else {
            Err(Self::Shared(other.into()))
        }
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
        system::SystemAccess,
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

        access.try_extend_metadata().unwrap();

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

        access.try_extend_metadata().unwrap();

        assert!(access.is_shared());

        access.try_extend_single(FilteredAccess::default()).unwrap();

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

        access.try_extend_metadata().unwrap_err();

        assert!(access.is_exclusive());

        access
            .try_extend_single(FilteredAccess::default())
            .unwrap_err();

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
    fn try_extend_metadata_err_on_exclusive() {
        let mut access = SystemAccess::Exclusive;
        access.try_extend_metadata().unwrap_err();
    }

    #[test]
    fn try_extend_exclusive_err_on_shared() {
        let mut access = SystemAccess::Shared(FilteredAccessSet::default());
        access.try_extend_exclusive().unwrap_err();
    }

    #[test]
    fn try_extend_exclusive_err_on_exclusive() {
        let mut access = SystemAccess::Exclusive;
        access.try_extend_exclusive().unwrap_err();
    }

    #[test]
    fn try_extend_single_returns_correctly() {
        let mut access = SystemAccess::None;
        let filtered_access = FilteredAccess::default();

        assert!(access.try_extend_single(filtered_access.clone()).is_ok());
        assert!(access.is_shared());

        let mut access_shared = SystemAccess::Shared(FilteredAccessSet::default());
        assert!(access_shared
            .try_extend_single(filtered_access.clone())
            .is_ok());

        let mut access_exclusive = SystemAccess::Exclusive;
        assert!(access_exclusive.try_extend_single(filtered_access).is_err());
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
