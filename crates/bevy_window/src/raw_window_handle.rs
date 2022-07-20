use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

/// A wrapper over [`RawWindowHandle`] that allows us to safely pass it across threads.
///
/// Depending on the platform, the underlying pointer-containing handle cannot be used on all threads,
/// and so we cannot simply make it (or any type that has a safe operation to get a [`RawWindowHandle`])
/// thread-safe.
#[derive(Debug, Clone)]
pub struct RawWindowHandleWrapper(RawWindowHandle);

impl RawWindowHandleWrapper {
    pub(crate) fn new(handle: RawWindowHandle) -> Self {
        Self(handle)
    }

    /// Returns a [`HasRawWindowHandle`] impl, which exposes [`RawWindowHandle`].
    ///
    /// # Safety
    ///
    /// Some platforms have constraints on where/how this handle can be used. For example, some platforms don't support doing window
    /// operations off of the main thread. The caller must ensure the [`RawWindowHandle`] is only used in valid contexts.
    pub unsafe fn get_handle(&self) -> ThreadLockedRawWindowHandleWrapper {
        ThreadLockedRawWindowHandleWrapper(self.0)
    }
}

// SAFETY: RawWindowHandle is just a normal "raw pointer", which doesn't impl Send/Sync. However the pointer is only
// exposed via an unsafe method that forces the user to make a call for a given platform. (ex: some platforms don't
// support doing window operations off of the main thread).
// A recommendation for this pattern (and more context) is available here:
// https://github.com/rust-windowing/raw-window-handle/issues/59
unsafe impl Send for RawWindowHandleWrapper {}
unsafe impl Sync for RawWindowHandleWrapper {}

/// A [`RawWindowHandleWrapper`] that cannot be sent across threads.
///
/// This safely exposes a [`RawWindowHandle`], but care must be taken to ensure that the construction itself is correct.
///
/// This can only be constructed via the [`RawWindowHandleWrapper::get_handle()`] method;
/// be sure to read the safety docs there about platform-specific limitations.
/// In many cases, this should only be constructed on the main thread.
pub struct ThreadLockedRawWindowHandleWrapper(RawWindowHandle);

// SAFETY: the caller has validated that this is a valid context to get RawWindowHandle
// as otherwise an instance of this type could not have been constructed
// NOTE: we cannot simply impl HasRawWindowHandle for RawWindowHandleWrapper,
// as the `raw_window_handle` method is safe. We cannot guarantee that all calls
// of this method are correct (as it may be off the main thread on an incompatible platform),
// and so exposing a safe method to get a `RawWindowHandle` directly would be UB.
unsafe impl HasRawWindowHandle for ThreadLockedRawWindowHandleWrapper {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0
    }
}
