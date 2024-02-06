//! This module contains the logic for bundling up resources together.
use bevy_utils::{all_tuples, TypeIdMap};
use std::any::TypeId;

use crate::{
    prelude::{Mut, World},
    system::Resource,
    world::unsafe_world_cell::UnsafeWorldCell,
};

/// Bundle of resources. With this trait we can fetch multiple resources at once from a world.
pub trait ResourceBundle {
    /// The resource bundle as it will be returned once fetched.
    type Bundle<'a>;
    /// The read-only version of this resource bundle.
    type ReadOnlyBundle<'a>;
    /// Get this resource bundle from the world.
    /// # Safety
    /// The caller must ensure that each resource in this bundle ([`Self::Bundle`]) is safe to access.
    /// For example, if `&R` is in the bundle, there should not be any valid mutable references to R.
    unsafe fn get_resource_bundle(world: UnsafeWorldCell<'_>) -> Option<Self::Bundle<'_>>;
    /// Get the read-only version of this bundle from the world.
    fn get_read_only_resource_bundle(world: &World) -> Option<Self::ReadOnlyBundle<'_>>;
    /// Return `true` if there are access conflicts within the bundle. For example, two mutable references to the same resource.
    /// If the bundled types aren't capable of tracking conflicts, this defaults to `false`.
    fn contains_access_conflicts() -> bool {
        false
    }
}

/// This isn't public and part of the [`ResourceBundle`] trait because [`BundleAccessTable`] shouldn't be public.
trait AccessConflictTracker {
    /// The [`access table`](BundleAccessTable) that tracks for access conflicts with a bundled type.
    fn access_table() -> BundleAccessTable;
    /// Return `true` if the is conflicting access within the bundle. For example, two mutable references
    /// to the same resource.
    fn contains_conflicting_access() -> bool {
        false
    }
}

#[derive(Copy, Clone)]
enum Access {
    Shared,
    Exclusive,
}

impl Access {
    fn is_conflicting(&self, other: Self) -> bool {
        !matches!((self, other), (Self::Shared, Self::Shared))
    }
}

struct BundleAccessTable {
    table: TypeIdMap<Access>,
    conflicted: bool,
}

impl BundleAccessTable {
    /// Create a new empty access table.
    fn new() -> Self {
        Self {
            table: TypeIdMap::default(),
            conflicted: false,
        }
    }

    /// Absorb the keys and values of another [`access table`](Self) and update the internal
    /// access conflict flag.
    /// # NOTE
    /// If `self` or `other` are already conflicted, the function will not absorb `other`'s
    /// table and instead just return (with the internal conflict flag set to `true`).
    fn merge_and_check_conflict(&mut self, other: BundleAccessTable) {
        if self.conflicted || other.conflicted {
            self.conflicted = true;
            return;
        }
        other
            .table
            .into_iter()
            .for_each(|(k, v)| self.insert_checked(k, v));
    }

    /// Insert a key-value pair to the table. If the insert causes an access conflict,
    /// the internal conflict flag will be set to `true`.
    /// # NOTE
    /// Even if the insertion solved an existing conflict, this will not be reflected.
    fn insert_checked(&mut self, id: TypeId, val: Access) {
        if let Some(prev_val) = self.table.insert(id, val) {
            self.conflicted |= prev_val.is_conflicting(val);
        }
    }

    /// Returns the internal access conflict flag.
    /// If this is `true`, that means that either the internal table contains an access conflict,
    /// or at one point there was an attempt to merge this table with a conflicted one.
    fn is_conflicted(&self) -> bool {
        self.conflicted
    }
}

impl<R: Resource> ResourceBundle for &R {
    type Bundle<'a> = &'a R;
    type ReadOnlyBundle<'a> = &'a R;
    unsafe fn get_resource_bundle(world: UnsafeWorldCell<'_>) -> Option<Self::Bundle<'_>> {
        world.get_resource::<R>()
    }
    fn get_read_only_resource_bundle(world: &'_ World) -> Option<Self::ReadOnlyBundle<'_>> {
        world.get_resource::<R>()
    }
}

impl<R: Resource> ResourceBundle for &mut R {
    type Bundle<'a> = Mut<'a, R>;
    type ReadOnlyBundle<'a> = &'a R;
    unsafe fn get_resource_bundle(world: UnsafeWorldCell<'_>) -> Option<Self::Bundle<'_>> {
        world.get_resource_mut::<R>()
    }
    fn get_read_only_resource_bundle(world: &World) -> Option<Self::ReadOnlyBundle<'_>> {
        world.get_resource::<R>()
    }
}

impl<R: Resource> AccessConflictTracker for &mut R {
    fn access_table() -> BundleAccessTable {
        let mut access_table = BundleAccessTable::new();
        access_table.insert_checked(TypeId::of::<R>(), Access::Exclusive);
        access_table
    }
}

impl<R: Resource> AccessConflictTracker for &R {
    fn access_table() -> BundleAccessTable {
        let mut access_table = BundleAccessTable::new();
        access_table.insert_checked(TypeId::of::<R>(), Access::Shared);
        access_table
    }
}

macro_rules! impl_conflict_tracker {
    ($($tracker:ident),*) => {
        impl <$($tracker: AccessConflictTracker),*> AccessConflictTracker for ($($tracker,)*) {
            fn contains_conflicting_access() -> bool {
                Self::access_table().is_conflicted()
            }

            fn access_table() -> BundleAccessTable {
                let mut access_table = BundleAccessTable::new();
                $(access_table.merge_and_check_conflict($tracker::access_table());)*
                access_table
            }
        }
    };
}

macro_rules! impl_resource_bundle {
    ($($bundle:ident),*) => {
        impl<$($bundle: ResourceBundle + AccessConflictTracker),*> ResourceBundle for ($($bundle,)*) {
            type Bundle<'a> = ($($bundle::Bundle<'a>,)*);
            type ReadOnlyBundle<'a> = ($($bundle::ReadOnlyBundle<'a>,)*);
            unsafe fn get_resource_bundle(world: UnsafeWorldCell<'_>) -> Option<Self::Bundle<'_>> {
                Some(($($bundle::get_resource_bundle(world)?,)*))
            }
            fn get_read_only_resource_bundle(world: &World) -> Option<Self::ReadOnlyBundle<'_>> {
                Some(($($bundle::get_read_only_resource_bundle(world)?,)*))
            }
            fn contains_access_conflicts() -> bool {
                <Self as AccessConflictTracker>::contains_conflicting_access()
            }
        }
    };
}

all_tuples!(impl_resource_bundle, 1, 15, B);
all_tuples!(impl_conflict_tracker, 1, 15, T);
