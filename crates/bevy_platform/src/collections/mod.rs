//! Provides [`HashMap`] and [`HashSet`] from [`hashbrown`] with some customized defaults,
//! as well as re-exports of [`alloc::collections`].
//!
//! Also provides the [`HashTable`] type, which is specific to [`hashbrown`].

pub use hash_map::HashMap;
pub use hash_set::HashSet;
pub use hash_table::HashTable;
pub use hashbrown::Equivalent;

pub mod hash_map;
pub mod hash_set;
pub mod hash_table;

crate::cfg::alloc! {
    pub use alloc::collections::{VecDeque, BTreeMap, BTreeSet, BinaryHeap, LinkedList, TryReserveError, vec_deque, btree_map, btree_set, binary_heap, linked_list};
}
