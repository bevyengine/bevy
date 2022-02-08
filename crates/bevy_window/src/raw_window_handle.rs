use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

/// This wrapper exist to enable safely passing a [`RawWindowHandle`] across threads. Extracting the handle
/// is still an unsafe operation, so the caller must still validate that using the raw handle is safe for a given context.
#[derive(Debug, Clone)]
pub struct RawWindowHandleWrapper(RawWindowHandle);

impl RawWindowHandleWrapper {
    pub(crate) fn new(handle: RawWindowHandle) -> Self {
        Self(handle)
    }

    /// # Safety
    /// This returns a [`HasRawWindowHandle`] impl, which exposes [`RawWindowHandle`]. Some platforms
    /// have constraints on where/how this handle can be used. For example, some platforms don't support doing window
    /// operations off of the main thread. The caller must ensure the [`RawWindowHandle`] is only used in valid contexts.
    pub unsafe fn get_handle(&self) -> HasRawWindowHandleWrapper {
        HasRawWindowHandleWrapper(self.0)
    }
}

// SAFE: RawWindowHandle is just a normal "raw pointer", which doesn't impl Send/Sync. However the pointer is only
// exposed via an unsafe method that forces the user to make a call for a given platform. (ex: some platforms don't
// support doing window operations off of the main thread).
// A recommendation for this pattern (and more context) is available here:
// https://github.com/rust-windowing/raw-window-handle/issues/59
unsafe impl Send for RawWindowHandleWrapper {}
unsafe impl Sync for RawWindowHandleWrapper {}

pub struct HasRawWindowHandleWrapper(RawWindowHandle);

// SAFE: the caller has validated that this is a valid context to get RawWindowHandle
unsafe impl HasRawWindowHandle for HasRawWindowHandleWrapper {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0
    }
}
