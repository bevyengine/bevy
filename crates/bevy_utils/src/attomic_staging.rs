//! Provides an abstracted system for staging modifications attomically.

use core::ops::Deref;

use bevy_platform_support::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Signifies that this type represents staged changes to [`Cold`](Self::Cold).
pub trait StagedChanges {
    /// The more compact data structure that these changes compact into.
    type Cold;

    /// This applies these changes to the passed [`Cold`](Self::Cold). When this is done, there should be no more staged changes, and [`any_staged`](Self::any_staged) should be false.
    fn apply_staged(&mut self, storage: &mut Self::Cold);

    /// Returns true if and only if there are staged changes that could be applied.
    fn any_staged(&self) -> bool;
}

/// A trait that signifies that it holds an immutable reference to a cold type (ie. [`StagedChanges::Cold`]).
pub trait ColdStorage<T: StagedChanges>: Deref<Target = T::Cold> {}

/// A struct that allows staging changes while reading from cold storage.
pub struct Stager<'a, T: StagedChanges> {
    /// The storage that is read optimized.
    pub cold: &'a T::Cold,
    /// The staged changes.
    pub staged: &'a mut T,
}

/// A struct that allows accessing changes while reading from cold storage.
#[derive(Copy)]
pub struct StagedRef<'a, T: StagedChanges> {
    /// The storage that is read optimized.
    pub cold: &'a T::Cold,
    /// The staged changes.
    pub staged: &'a T,
}

/// A locked version of [`Stager`]
pub struct StagerLocked<'a, T: StagedChanges, C: ColdStorage<T>> {
    cold: C,
    staged: RwLockWriteGuard<'a, T>,
}

/// A locked version of [`StagedRef`]
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

/// A struct that allows read-optimized operations while still allowing mutation.
#[derive(Default)]
pub struct StageOnWrite<T: StagedChanges> {
    /// Cold data is read optimized.
    cold: T::Cold,
    /// Staged data stores recent modifications to cold. It's [`RwLock`] coordinates mutations.
    staged: RwLock<T>,
}

#[derive(Default)]
struct AttomicStageOnWriteInner<T: StagedChanges> {
    /// Cold data is read optimized. This lives behind a [`RwLock`], but it is only written to for applying changes in
    /// a non-blocking way. In other worlds this locks, but almost never blocks. (It can technically block if a thread
    /// tries to read from it while it is having changes applied, but that is extremely rare.)
    cold: RwLock<T::Cold>,
    /// Staged data stores recent modifications to cold. It's [`RwLock`] coordinates mutations.
    staged: RwLock<T>,
}

/// A struct that allows read-optimized operations while still allowing mutation.
#[derive(Clone)]
pub struct AttomicStageOnWrite<T: StagedChanges>(Arc<AttomicStageOnWriteInner<T>>);

impl<T: StagedChanges> StageOnWrite<T> {
    /// Gets the inner cold data if it is safe. If [`any_staged`](Self::any_staged) is known to be false, this can be safely unwrapped.
    #[inline]
    pub fn full(&mut self) -> Option<&mut T::Cold> {
        if self.staged.get_mut().unwrap().any_staged() {
            None
        } else {
            Some(self.cold.get_mut().unwrap())
        }
    }

    /// Gets the inner cold data if it is safe. If [`any_staged`](Self::any_staged) is known to be false, this can be safely unwrapped.
    #[inline]
    pub fn full_locked(&self) -> Option<&RwLock<T::Cold>> {
        if self.any_staged() {
            None
        } else {
            Some(&self.cold)
        }
    }

    /// Returns true if and only if there are staged changes that could be applied.
    #[inline]
    pub fn any_staged(&self) -> bool {
        self.staged.read().unwrap().any_staged()
    }

    /// Applies any staged changes before returning the full value with all changes applied. Immediately after this, [`any_staged`](Self::any_staged) will be false.
    #[inline]
    pub fn apply_staged_for_full(&mut self) -> &mut T::Cold {
        let staged = self.staged.get_mut().unwrap();
        let cold = self.cold.get_mut().unwrap();
        if staged.any_staged() {
            staged.apply_staged(cold);
        }
        cold
    }

    /// A version of [`apply_staged_for_full`](Self::apply_staged_for_full) that locks (and may block).
    /// Returns `None` if no changes needed to be made, and the stage could be skipped.
    #[inline]
    pub fn apply_staged_lock(&self) -> Option<RwLockWriteGuard<'_, T::Cold>> {
        let mut staged = self.staged.write().unwrap();
        if staged.any_staged() {
            let mut cold = self.cold.write().unwrap();
            staged.apply_staged(&mut cold);
            Some(cold)
        } else {
            None
        }
    }

    /// A version of [`apply_staged_for_full`](Self::apply_staged_for_full) that locks and never blocks.
    /// If a read on another thread is immediately hit, that may block, but this will not. Returns `None`
    /// if either their were no changes to be made and the stage could be skipped, or if the operation would block.
    #[inline]
    pub fn apply_staged_non_blocking(&self) -> Option<RwLockWriteGuard<'_, T::Cold>> {
        let mut staged = self.staged.write().unwrap();
        if staged.any_staged() {
            let mut cold = self.cold.try_write().ok()?;
            staged.apply_staged(&mut cold);
            Some(cold)
        } else {
            None
        }
    }

    /// Constructs a [`Stager`] that will stage changes.
    #[inline]
    pub fn stage(&mut self) -> Stager<'_, T> {
        Stager {
            cold: self.cold.get_mut().unwrap(),
            staged: self.staged.get_mut().unwrap(),
        }
    }

    /// Constructs a [`StagerLocked`], locking internally.
    #[inline]
    pub fn stage_lock(&self) -> StagerLocked<'_, T, RwLockReadGuard<'_, T::Cold>> {
        StagerLocked {
            cold: self.cold.read().unwrap(),
            staged: self.staged.write().unwrap(),
        }
    }

    /// Easily run a stager function to stage changes.
    #[inline]
    pub fn stage_scope_locked<R>(&self, f: impl FnOnce(&mut Stager<T>) -> R) -> R {
        f(&mut self.stage_lock().as_stager())
    }

    /// Easily run a stager function to stage changes. Then, try to apply those staged changes if it can do so without blocking.
    #[inline]
    pub fn stage_scope_locked_eager<R>(&self, f: impl FnOnce(&mut Stager<T>) -> R) -> R {
        let v = self.stage_scope_locked(f);
        self.apply_staged_non_blocking();
        v
    }

    /// Easily run a stager function to stage changes and return locked data.
    #[inline]
    pub fn stage_locked_scope<R>(
        &self,
        f: impl FnOnce(StagerLocked<T, RwLockReadGuard<'_, T::Cold>>) -> R,
    ) -> R {
        f(self.stage_lock())
    }

    /// Constructs a [`Stager`] that will stage changes.
    #[inline]
    pub fn read(&mut self) -> StagedRef<'_, T> {
        StagedRef {
            cold: self.cold.get_mut().unwrap(),
            staged: self.staged.get_mut().unwrap(),
        }
    }

    /// Constructs a [`StagerLocked`], locking internally.
    #[inline]
    pub fn read_lock(&self) -> StagedRefLocked<'_, T, RwLockReadGuard<'_, T::Cold>> {
        StagedRefLocked {
            cold: self.cold.read().unwrap(),
            staged: self.staged.read().unwrap(),
        }
    }

    /// Easily run a [`StagedRef`] function.
    #[inline]
    pub fn read_scope_locked<R>(&self, f: impl FnOnce(&StagedRef<T>) -> R) -> R {
        f(&self.read_lock().as_staged_ref())
    }

    /// Runs different logic depending on if additional changes are already staged. This can be faster than greedily applying staged changes if there are already staged changes.
    pub fn maybe_stage<C, S>(
        &mut self,
        for_full: impl FnOnce(&mut T::Cold) -> C,
        for_staged: impl FnOnce(Stager<T>) -> S,
    ) -> MaybeStaged<C, S> {
        let staged = self.staged.get_mut().unwrap();
        let cold = self.cold.get_mut().unwrap();
        if staged.any_staged() {
            MaybeStaged::Staged(for_staged(Stager { cold, staged }))
        } else {
            MaybeStaged::Cold(for_full(cold))
        }
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

impl<T: StagedChanges> ColdStorage<T> for RwLockReadGuard<'_, T::Cold> {}

impl<T: StagedChanges> ColdStorage<T> for &'_ T::Cold {}
