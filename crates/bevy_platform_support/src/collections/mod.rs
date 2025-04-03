//! Provides [`HashMap`] and [`HashSet`] from [`hashbrown`] with some customized defaults.
//!
//! Also provides the [`HashTable`] type, which is specific to [`hashbrown`].
//!
//! Note that due to the implementation details of [`hashbrown`], [`HashMap::new`] is only implemented for `HashMap<K, V, RandomState>`.
//! Whereas, Bevy exports `HashMap<K, V, FixedHasher>` as its default [`HashMap`] type, meaning [`HashMap::new`] will typically fail.
//! To bypass this issue, use [`HashMap::default`] instead.

pub use hash_map::HashMap;
pub use hash_set::HashSet;
pub use hash_table::HashTable;
pub use hashbrown::Equivalent;

pub mod hash_map;
pub mod hash_set;
pub mod hash_table;
