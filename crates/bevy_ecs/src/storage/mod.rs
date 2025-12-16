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

use crate::component::{ComponentInfo, StorageType};
use alloc::vec::Vec;

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

/// Unsafe extension functions for `Vec<T>`
trait VecExtensions<T> {
    /// Removes an element from the vector and returns it.
    ///
    /// The removed element is replaced by the last element of the vector.
    ///
    /// This does not preserve ordering of the remaining elements, but is O(1). If you need to preserve the element order, use [`remove`] instead.
    ///
    ///
    /// # Safety
    ///
    /// All of the following must be true:
    /// - `self.len() > 1`
    /// - `index < self.len() - 1`
    ///
    /// [`remove`]: alloc::vec::Vec::remove
    /// [`swap_remove`]: alloc::vec::Vec::swap_remove
    unsafe fn swap_remove_nonoverlapping_unchecked(&mut self, index: usize) -> T;
}

impl<T> VecExtensions<T> for Vec<T> {
    #[inline]
    unsafe fn swap_remove_nonoverlapping_unchecked(&mut self, index: usize) -> T {
        // SAFETY: The caller must ensure that the element at `index` must be valid.
        // This function, and then the caller takes ownership of the value, and it cannot be
        // accessed due to the length being decremented immediately after this.
        let value = unsafe { self.as_mut_ptr().add(index).read() };
        let len = self.len();
        let base_ptr = self.as_mut_ptr();
        // SAFETY: We replace self[index] with the last element. The caller must ensure that
        // both the last element and `index` must be valid and cannot point to the same place.
        unsafe { core::ptr::copy_nonoverlapping(base_ptr.add(len - 1), base_ptr.add(index), 1) };
        // SAFETY: Upheld by caller
        unsafe { self.set_len(len - 1) };
        value
    }
}
