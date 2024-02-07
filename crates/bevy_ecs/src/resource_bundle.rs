//! This module contains the logic for bundling up resources together.
use bevy_utils::{all_tuples, TypeIdSet};
use std::any::TypeId;

use crate::{
    prelude::Mut,
    system::{Res, Resource},
    world::unsafe_world_cell::UnsafeWorldCell,
};

/// Bundle of resources. With this trait we can fetch multiple resources at once from a world.
pub trait ResourceBundle {
    /// Write access to the resources of this resource bundle. This type should provide write access, like `&mut R` or `ResMut<R>`
    type WriteAccess<'a>;
    /// Read-only access to the resources of this resource bundle. This type should provide read-only access, like `&R` or `Res<R>`
    type ReadOnlyAccess<'a>;
    /// Get write access to the resources in the bundle.
    ///
    /// # Safety
    /// The caller must ensure that each resource in this bundle is safe to access mutably.
    /// For example, if `R` is in the bundle, there should not be any other valid references to R.
    unsafe fn fetch_write_access(world: UnsafeWorldCell<'_>) -> Option<Self::WriteAccess<'_>>;
    /// Get read-only access to the resources in this bundle.
    ///
    /// # Safety
    /// The caller must it is valid to get read-only access to each of the resources in this bundle.
    /// For example, if `R` is in the bundle, there should not be any valid *mutable* references to R.
    unsafe fn fetch_read_only(world: UnsafeWorldCell<'_>) -> Option<Self::ReadOnlyAccess<'_>>;
    /// Return `true` if there are access conflicts within the bundle. In other words, this returns `true`
    /// if and only a resource appears twice in the bundle.
    fn contains_access_conflicts() -> bool {
        false
    }
}

/// This isn't public and part of the [`ResourceBundle`] trait because [`BundleAccessTable`] shouldn't be public.
trait AccessConflictTracker {
    /// Merge the internal [`access table`](BundleAccessTable) with some external one.
    fn merge_with(other: &mut BundleAccessTable);
    /// Return `true` if there is conflicting access within the bundle. For example, two mutable references
    /// to the same resource.
    fn contains_conflicting_access() -> bool {
        false
    }
}

/// Type to keep track which resources the [`ResourceBundle`] accesses.
struct BundleAccessTable {
    table: TypeIdSet,
    conflicted: bool,
}

impl BundleAccessTable {
    /// Create a new empty access table.
    fn new_empty_unconflicted() -> Self {
        Self {
            table: TypeIdSet::default(),
            conflicted: false,
        }
    }

    /// Insert a key-value pair to the table. If the insert causes an access conflict, the internal conflict flag will be set to `true`.
    fn insert_checked(&mut self, id: TypeId) {
        self.conflicted |= !self.table.insert(id);
    }

    /// Returns the internal access conflict flag.
    /// If this is `true`, that means that either the internal table contains an access conflict,
    /// or at one point there was an attempt to merge this table with a conflicted one.
    fn is_conflicted(&self) -> bool {
        self.conflicted
    }
}

impl<R: Resource> ResourceBundle for R {
    type WriteAccess<'a> = Mut<'a, R>;
    type ReadOnlyAccess<'a> = &'a R;
    unsafe fn fetch_write_access(world: UnsafeWorldCell<'_>) -> Option<Self::WriteAccess<'_>> {
        world.get_resource_mut::<R>()
    }
    unsafe fn fetch_read_only(world: UnsafeWorldCell<'_>) -> Option<Self::ReadOnlyAccess<'_>> {
        world.get_resource::<R>()
    }
}

// Allow the user to get `Res` access to a resource as well.
// But getting `ResMut` isn't supported attow.
impl<R: Resource> ResourceBundle for Res<'_, R> {
    type WriteAccess<'a> = Mut<'a, R>;
    type ReadOnlyAccess<'a> = Res<'a, R>;
    unsafe fn fetch_write_access(world: UnsafeWorldCell<'_>) -> Option<Self::WriteAccess<'_>> {
        world.get_resource_mut::<R>()
    }
    unsafe fn fetch_read_only(world: UnsafeWorldCell<'_>) -> Option<Self::ReadOnlyAccess<'_>> {
        world.get_resource_ref::<R>()
    }
}

impl<R: Resource> AccessConflictTracker for Res<'_, R> {
    fn merge_with(other: &mut BundleAccessTable) {
        other.insert_checked(TypeId::of::<R>());
    }
}

impl<R: Resource> AccessConflictTracker for R {
    fn merge_with(other: &mut BundleAccessTable) {
        other.insert_checked(TypeId::of::<R>());
    }
}

macro_rules! impl_conflict_tracker {
    ($($tracker:ident),*) => {
        impl <$($tracker: AccessConflictTracker),*> AccessConflictTracker for ($($tracker,)*) {
            fn contains_conflicting_access() -> bool {
                let mut tmp_table = BundleAccessTable::new_empty_unconflicted();
                Self::merge_with(&mut tmp_table);
                tmp_table.is_conflicted()
            }

            fn merge_with(other: &mut BundleAccessTable) {
                $($tracker::merge_with(other));*
            }
        }
    };
}

macro_rules! impl_resource_bundle {
    ($($bundle:ident),*) => {
        impl<$($bundle: ResourceBundle + AccessConflictTracker),*> ResourceBundle for ($($bundle,)*) {
            type WriteAccess<'a> = ($($bundle::WriteAccess<'a>,)*);
            type ReadOnlyAccess<'a> = ($($bundle::ReadOnlyAccess<'a>,)*);
            unsafe fn fetch_write_access(world: UnsafeWorldCell<'_>) -> Option<Self::WriteAccess<'_>> {
                Some(($($bundle::fetch_write_access(world)?,)*))
            }
            unsafe fn fetch_read_only(world: UnsafeWorldCell<'_>) -> Option<Self::ReadOnlyAccess<'_>> {
                Some(($($bundle::fetch_read_only(world)?,)*))
            }
            fn contains_access_conflicts() -> bool {
                <Self as AccessConflictTracker>::contains_conflicting_access()
            }
        }
    };
}

all_tuples!(impl_resource_bundle, 1, 15, B);
all_tuples!(impl_conflict_tracker, 1, 15, T);
