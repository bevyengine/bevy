//! Provides Android functionality for Bevy Engine.

#![cfg(target_os = "android")]

pub use android_activity;

use android_activity::AndroidApp;
use std::{
    ops::Deref,
    sync::{RwLock, RwLockReadGuard},
};

/// Global storage for the current Android application handle.
/// [`AndroidApp`] provides an interface to query the application state as well as monitor events
/// (for example lifecycle and input events).
/// The handle can change when Android destroys/recreates the activity.
pub static ANDROID_APP: AndroidAppStorage = AndroidAppStorage::empty();

/// Thread-safe storage for the Android application handle.
pub struct AndroidAppStorage {
    app: RwLock<Option<AndroidApp>>,
}

impl AndroidAppStorage {
    /// Creates an uninitialized storage.
    const fn empty() -> Self {
        Self {
            app: RwLock::new(None),
        }
    }

    /// Updates/Replaces the stored Android application handle.
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned.
    pub fn write(&self, app: Option<AndroidApp>) {
        *self.app.write().unwrap() = app;
    }

    /// Returns a read guard for the current Android application handle.
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned or if the Android app has not been initialized yet.
    pub fn read(&self) -> AndroidAppReadGuard<'_> {
        let guard = self.app.read().unwrap();
        AndroidAppReadGuard { guard }
    }
}

/// Currently there is no `map` fn for `RwLockReadGuard`, this is a wrapper for it.
pub struct AndroidAppReadGuard<'a> {
    guard: RwLockReadGuard<'a, Option<AndroidApp>>,
}

impl<'a> Deref for AndroidAppReadGuard<'a> {
    type Target = AndroidApp;

    fn deref(&self) -> &Self::Target {
        self.guard
            .as_ref()
            .expect("Bevy must be setup with the #[bevy_main] macro on Android")
    }
}
