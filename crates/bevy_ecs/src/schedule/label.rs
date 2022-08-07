use std::{any::TypeId, hash::Hash};

use bevy_utils::{define_label, StableHashMap};
use parking_lot::{RwLock, RwLockReadGuard};

pub use bevy_ecs_macros::{AmbiguitySetLabel, RunCriteriaLabel, StageLabel, SystemLabel};

define_label!(
    /// A strongly-typed class of labels used to identify [`Stage`](crate::schedule::Stage)s.
    StageLabel,
    /// Strongly-typed identifier for a [`StageLabel`].
    StageLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify [`System`](crate::system::System)s.
    SystemLabel,
    /// Strongly-typed identifier for a [`SystemLabel`].
    SystemLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify sets of systems with intentionally ambiguous execution order.
    AmbiguitySetLabel,
    /// Strongly-typed identifier for an [`AmbiguitySetLabel`].
    AmbiguitySetLabelId,
);
define_label!(
    /// A strongly-typed class of labels used to identify [run criteria](crate::schedule::RunCriteria).
    RunCriteriaLabel,
    /// Strongly-typed identifier for a [`RunCriteriaLabel`].
    RunCriteriaLabelId,
);

//
// Implement string-labels for now.

#[doc(hidden)]
pub static STR_INTERN: TypedLabels<&'static str> = TypedLabels::new();

/// Implements a label trait for `&'static str`, the string literal type.
#[macro_export]
macro_rules! impl_string_label {
    ($label:ident) => {
        impl $label for &'static str {
            fn data(&self) -> u64 {
                $crate::schedule::STR_INTERN.intern(self)
            }
            fn fmt(idx: u64, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                let s = $crate::schedule::STR_INTERN
                    .get(idx)
                    .ok_or(std::fmt::Error)?;
                write!(f, "{s}")
            }
        }
    };
}

impl_string_label!(SystemLabel);
impl_string_label!(StageLabel);
impl_string_label!(AmbiguitySetLabel);
impl_string_label!(RunCriteriaLabel);

/// A data structure used to intern a set of labels of a single concrete type.
/// To store multiple distinct types, or generic types, try [`Labels`].
pub struct TypedLabels<T: Clone + Hash + Eq>(
    // The `IndexSet` is a hash set that preservers ordering as long as
    // you don't remove items (which we don't).
    // This allows us to have O(~1) hashing and map each entry to a stable index.
    RwLock<IndexSet<T>>,
);

/// The type returned from [`Labels::get`](Labels#method.get).
///
/// Will hold a lock on the label interner until this value gets dropped.
pub type LabelGuard<'a, L> = parking_lot::MappedRwLockReadGuard<'a, L>;

impl<T: Clone + Hash + Eq> TypedLabels<T> {
    pub const fn new() -> Self {
        Self(RwLock::new(IndexSet::with_hasher(bevy_utils::FixedState)))
    }

    /// Interns a value, if it was not already interned in this set.
    ///
    /// Returns an integer used to refer to the value later on.
    pub fn intern(&self, val: &T) -> u64 {
        use parking_lot::RwLockUpgradableReadGuard as Guard;

        // Acquire an upgradeable read lock, since we might not have to do any writing.
        let set = self.0.upgradable_read();

        // If the value is already interned, return its index.
        if let Some(idx) = set.get_index_of(val) {
            return idx as u64;
        }

        // Upgrade to a mutable lock.
        let mut set = Guard::upgrade(set);
        let (idx, _) = set.insert_full(val.clone());
        idx as u64
    }

    /// Gets a reference to the label with specified index.
    pub fn get(&self, idx: u64) -> Option<LabelGuard<T>> {
        RwLockReadGuard::try_map(self.0.read(), |set| set.get_index(idx as usize)).ok()
    }
}

/// Data structure used to intern a set of labels.
///
/// To reduce lock contention, each kind of label should have its own global instance of this type.
///
/// For generic labels, all labels with the same type constructor must go in the same instance,
/// since Rust does not allow associated statics. To deal with this, specific label types are
/// specified on the *methods*, not the type.
///
/// If you need to store a single concrete type, [`TypedLabels`] is more efficient.
pub struct Labels(
    // This type-map stores instances of `IndexSet<T>`, for any `T`.
    RwLock<TypeMap>,
);

struct TypeMap(StableHashMap<TypeId, Box<dyn std::any::Any + Send + Sync>>);

impl TypeMap {
    pub const fn new() -> Self {
        Self(StableHashMap::with_hasher(bevy_utils::FixedState))
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) -> Option<impl Drop> {
        self.0.insert(TypeId::of::<T>(), Box::new(val))
    }
    pub fn get<T: 'static>(&self) -> Option<&T> {
        let val = self.0.get(&TypeId::of::<T>())?.as_ref();
        // SAFETY: `val` was keyed with the TypeId of `T`, so we can cast it to `T`.
        Some(unsafe { &*(val as *const _ as *const T) })
    }
    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let val = self.0.get_mut(&TypeId::of::<T>())?.as_mut();
        // SAFETY: `val` was keyed with the TypeId of `T`, so we can cast it to `T`.
        Some(unsafe { &mut *(val as *mut _ as *mut T) })
    }
}

type IndexSet<T> = indexmap::IndexSet<T, bevy_utils::FixedState>;

impl Labels {
    pub const fn new() -> Self {
        Self(RwLock::new(TypeMap::new()))
    }

    /// Interns a value, if it was not already interned in this set.
    ///
    /// Returns an integer used to refer to the value later on.
    pub fn intern<L>(&self, val: &L) -> u64
    where
        L: Clone + Hash + Eq + Send + Sync + 'static,
    {
        use parking_lot::RwLockUpgradableReadGuard as Guard;

        // Acquire an upgradeable read lock, since we might not have to do any writing.
        let type_map = self.0.upgradable_read();

        if let Some(set) = type_map.get::<IndexSet<L>>() {
            // If the value is already interned, return its index.
            if let Some(idx) = set.get_index_of(val) {
                return idx as u64;
            }

            // Get mutable access to the interner.
            let mut type_map = Guard::upgrade(type_map);
            let set = type_map.get_mut::<IndexSet<L>>().unwrap();

            // Insert a clone of the value and return its index.
            let (idx, _) = set.insert_full(val.clone());
            idx as u64
        } else {
            let mut type_map = Guard::upgrade(type_map);

            // Initialize the `L` interner for the first time, including `val` in it.
            let mut set = IndexSet::default();
            let (idx, _) = set.insert_full(val.clone());
            let old = type_map.insert(set);
            // We already checked that there is no set for type `L`,
            // so let's avoid generating useless drop code for the "previous" entry.
            std::mem::forget(old);
            idx as u64
        }
    }

    /// Gets a reference to the label with specified index.
    pub fn get<L: 'static>(&self, key: u64) -> Option<LabelGuard<L>> {
        RwLockReadGuard::try_map(self.0.read(), |type_map| {
            type_map.get::<IndexSet<L>>()?.get_index(key as usize)
        })
        .ok()
    }
}
