//! Storage layouts for ECS data.
//!
//! This module implements the low-level collections that store data in a [`World`]. These all offer minimal and often
//! unsafe APIs, and have been made `pub` primarily for debugging and monitoring purposes.
//!
//! # Fetching Storages
//! Each of the below data stores can be fetched via [`Storages`], which can be fetched from a
//! [`World`] via [`World::storages`]. It exposes a top level container for each class of storage within
//! ECS:
//!
//!  - [`Tables`] - columnar contiguous blocks of memory, optimized for fast iteration.
//!  - [`SparseSets`] - sparse `HashMap`-like mappings from entities to components, optimized for random
//!    lookup and regular insertion/removal of components.
//!  - [`Resources`] - singleton storage for the resources in the world
//!
//! # Safety
//! To avoid trivially unsound use of the APIs in this module, it is explicitly impossible to get a mutable
//! reference to [`Storages`] from [`World`], and none of the types publicly expose a mutable interface.
//!
//! [`World`]: crate::world::World
//! [`World::storages`]: crate::world::World::storages

mod blob_vec;
mod resource;
mod sparse_set;
mod table;

pub use resource::*;
pub use sparse_set::*;
pub use table::*;

use crate::component::ComponentId;

/// The raw data stores of a [World](crate::world::World)
#[derive(Default)]
pub struct Storages {
    /// Backing storage for [`SparseSet`] components.
    pub sparse_sets: SparseSets,
    /// Backing storage for [`Table`] components.
    pub tables: Tables,
    /// Backing storage for resources.
    pub resources: Resources<true>,
    /// Backing storage for `!Send` resources.
    pub non_send_resources: Resources<false>,
}

/// Provides interior-mutable access to a world's internal data storages.
///
/// Any instance of this type is associated with a set of world data that
/// it is allowed to access. This should be described in the documentation
/// of wherever you obtained the `UnsafeStorages`.
///
/// For instance, if you originally obtained it from a system running on
/// a multi-threaded executor, then you are only allowed to access data
/// that has been registered in the system's `archetype_component_access`.
/// If you originally obtained an `UnsafeStorages` from an `&World`,
/// then you have read-only access to the entire world.
///
/// Accessing world data that do not have access to, or mutably accessing
/// data that you only have read-access to, is considered undefined behavior.
pub struct UnsafeStorages<'a>(&'a Storages);

impl<'a> UnsafeStorages<'a> {
    pub(crate) fn new(storages: &'a Storages) -> Self {
        Self(storages)
    }

    /// Gets a view into the [`ComponentSparseSet`] associated with `component_id`,
    /// if one exists.
    pub fn get_sparse_set(self, component_id: ComponentId) -> Option<UnsafeComponentSparseSet<'a>> {
        self.0
            .sparse_sets
            .get(component_id)
            .map(|sparse_set| UnsafeComponentSparseSet { sparse_set })
    }

    /// Gets a view into the [`Table`] associated with `id`, if one exists.
    pub fn get_table(self, id: TableId) -> Option<UnsafeTable<'a>> {
        self.0.tables.get(id).map(|table| UnsafeTable { table })
    }

    /// Gets access to the resource's data store, if it is registered.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] that self was obtained from has permission to access the resource
    /// - no mutable reference to the resource exists at the same time
    ///
    /// [`UnsafeWorldCell`]: crate::world::unsafe_world_cell::UnsafeWorldCell
    pub unsafe fn get_resource(self, component_id: ComponentId) -> Option<&'a ResourceData<true>> {
        self.0.resources.get(component_id)
    }

    /// Gets access to the specified non-send resource's data store, if it is registered.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] that self was obtained from has permission to access the resource
    /// - no mutable reference to the resource exists at the same time
    ///
    /// [`UnsafeWorldCell`]: crate::world::unsafe_world_cell::UnsafeWorldCell
    pub unsafe fn get_non_send_resource(
        self,
        component_id: ComponentId,
    ) -> Option<&'a ResourceData<false>> {
        self.0.non_send_resources.get(component_id)
    }
}
