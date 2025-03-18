//! This module contains optimistic mutexes for specialized situations.
#![expect(unsafe_code, reason = "This is impossible otherwise")]

// inspired by https://marabos.nl/atomics/building-spinlock.html

use core::ops::{Deref, DerefMut};

use bevy_platform_support::sync::atomic::{AtomicBool, Ordering};

use crate::syncunsafecell::SyncUnsafeCell;

/// This is an alternative to `std`'s `Mutex`.
///
/// That mutex implementation is concerned with safety, but this one is concerned only with speed.
///
/// # Danger
///
/// - This mutex does not track poison.
/// - This mutex provides raw access, which could lead to unsound use.
/// - This mutex spins when contested, so don't hold the lock for extended periods.
#[derive(Default)]
pub struct OptimisticMutex<T> {
    value: SyncUnsafeCell<T>,
    locked: AtomicBool,
}

/// The guard for [`OptimisticMutex`].
pub struct OptimisticMutexGuard<'a, T>(&'a OptimisticMutex<T>);

impl<T> OptimisticMutex<T> {
    /// Creates a new [`OptimisticMutex`].
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: SyncUnsafeCell::new(value),
        }
    }

    /// Unlocks the mutex.
    ///
    /// # Safety
    ///
    /// The lock must not be held by another thread.
    pub unsafe fn raw_unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    /// Locks the mutex.
    pub fn raw_lock(&self) {
        while self.locked.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }
    }

    /// Tries to lock the mutex, returning true if it was locked
    pub fn raw_try_lock(&self) -> bool {
        !self.locked.swap(true, Ordering::Acquire)
    }

    /// Gets the inner value.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// Consumes the mutex, returning the contained value.
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    /// Locks the mutex for a [`OptimisticMutexGuard`]
    pub fn lock(&self) -> OptimisticMutexGuard<'_, T> {
        self.raw_lock();
        OptimisticMutexGuard(self)
    }

    /// Locks the mutex for a [`OptimisticMutexGuard`]
    pub fn try_lock(&self) -> Option<OptimisticMutexGuard<'_, T>> {
        self.raw_try_lock().then_some(OptimisticMutexGuard(self))
    }
}

impl<T> Drop for OptimisticMutexGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: we are locked here
        unsafe {
            self.0.raw_unlock();
        }
    }
}

impl<T> Deref for OptimisticMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We are holding the lock
        unsafe { &*self.0.value.get() }
    }
}

impl<T> DerefMut for OptimisticMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: We are holding the lock
        unsafe { &mut *self.0.value.get() }
    }
}

impl<'a, T> OptimisticMutexGuard<'a, T> {
    /// Gets the inner mutex for reuse.
    pub fn into_inner(self) -> &'a OptimisticMutex<T> {
        let this = core::mem::ManuallyDrop::new(self);
        // SAFETY: We still hold the lock
        unsafe {
            this.0.raw_unlock();
        }
        this.0
    }
}
