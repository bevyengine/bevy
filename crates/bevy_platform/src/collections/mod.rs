//! Provides [`HashMap`] and [`HashSet`] from [`hashbrown`] with some customized defaults.
//!
//! Also provides the [`HashTable`] type, which is specific to [`hashbrown`].
//!
//! Provides [`AlignedVec`] based on [rkyv::util::AlignedVec](https://github.com/rkyv/rkyv/blob/main/rkyv/src/util/alloc/aligned_vec.rs)'s implementation but the alignment can be set at runtime.

pub use aligned_vec::AlignedVec;
pub use hash_map::HashMap;
pub use hash_set::HashSet;
pub use hash_table::HashTable;
pub use hashbrown::Equivalent;

pub mod aligned_vec;
pub mod hash_map;
pub mod hash_set;
pub mod hash_table;
