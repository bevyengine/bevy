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

/// Data structure used to intern a set of labels for a given type.
pub struct Labels<L>(RwLock<StableHashMap<u64, L>>);

/// The type returned from [`Labels::get`](Labels#method.get).
///
/// Will hold a lock on the string interner for type `L`, until this value gets dropped.
pub type LabelGuard<'a, L> = parking_lot::MappedRwLockReadGuard<'a, L>;

impl<L> Labels<L> {
    #[inline]
    pub const fn new() -> Self {
        Self(RwLock::new(StableHashMap::with_hasher(
            bevy_utils::FixedState,
        )))
    }

    /// Interns a value, if it was not already interned in this set.
    pub fn intern(&self, key: u64, f: impl FnOnce() -> L) {
        use parking_lot::RwLockUpgradableReadGuard as Guard;

        // Acquire an upgradeable read lock, since we might not have to do any writing.
        let map = self.0.upgradable_read();
        if map.contains_key(&key) {
            return;
        }
        // Upgrade the lock to a mutable one.
        let mut map = Guard::upgrade(map);
        let old = map.insert(key, f());

        // We already checked that the entry was empty, so make sure
        // useless drop code doesn't get inserted.
        debug_assert!(old.is_none());
        std::mem::forget(old);
    }

    /// Allows one to peek at an interned label and execute code,
    /// optionally returning a value.
    ///
    /// Returns `None` if there is no interned label with that key.
    pub fn scope<U>(&self, key: u64, f: impl FnOnce(&L) -> U) -> Option<U> {
        self.0.read().get(&key).map(f)
    }

    pub fn get(&self, key: u64) -> Option<LabelGuard<L>> {
        RwLockReadGuard::try_map(self.0.read(), |map| map.get(&key)).ok()
    }
}
