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
pub static ANDROID_APP: AndroidAppStorage = AndroidAppStorage::empty();

/// Thread-safe storage for the Android application handle.
/// The handle can change when Android recreates the activity.
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
    pub fn set(&self, app: AndroidApp) {
        *self.app.write().unwrap() = Some(app);
    }

    /// Returns a read guard for the current Android application handle.
    ///
    /// # Panics
    ///
    /// Panics if the internal lock is poisoned or if the Android app has not been initialized yet.
    pub fn get(&self) -> AndroidAppReadGuard<'_> {
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
