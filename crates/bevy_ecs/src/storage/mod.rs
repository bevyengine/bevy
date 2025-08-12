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

mod blob_array;
mod resource;
mod sparse_set;
mod table;
mod thin_array_ptr;

pub use resource::*;
pub use sparse_set::*;
pub use table::*;

use alloc::vec::Vec;
use crate::component::{ComponentInfo, StorageType};

/// The raw data stores of a [`World`](crate::world::World)
#[derive(Default)]
pub struct Storages {
    /// Backing storage for [`SparseSet`] components.
    /// Note that sparse sets are only present for components that have been spawned or have had a relevant bundle registered.
    pub sparse_sets: SparseSets,
    /// Backing storage for [`Table`] components.
    pub tables: Tables,
    /// Backing storage for resources.
    pub resources: Resources<true>,
    /// Backing storage for `!Send` resources.
    pub non_send_resources: Resources<false>,
}

impl Storages {
    /// ensures that the component has its necessary storage initialize.
    pub fn prepare_component(&mut self, component: &ComponentInfo) {
        match component.storage_type() {
            StorageType::Table => {
                // table needs no preparation
            }
            StorageType::SparseSet => {
                self.sparse_sets.get_or_insert(component);
            }
        }
    }
}

struct AbortOnPanic;

impl Drop for AbortOnPanic {
    fn drop(&mut self) {
        // Panicking while unwinding will force an abort.
        panic!("Aborting due to allocator error");
    }
}

trait VecExtensions<T> {
    unsafe fn swap_remove_nonoverlapping_unchecked(&mut self, index: usize);
}

impl<T> VecExtensions<T> for Vec<T> {
    unsafe fn swap_remove_nonoverlapping_unchecked(&mut self, index: usize) {
        let len = self.len();
        // We replace self[index] with the last element. Note that if the
        // bounds check above succeeds there must be a last element (which
        // can be self[index] itself).
        let base_ptr = self.as_mut_ptr();
        core::ptr::copy_nonoverlapping(base_ptr.add(len - 1), base_ptr.add(index), 1);
        self.set_len(len - 1);
    }
}