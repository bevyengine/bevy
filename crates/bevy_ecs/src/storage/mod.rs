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
pub struct UnsafeStorages<'a> {
    pub sparse_sets: UnsafeSparseSets<'a>,
    pub tables: UnsafeTables<'a>,
    pub resources: UnsafeResources<'a, true>,
    pub non_send_resources: UnsafeResources<'a, false>,
}

impl<'a> UnsafeStorages<'a> {
    pub(crate) fn new(storages: &'a Storages) -> Self {
        Self {
            sparse_sets: UnsafeSparseSets {
                sparse_sets: &storages.sparse_sets,
            },
            tables: UnsafeTables {
                tables: &storages.tables,
            },
            resources: UnsafeResources {
                resources: &storages.resources,
            },
            non_send_resources: UnsafeResources {
                resources: &storages.non_send_resources,
            },
        }
    }
}

#[derive(Clone, Copy)]
pub struct UnsafeSparseSets<'a> {
    sparse_sets: &'a SparseSets,
}

impl<'a> UnsafeSparseSets<'a> {
    pub fn get(self, component_id: ComponentId) -> Option<UnsafeComponentSparseSet<'a>> {
        self.sparse_sets
            .get(component_id)
            .map(|sparse_set| UnsafeComponentSparseSet { sparse_set })
    }
}

#[derive(Clone, Copy)]
pub struct UnsafeTables<'a> {
    tables: &'a Tables,
}

impl<'a> UnsafeTables<'a> {
    pub fn get(self, id: TableId) -> Option<UnsafeTable<'a>> {
        self.tables.get(id).map(|table| UnsafeTable { table })
    }
}
