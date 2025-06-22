//! Provides [`HashMap`] and [`HashSet`] from [`hashbrown`] with some customized defaults.
//!
//! Also provides the [`HashTable`] type, which is specific to [`hashbrown`].

pub use hash_map::HashMap;
pub use hash_set::HashSet;
pub use hash_table::HashTable;
pub use hashbrown::Equivalent;

pub mod hash_map;
pub mod hash_set;
pub mod hash_table;
