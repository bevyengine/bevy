use bevy_utils::hashbrown::hash_set::IntoIter;
use bevy_utils::HashSet;
use std::any::{Any, TypeId};

/// A filter used to control which types can be added to a [`DynamicScene`].
///
/// This scene filter _can_ be used more generically to represent a filter for any given type;
/// however, note that its intended usage with `DynamicScene` only considers [components] and [resources].
/// Adding types that are not a component or resource will have no effect when used with `DynamicScene`.
///
/// [`DynamicScene`]: crate::DynamicScene
/// [components]: bevy_ecs::prelude::Component
/// [resources]: bevy_ecs::prelude::Resource
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum SceneFilter {
    /// Represents an unset filter.
    ///
    /// This is the equivalent of an empty [`Denylist`] or an [`Allowlist`] containing every type—
    /// essentially, all types are permissible.
    ///
    /// [Allowing] a type will convert this filter to an `Allowlist`.
    /// Similarly, [denying] a type will convert this filter to a `Denylist`.
    ///
    /// [`Denylist`]: SceneFilter::Denylist
    /// [`Allowlist`]: SceneFilter::Allowlist
    /// [Allowing]: SceneFilter::allow
    /// [denying]: SceneFilter::deny
    #[default]
    Unset,
    /// Contains the set of permitted types by their [`TypeId`].
    ///
    /// Types not contained within this set should not be allowed to be saved to an associated [`DynamicScene`].
    ///
    /// [`DynamicScene`]: crate::DynamicScene
    Allowlist(HashSet<TypeId>),
    /// Contains the set of prohibited types by their [`TypeId`].
    ///
    /// Types contained within this set should not be allowed to be saved to an associated [`DynamicScene`].
    ///
    /// [`DynamicScene`]: crate::DynamicScene
    Denylist(HashSet<TypeId>),
}

impl SceneFilter {
    /// Creates a filter where all types are allowed.
    ///
    /// This is the equivalent of creating an empty [`Denylist`].
    ///
    /// [`Denylist`]: SceneFilter::Denylist
    pub fn allow_all() -> Self {
        Self::Denylist(HashSet::new())
    }

    /// Creates a filter where all types are denied.
    ///
    /// This is the equivalent of creating an empty [`Allowlist`].
    ///
    /// [`Allowlist`]: SceneFilter::Allowlist
    pub fn deny_all() -> Self {
        Self::Allowlist(HashSet::new())
    }

    /// Allow the given type, `T`.
    ///
    /// If this filter is already set as a [`Denylist`],
    /// then the given type will be removed from the denied set.
    ///
    /// If this filter is [`Unset`], then it will be completely replaced by a new [`Allowlist`].
    ///
    /// [`Denylist`]: SceneFilter::Denylist
    /// [`Unset`]: SceneFilter::Unset
    /// [`Allowlist`]: SceneFilter::Allowlist
    pub fn allow<T: Any>(&mut self) -> &mut Self {
        self.allow_by_id(TypeId::of::<T>())
    }

    /// Allow the given type.
    ///
    /// If this filter is already set as a [`Denylist`],
    /// then the given type will be removed from the denied set.
    ///
    /// If this filter is [`Unset`], then it will be completely replaced by a new [`Allowlist`].
    ///
    /// [`Denylist`]: SceneFilter::Denylist
    /// [`Unset`]: SceneFilter::Unset
    /// [`Allowlist`]: SceneFilter::Allowlist
    pub fn allow_by_id(&mut self, type_id: TypeId) -> &mut Self {
        match self {
            Self::Unset => {
                *self = Self::Allowlist(HashSet::from([type_id]));
            }
            Self::Allowlist(list) => {
                list.insert(type_id);
            }
            Self::Denylist(list) => {
                list.remove(&type_id);
            }
        }
        self
    }

    /// Deny the given type, `T`.
    ///
    /// If this filter is already set as an [`Allowlist`],
    /// then the given type will be removed from the allowed set.
    ///
    /// If this filter is [`Unset`], then it will be completely replaced by a new [`Denylist`].
    ///
    /// [`Allowlist`]: SceneFilter::Allowlist
    /// [`Unset`]: SceneFilter::Unset
    /// [`Denylist`]: SceneFilter::Denylist
    pub fn deny<T: Any>(&mut self) -> &mut Self {
        self.deny_by_id(TypeId::of::<T>())
    }

    /// Deny the given type.
    ///
    /// If this filter is already set as an [`Allowlist`],
    /// then the given type will be removed from the allowed set.
    ///
    /// If this filter is [`Unset`], then it will be completely replaced by a new [`Denylist`].
    ///
    /// [`Allowlist`]: SceneFilter::Allowlist
    /// [`Unset`]: SceneFilter::Unset
    /// [`Denylist`]: SceneFilter::Denylist
    pub fn deny_by_id(&mut self, type_id: TypeId) -> &mut Self {
        match self {
            Self::Unset => *self = Self::Denylist(HashSet::from([type_id])),
            Self::Allowlist(list) => {
                list.remove(&type_id);
            }
            Self::Denylist(list) => {
                list.insert(type_id);
            }
        }
        self
    }

    /// Returns true if the given type, `T`, is allowed by the filter.
    ///
    /// If the filter is [`Unset`], this will always return `true`.
    ///
    /// [`Unset`]: SceneFilter::Unset
    pub fn is_allowed<T: Any>(&self) -> bool {
        self.is_allowed_by_id(TypeId::of::<T>())
    }

    /// Returns true if the given type is allowed by the filter.
    ///
    /// If the filter is [`Unset`], this will always return `true`.
    ///
    /// [`Unset`]: SceneFilter::Unset
    pub fn is_allowed_by_id(&self, type_id: TypeId) -> bool {
        match self {
            Self::Unset => true,
            Self::Allowlist(list) => list.contains(&type_id),
            Self::Denylist(list) => !list.contains(&type_id),
        }
    }

    /// Returns true if the given type, `T`, is denied by the filter.
    ///
    /// If the filter is [`Unset`], this will always return `false`.
    ///
    /// [`Unset`]: SceneFilter::Unset
    pub fn is_denied<T: Any>(&self) -> bool {
        self.is_denied_by_id(TypeId::of::<T>())
    }

    /// Returns true if the given type is denied by the filter.
    ///
    /// If the filter is [`Unset`], this will always return `false`.
    ///
    /// [`Unset`]: SceneFilter::Unset
    pub fn is_denied_by_id(&self, type_id: TypeId) -> bool {
        !self.is_allowed_by_id(type_id)
    }

    /// Returns an iterator over the items in the filter.
    ///
    /// If the filter is [`Unset`], this will return an empty iterator.
    ///
    /// [`Unset`]: SceneFilter::Unset
    pub fn iter(&self) -> Box<dyn ExactSizeIterator<Item = &TypeId> + '_> {
        match self {
            Self::Unset => Box::new(core::iter::empty()),
            Self::Allowlist(list) | Self::Denylist(list) => Box::new(list.iter()),
        }
    }

    /// Returns the number of items in the filter.
    ///
    /// If the filter is [`Unset`], this will always return a length of zero.
    ///
    /// [`Unset`]: SceneFilter::Unset
    pub fn len(&self) -> usize {
        match self {
            Self::Unset => 0,
            Self::Allowlist(list) | Self::Denylist(list) => list.len(),
        }
    }

    /// Returns true if there are zero items in the filter.
    ///
    /// If the filter is [`Unset`], this will always return `true`.
    ///
    /// [`Unset`]: SceneFilter::Unset
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Unset => true,
            Self::Allowlist(list) | Self::Denylist(list) => list.is_empty(),
        }
    }
}

impl IntoIterator for SceneFilter {
    type Item = TypeId;
    type IntoIter = IntoIter<TypeId>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Unset => HashSet::new().into_iter(),
            Self::Allowlist(list) | Self::Denylist(list) => list.into_iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_set_list_type_if_none() {
        let mut filter = SceneFilter::Unset;
        filter.allow::<i32>();
        assert!(matches!(filter, SceneFilter::Allowlist(_)));

        let mut filter = SceneFilter::Unset;
        filter.deny::<i32>();
        assert!(matches!(filter, SceneFilter::Denylist(_)));
    }

    #[test]
    fn should_add_to_list() {
        let mut filter = SceneFilter::default();
        filter.allow::<i16>();
        filter.allow::<i32>();
        assert_eq!(2, filter.len());
        assert!(filter.is_allowed::<i16>());
        assert!(filter.is_allowed::<i32>());

        let mut filter = SceneFilter::default();
        filter.deny::<i16>();
        filter.deny::<i32>();
        assert_eq!(2, filter.len());
        assert!(filter.is_denied::<i16>());
        assert!(filter.is_denied::<i32>());
    }

    #[test]
    fn should_remove_from_list() {
        let mut filter = SceneFilter::default();
        filter.allow::<i16>();
        filter.allow::<i32>();
        filter.deny::<i32>();
        assert_eq!(1, filter.len());
        assert!(filter.is_allowed::<i16>());
        assert!(!filter.is_allowed::<i32>());

        let mut filter = SceneFilter::default();
        filter.deny::<i16>();
        filter.deny::<i32>();
        filter.allow::<i32>();
        assert_eq!(1, filter.len());
        assert!(filter.is_denied::<i16>());
        assert!(!filter.is_denied::<i32>());
    }
}
