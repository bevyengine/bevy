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

pub mod hash_map {
    //! Provides [`HashMap`], re-exported from `hashbrown` to match `std::collections::hash_map` without an `std` dependency.

    use hashbrown::hash_map as hb;

    // Re-exports to match `std::collections::hash_map`
    pub use {
        crate::hash::{DefaultHasher, RandomState},
        hb::{
            Drain, IntoIter, IntoKeys, IntoValues, Iter, IterMut, Keys, OccupiedEntry, VacantEntry,
            Values, ValuesMut,
        },
    };

    // Additional items from `hashbrown`
    pub use hb::{
        Entry, EntryRef, ExtractIf, HashMap, OccupiedError, RawEntryBuilder, RawEntryBuilderMut,
        RawEntryMut, RawOccupiedEntryMut,
    };
}

pub mod hash_set {
    //! Provides [`HashSet`], re-exported from [`hashbrown`] to match `std::collections::hash_set` without an `std` dependency.

    use hashbrown::hash_set as hb;

    // Re-exports to match `std::collections::hash_set`
    pub use hb::{Difference, Drain, Intersection, IntoIter, Iter, SymmetricDifference, Union};

    // Additional items from `hashbrown`
    pub use hb::{Entry, ExtractIf, HashSet, OccupiedEntry, VacantEntry};
}

pub mod hash_table {
    //! Provides [`HashTable`]

    pub use hashbrown::hash_table::{
        AbsentEntry, Drain, Entry, ExtractIf, HashTable, IntoIter, Iter, IterHash, IterHashMut,
        IterMut, OccupiedEntry, VacantEntry,
    };
}
