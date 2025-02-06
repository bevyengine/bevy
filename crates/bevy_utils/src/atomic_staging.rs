//! Provides an abstracted system for staging modifications to data structures that rarely change. See [`StageOnWrite`] as a starting point.

use core::ops::Deref;

use bevy_platform_support::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Signifies that this type represents staged changes to [`Cold`](Self::Cold).
pub trait StagedChanges: Default {
    /// The more compact data structure that these changes compact into.
    type Cold;

    /// This applies these changes to the passed [`Cold`](Self::Cold). When this is done, there should be no more staged changes, and [`any_staged`](Self::any_staged) should be false.
    fn apply_staged(&mut self, storage: &mut Self::Cold);

    /// Returns true if and only if there are staged changes that could be applied.
    fn any_staged(&self) -> bool;
}

/// A trait that signifies that it holds an immutable reference to a cold type (ie. [`StagedChanges::Cold`]).
pub trait ColdStorage<T: StagedChanges>: Deref<Target = T::Cold> {}

/// A struct that allows staging changes while reading from cold storage. Generally, staging changes should be implemented on this type.
pub struct Stager<'a, T: StagedChanges> {
    /// The storage that is read optimized.
    pub cold: &'a T::Cold,
    /// The staged changes.
    pub staged: &'a mut T,
}

/// A struct that allows accessing changes while reading from cold storage. Generally, reading data should be implemented on this type.
#[derive(Copy)]
pub struct StagedRef<'a, T: StagedChanges> {
    /// The storage that is read optimized.
    pub cold: &'a T::Cold,
    /// The staged changes.
    pub staged: &'a T,
}

/// A locked version of [`Stager`]. Use this to hold a lock guard while using [`StagerLocked::as_stager`] or similar.
pub struct StagerLocked<'a, T: StagedChanges, C: ColdStorage<T>> {
    cold: C,
    staged: RwLockWriteGuard<'a, T>,
}

/// A locked version of [`StagedRef`] Use this to hold a lock guard while using [`StagerLocked::as_staged_ref`].
pub struct StagedRefLocked<'a, T: StagedChanges, C: ColdStorage<T>> {
    cold: C,
    staged: RwLockReadGuard<'a, T>,
}

/// A general purpose enum for representing data that may or may not need to be staged.
pub enum MaybeStaged<C, S> {
    /// There is no staging necessary.
    Cold(C),
    /// There is staging necessary.
    Staged(S),
}

/// A struct that allows read-optimized operations while still allowing mutation. When mutattions are made,
/// they are staged. Then, at user-defined times, they are applied to the read-optimized storage. This allows muttations
/// through [`RwLock`]s without needing to constantly lock old or cold data.
///
/// This is not designed for atomic use (ie. in an [`Arc`]). See [`AtomicStageOnWrite`] for that.
#[derive(Default)]
pub struct StageOnWrite<T: StagedChanges> {
    /// Cold data is read optimized.
    cold: T::Cold,
    /// Staged data stores recent modifications to cold. It's [`RwLock`] coordinates mutations.
    staged: RwLock<T>,
}

#[derive(Default)]
struct AtomicStageOnWriteInner<T: StagedChanges> {
    /// Cold data is read optimized. This lives behind a [`RwLock`], but it is only written to for applying changes in
    /// a non-blocking way. In other worlds this locks, but almost never blocks. (It can technically block if a thread
    /// tries to read from it while it is having changes applied, but that is extremely rare.)
    cold: RwLock<T::Cold>,
    /// Staged data stores recent modifications to cold. It's [`RwLock`] coordinates mutations.
    staged: RwLock<T>,
}

/// A version of [`StageOnWrite`] designed for atomic use. See [`StageOnWrite`] for details.
///
/// This type includes a baked in [`Arc`], so it can be shared similarly. Many of it's methods take `&mut self` even though
/// it doesn't technically need the mutation. This is done to signify that the methods involve a state change of the data and to prevent deadlocks.
/// Because everything that involves a write lock requires `&mut self`, it is impossible to deadlock because doing so would require another lock guard
/// with the same lifetime, which rust will complaine about. If you do not want this behavior, see [`AtomicStageOnWriteInner`].
#[derive(Clone)]
pub struct AtomicStageOnWrite<T: StagedChanges>(Arc<AtomicStageOnWriteInner<T>>);

impl<T: StagedChanges> StageOnWrite<T> {
    /// Constructs a new [`StageOnWrite`] with the given value and no staged changes.
    pub fn new(value: T::Cold) -> Self {
        Self {
            cold: value,
            staged: RwLock::default(),
        }
    }

    /// Gets the inner cold data if it is safe. If [`any_staged`](Self::any_staged) is known to be false, this can be safely unwrapped.
    #[inline]
    pub fn full(&mut self) -> Option<&mut T::Cold> {
        if self.staged.get_mut().unwrap().any_staged() {
            None
        } else {
            Some(&mut self.cold)
        }
    }

    /// Applies any staged changes before returning the full value with all changes applied.
    /// Immediately after this, [`any_staged`](Self::any_staged) will be false.
    #[inline]
    pub fn apply_staged_for_full(&mut self) -> &mut T::Cold {
        let staged = self.staged.get_mut().unwrap();
        if staged.any_staged() {
            staged.apply_staged(&mut self.cold);
        }
        &mut self.cold
    }

    /// Returns true if and only if there are staged changes that could be applied.
    /// If you only have a immutable reference, consider using [`read_scope_locked`] with [`StagedChanges::any_staged`].
    #[inline]
    pub fn any_staged(&mut self) -> bool {
        self.staged.get_mut().unwrap().any_staged()
    }

    /// Constructs a [`Stager`] that will stage changes.
    #[inline]
    pub fn stage(&mut self) -> Stager<'_, T> {
        Stager {
            cold: &mut self.cold,
            staged: self.staged.get_mut().unwrap(),
        }
    }

    /// Constructs a [`StagerLocked`], locking internally.
    ///
    /// # Deadlocks
    ///
    /// This deadlocks if there are any other lock guards on this thread for this value.
    #[inline]
    pub fn stage_lock(&self) -> StagerLocked<'_, T, &T::Cold> {
        StagerLocked {
            cold: &self.cold,
            staged: self.staged.write().unwrap(),
        }
    }

    /// Constructs a [`Stager`] that will stage changes.
    #[inline]
    pub fn read(&mut self) -> StagedRef<'_, T> {
        StagedRef {
            cold: &self.cold,
            staged: self.staged.get_mut().unwrap(),
        }
    }

    /// Constructs a [`StagedRefLocked`], locking internally.
    ///
    /// # Deadlocks
    ///
    /// This deadlocks if there are any write lock guards on this thread for this value.
    #[inline]
    pub fn read_lock(&self) -> StagedRefLocked<'_, T, &T::Cold> {
        StagedRefLocked {
            cold: &self.cold,
            staged: self.staged.read().unwrap(),
        }
    }

    /// Runs different logic depending on if additional changes are already staged.
    /// This can be faster than greedily applying staged changes if there are already staged changes.
    pub fn maybe_stage<C, S>(
        &mut self,
        for_full: impl FnOnce(&mut T::Cold) -> C,
        for_staged: impl FnOnce(&mut Stager<T>) -> S,
    ) -> MaybeStaged<C, S> {
        let staged = self.staged.get_mut().unwrap();
        let cold = &mut self.cold;
        if staged.any_staged() {
            MaybeStaged::Staged(for_staged(&mut Stager { cold, staged }))
        } else {
            MaybeStaged::Cold(for_full(cold))
        }
    }

    /// Easily run a stager function to stage changes.
    ///
    /// # Deadlocks
    ///
    /// This deadlocks if there are any other lock guards on this thread for this value.
    #[inline]
    pub fn stage_scope_locked<R>(&self, f: impl FnOnce(&mut Stager<T>) -> R) -> R {
        f(&mut self.stage_lock().as_stager())
    }

    /// Easily run a [`StagedRef`] function.
    ///
    /// # Deadlocks
    ///
    /// This deadlocks if there are any write lock guards on this thread for this value.
    #[inline]
    pub fn read_scope_locked<R>(&self, f: impl FnOnce(&StagedRef<T>) -> R) -> R {
        f(&self.read_lock().as_staged_ref())
    }
}

impl<T: StagedChanges> AtomicStageOnWrite<T> {
    /// Constructs a new [`AtomicStageOnWrite`] with the given value and no staged changes.
    pub fn new(value: T::Cold) -> Self {
        Self(Arc::new(AtomicStageOnWriteInner {
            cold: RwLock::new(value),
            staged: RwLock::default(),
        }))
    }

    /// Gets the inner cold data if it is safe. If [`any_staged`](Self::any_staged) is known to be false, this can be safely unwrapped.
    ///
    /// Note that this **Blocks**, so generally prefer [`full_non_blocking`](Self::full_non_blocking).
    #[inline]
    pub fn full_locked(&mut self) -> Option<RwLockWriteGuard<'_, T::Cold>> {
        if self.0.staged.read().unwrap().any_staged() {
            None
        } else {
            Some(self.0.cold.write().unwrap())
        }
    }

    /// Applies any staged changes before returning the full value with all changes applied.
    /// Immediately after this, [`any_staged`](Self::any_staged) will be false.
    ///
    /// Note that this **Blocks**, so generally prefer [`apply_staged_for_full_non_blocking`](Self::apply_staged_for_full_non_blocking).
    #[inline]
    pub fn apply_staged_for_full_locked(&mut self) -> RwLockWriteGuard<'_, T::Cold> {
        let mut staged = self.0.staged.write().unwrap();
        let mut cold = self.0.cold.write().unwrap();
        if staged.any_staged() {
            staged.apply_staged(&mut cold);
        }
        cold
    }

    /// Gets the inner cold data if it is safe.
    #[inline]
    pub fn full_non_blocking(&mut self) -> Option<RwLockWriteGuard<'_, T::Cold>> {
        let staged = self.0.staged.try_read().ok()?;
        if staged.any_staged() {
            None
        } else {
            self.0.cold.try_write().ok()
        }
    }

    /// Applies any staged changes before returning the full value with all changes applied.
    #[inline]
    pub fn apply_staged_for_full_non_blocking(&mut self) -> Option<RwLockWriteGuard<'_, T::Cold>> {
        let mut cold = self.0.cold.try_write().ok()?;
        match self.0.staged.try_write() {
            Ok(mut staged) => {
                if staged.any_staged() {
                    staged.apply_staged(&mut cold);
                }
                Some(cold)
            }
            Err(_) => {
                let staged = self.0.staged.read().unwrap();
                if staged.any_staged() {
                    None
                } else {
                    Some(cold)
                }
            }
        }
    }

    /// If possible applies any staged changes. Returns true if it can guarantee there are no more staged changes.
    #[inline]
    pub fn apply_staged_non_blocking(&mut self) -> bool {
        let Ok(mut staged) = self.0.staged.try_write() else {
            return false;
        };
        if staged.any_staged() {
            let Ok(mut cold) = self.0.cold.try_write() else {
                return false;
            };
            staged.apply_staged(&mut cold);
            true
        } else {
            false
        }
    }

    /// Returns true if and only if there are staged changes that could be applied.
    #[inline]
    pub fn any_staged(&self) -> bool {
        self.0.staged.read().unwrap().any_staged()
    }

    /// Constructs a [`StagerLocked`], locking internally.
    #[inline]
    pub fn stage_lock(&mut self) -> StagerLocked<'_, T, RwLockReadGuard<'_, T::Cold>> {
        StagerLocked {
            cold: self.0.cold.read().unwrap(),
            staged: self.0.staged.write().unwrap(),
        }
    }

    /// Constructs a [`StagedRefLocked`], locking internally.
    #[inline]
    pub fn read_lock(&self) -> StagedRefLocked<'_, T, RwLockReadGuard<'_, T::Cold>> {
        StagedRefLocked {
            cold: self.0.cold.read().unwrap(),
            staged: self.0.staged.read().unwrap(),
        }
    }

    /// Runs different logic depending on if additional changes are already staged and if using cold directly would block.
    /// This *can* be faster than greedily applying staged changes if there are no staged changes and no reads from cold.
    pub fn maybe_stage<C, S>(
        &mut self,
        for_full: impl FnOnce(&mut T::Cold) -> C,
        for_staged: impl FnOnce(&mut Stager<T>) -> S,
    ) -> MaybeStaged<C, S> {
        let mut staged = self.0.staged.write().unwrap();
        if staged.any_staged() {
            let cold = self.0.cold.read().unwrap();
            MaybeStaged::Staged(for_staged(&mut Stager {
                cold: &cold,
                staged: &mut staged,
            }))
        } else if let Ok(mut cold) = self.0.cold.try_write() {
            MaybeStaged::Cold(for_full(&mut cold))
        } else {
            let cold = self.0.cold.read().unwrap();
            MaybeStaged::Staged(for_staged(&mut Stager {
                cold: &cold,
                staged: &mut staged,
            }))
        }
    }

    /// Easily run a stager function to stage changes.
    #[inline]
    pub fn stage_scope_locked<R>(&mut self, f: impl FnOnce(&mut Stager<T>) -> R) -> R {
        f(&mut self.stage_lock().as_stager())
    }

    /// Easily run a stager function to stage changes. Then, tries to apply those changes if doing so wouldn't lock.
    #[inline]
    pub fn stage_scope_locked_eager<R>(&mut self, f: impl FnOnce(&mut Stager<T>) -> R) -> R {
        let result = self.stage_scope_locked(f);
        self.apply_staged_non_blocking();
        result
    }

    /// Easily run a [`StagedRef`] function.
    #[inline]
    pub fn read_scope_locked<R>(&self, f: impl FnOnce(&StagedRef<T>) -> R) -> R {
        f(&self.read_lock().as_staged_ref())
    }
}

impl<T: StagedChanges, C: ColdStorage<T>> StagerLocked<'_, T, C> {
    /// Allows a user to view this as a [`Stager`].
    #[inline]
    pub fn as_stager(&mut self) -> Stager<'_, T> {
        Stager {
            cold: &self.cold,
            staged: &mut self.staged,
        }
    }

    /// Allows a user to view this as a [`StagedRef`].
    #[inline]
    pub fn as_staged_ref(&self) -> StagedRef<'_, T> {
        StagedRef {
            cold: &self.cold,
            staged: &self.staged,
        }
    }
}

impl<T: StagedChanges, C: ColdStorage<T>> StagedRefLocked<'_, T, C> {
    /// Allows a user to view this as a [`StagedRef`].
    #[inline]
    pub fn as_staged_ref(&self) -> StagedRef<'_, T> {
        StagedRef {
            cold: &self.cold,
            staged: &self.staged,
        }
    }
}

impl<'a, T: StagedChanges> Clone for StagedRef<'a, T> {
    fn clone(&self) -> Self {
        Self {
            staged: self.staged,
            cold: self.cold,
        }
    }
}

impl<T: StagedChanges> Default for AtomicStageOnWrite<T>
where
    T::Cold: Default,
{
    fn default() -> Self {
        Self::new(T::Cold::default())
    }
}

impl<T: StagedChanges> ColdStorage<T> for RwLockReadGuard<'_, T::Cold> {}

impl<T: StagedChanges> ColdStorage<T> for &'_ T::Cold {}

#[cfg(test)]
mod tests {
    use bevy_platform_support::{collections::HashMap, prelude::Vec};

    use super::*;

    #[derive(Default)]
    struct StagedNumVec {
        added: Vec<u32>,
        changed: HashMap<usize, u32>,
    }

    impl StagedChanges for StagedNumVec {
        type Cold = Vec<u32>;

        fn apply_staged(&mut self, storage: &mut Self::Cold) {
            storage.append(&mut self.added);
            for (index, new) in self.changed.drain() {
                storage[index] = new;
            }
        }

        fn any_staged(&self) -> bool {
            !self.added.is_empty() || !self.changed.is_empty()
        }
    }

    // This is commented out, as it intentionally does not compile. This demonstrates how `AtomicStageOnWrite` prevents deadlock using the borrow checker.
    // #[test]
    // fn test_no_compile_for_deadlock() {
    //     let mut stage_on_write = AtomicStageOnWrite::<StagedNumVec>::default();
    //     let reading = stage_on_write.read_lock();
    //     stage_on_write.apply_staged_non_blocking();
    // }

    #[test]
    fn test_simple_stage() {
        let mut data = StageOnWrite::<StagedNumVec>::default();
        data.stage_scope_locked(|stager| stager.staged.added.push(5));
        let full = data.apply_staged_for_full();
        assert_eq!(full[0], 5);
    }
}
