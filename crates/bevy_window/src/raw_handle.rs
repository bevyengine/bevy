use bevy_ecs::prelude::Component;
use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
};

/// A wrapper over [`RawWindowHandle`] and [`RawDisplayHandle`] that allows us to safely pass it across threads.
///
/// Depending on the platform, the underlying pointer-containing handle cannot be used on all threads,
/// and so we cannot simply make it (or any type that has a safe operation to get a [`RawWindowHandle`] or [`RawDisplayHandle`])
/// thread-safe.
#[derive(Debug, Clone, Component)]
pub struct RawHandleWrapper {
    pub window_handle: RawWindowHandle,
    pub display_handle: RawDisplayHandle,
}

impl RawHandleWrapper {
    /// Returns a [`HasRawWindowHandle`] + [`HasRawDisplayHandle`] impl, which exposes [`RawWindowHandle`] and [`RawDisplayHandle`].
    ///
    /// # Safety
    ///
    /// Some platforms have constraints on where/how this handle can be used. For example, some platforms don't support doing window
    /// operations off of the main thread. The caller must ensure the [`RawHandleWrapper`] is only used in valid contexts.
    pub unsafe fn get_handle(&self) -> ThreadLockedRawWindowHandleWrapper {
        ThreadLockedRawWindowHandleWrapper(self.clone())
    }

    pub fn get_display_handle(&self) -> RawDisplayHandle {
        self.display_handle
    }

    pub fn get_window_handle(&self) -> RawWindowHandle {
        self.window_handle
    }
}

// SAFETY: [`RawHandleWrapper`] is just a normal "raw pointer", which doesn't impl Send/Sync. However the pointer is only
// exposed via an unsafe method that forces the user to make a call for a given platform. (ex: some platforms don't
// support doing window operations off of the main thread).
// A recommendation for this pattern (and more context) is available here:
// https://github.com/rust-windowing/raw-window-handle/issues/59
unsafe impl Send for RawHandleWrapper {}
unsafe impl Sync for RawHandleWrapper {}

/// A [`RawHandleWrapper`] that cannot be sent across threads.
///
/// This safely exposes [`RawWindowHandle`] and [`RawDisplayHandle`], but care must be taken to ensure that the construction itself is correct.
///
/// This can only be constructed via the [`RawHandleWrapper::get_handle()`] method;
/// be sure to read the safety docs there about platform-specific limitations.
/// In many cases, this should only be constructed on the main thread.
pub struct ThreadLockedRawWindowHandleWrapper(RawHandleWrapper);

// SAFETY: the caller has validated that this is a valid context to get [`RawHandleWrapper`]
// as otherwise an instance of this type could not have been constructed
// NOTE: we cannot simply impl HasRawWindowHandle for RawHandleWrapper,
// as the `raw_window_handle` method is safe. We cannot guarantee that all calls
// of this method are correct (as it may be off the main thread on an incompatible platform),
// and so exposing a safe method to get a [`RawWindowHandle`] directly would be UB.
unsafe impl HasRawWindowHandle for ThreadLockedRawWindowHandleWrapper {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0.get_window_handle()
    }
}

// SAFETY: the caller has validated that this is a valid context to get [`RawDisplayHandle`]
// as otherwise an instance of this type could not have been constructed
// NOTE: we cannot simply impl HasRawDisplayHandle for RawHandleWrapper,
// as the `raw_display_handle` method is safe. We cannot guarantee that all calls
// of this method are correct (as it may be off the main thread on an incompatible platform),
// and so exposing a safe method to get a [`RawDisplayHandle`] directly would be UB.
unsafe impl HasRawDisplayHandle for ThreadLockedRawWindowHandleWrapper {
    fn raw_display_handle(&self) -> RawDisplayHandle {
        self.0.get_display_handle()
    }
}
