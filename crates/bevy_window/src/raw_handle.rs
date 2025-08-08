#![expect(
    unsafe_code,
    reason = "This module acts as a wrapper around the `raw_window_handle` crate, which exposes many unsafe interfaces; thus, we have to use unsafe code here."
)]

use alloc::sync::Arc;
use bevy_ecs::{prelude::Component, resource::Resource};
use bevy_platform::sync::Mutex;
use core::{any::Any, marker::PhantomData, ops::Deref};
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, WindowHandle,
};

/// A wrapper over a window.
///
/// This allows us to extend the lifetime of the window, so it doesn't get eagerly dropped while a
/// pipelined renderer still has frames in flight that need to draw to it.
///
/// This is achieved by storing a shared reference to the window in the [`RawWindowHandleWrapper`],
/// which gets picked up by the renderer during extraction.
#[derive(Debug)]
pub struct WindowWrapper<W> {
    reference: Arc<dyn Any + Send + Sync>,
    ty: PhantomData<W>,
}

impl<W: Send + Sync + 'static> WindowWrapper<W> {
    /// Creates a `WindowWrapper` from a window.
    pub fn new(window: W) -> WindowWrapper<W> {
        WindowWrapper {
            reference: Arc::new(window),
            ty: PhantomData,
        }
    }
}

impl<W: 'static> Deref for WindowWrapper<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.reference.downcast_ref::<W>().unwrap()
    }
}

/// A wrapper over [`RawWindowHandle`] and [`RawDisplayHandle`] that allows us to safely pass it across threads.
///
/// Depending on the platform, the underlying pointer-containing handle cannot be used on all threads,
/// and so we cannot simply make it (or any type that has a safe operation to get a [`RawWindowHandle`] or [`RawDisplayHandle`])
/// thread-safe.
#[derive(Debug, Clone, Component)]
pub struct RawWindowHandleWrapper {
    /// A shared reference to the window.
    /// This allows us to extend the lifetime of the window,
    /// so it doesn’t get eagerly dropped while a pipelined
    /// renderer still has frames in flight that need to draw to it.
    _window: Arc<dyn Any + Send + Sync>,
    /// Raw handle to a window.
    window_handle: RawWindowHandle,
}

impl RawWindowHandleWrapper {
    /// Creates a [`RawWindowHandleWrapper`] from a [`WindowWrapper`].
    pub fn new<W: HasWindowHandle + 'static>(
        window: &WindowWrapper<W>,
    ) -> Result<Self, HandleError> {
        Ok(Self {
            _window: window.reference.clone(),
            window_handle: window.window_handle()?.as_raw(),
            // display_handle: window.display_handle()?.as_raw(),
        })
    }

    /// Returns a [`HasWindowHandle`] + [`HasDisplayHandle`] impl, which exposes [`WindowHandle`] and [`DisplayHandle`].
    ///
    /// # Safety
    ///
    /// Some platforms have constraints on where/how this handle can be used. For example, some platforms don't support doing window
    /// operations off of the main thread. The caller must ensure the [`RawWindowHandleWrapper`] is only used in valid contexts.
    pub unsafe fn get_handle(&self) -> ThreadLockedRawWindowHandleWrapper {
        ThreadLockedRawWindowHandleWrapper(self.clone())
    }

    /// Gets the stored window handle.
    pub fn get_window_handle(&self) -> RawWindowHandle {
        self.window_handle
    }

    /// Sets the window handle.
    ///
    /// # Safety
    ///
    /// The passed in [`RawWindowHandle`] must be a valid window handle.
    // NOTE: The use of an explicit setter instead of a getter for a mutable reference is to limit the amount of time unsoundness can happen.
    //       If we handed out a mutable reference the user would have to maintain safety invariants throughout its lifetime. For consistency
    //       we also prefer to handout copies of the handles instead of immutable references.
    pub unsafe fn set_window_handle(&mut self, window_handle: RawWindowHandle) -> &mut Self {
        self.window_handle = window_handle;

        self
    }
}

// SAFETY: `RawWindowHandleWrapper` is just a normal "raw pointer", which doesn't impl Send/Sync. However the pointer is only
// exposed via an unsafe method that forces the user to make a call for a given platform. (ex: some platforms don't
// support doing window operations off of the main thread).
// A recommendation for this pattern (and more context) is available here:
// https://github.com/rust-windowing/raw-window-handle/issues/59
unsafe impl Send for RawWindowHandleWrapper {}
// SAFETY: This is safe for the same reasons as the Send impl above.
unsafe impl Sync for RawWindowHandleWrapper {}

/// A [`RawWindowHandleWrapper`] that cannot be sent across threads.
///
/// This safely exposes [`RawWindowHandle`], but care must be taken to ensure that the construction itself is correct.
///
/// This can only be constructed via the [`RawWindowHandleWrapper::get_handle()`] method;
/// be sure to read the safety docs there about platform-specific limitations.
/// In many cases, this should only be constructed on the main thread.
pub struct ThreadLockedRawWindowHandleWrapper(RawWindowHandleWrapper);

impl HasWindowHandle for ThreadLockedRawWindowHandleWrapper {
    fn window_handle(&self) -> Result<WindowHandle, HandleError> {
        // SAFETY: the caller has validated that this is a valid context to get `RawWindowHandleWrapper`
        // as otherwise an instance of this type could not have been constructed
        // NOTE: we cannot simply impl HasRawWindowHandle for RawWindowHandleWrapper,
        // as the `raw_window_handle` method is safe. We cannot guarantee that all calls
        // of this method are correct (as it may be off the main thread on an incompatible platform),
        // and so exposing a safe method to get a [`RawWindowHandle`] directly would be UB.
        // XXX: ^ that note is flawed. "getting" the handle is safe - it's about how it's used. Any
        // API **consuming** a RawWindowHandle should be adequately marked unsafe instead.
        Ok(unsafe { WindowHandle::borrow_raw(self.0.window_handle) })
    }
}

// XXX: Could still be implemented because `_window` provides it
// impl HasDisplayHandle for ThreadLockedRawDisplayHandleWrapper {
//     fn display_handle(&self) -> Result<DisplayHandle, HandleError> {
//     }
// }

/// Holder of the [`RawHWindowandleWrapper`] with wrappers, to allow use in asynchronous context
#[derive(Debug, Clone, Component)]
pub struct RawWindowHandleWrapperHolder(pub Arc<Mutex<Option<RawWindowHandleWrapper>>>);

/// A wrapper over [`RawDisplayHandle`] that allows us to safely pass it across threads.
///
/// Depending on the platform, the underlying pointer-containing handle cannot be used on all threads,
/// and so we cannot simply make it (or any type that has a safe operation to get a [`RawWindowHandle`] or [`RawDisplayHandle`])
/// thread-safe.
#[derive(Debug, Clone, Resource)]
pub struct RawDisplayHandleWrapper {
    // XXX: Do we need to hold on to the EventLoop who owns/runs the app?
    /// Raw handle to the display server.
    display_handle: RawDisplayHandle,
}

impl RawDisplayHandleWrapper {
    /// Creates a `RawDisplayHandleWrapper` from an event loop or similar.
    // XXX: Do we need to store `display` for lifetime purposes? It should outlive App.
    pub fn new<W: HasDisplayHandle + 'static>(display: &W) -> Result<Self, HandleError> {
        Ok(Self {
            display_handle: display.display_handle()?.as_raw(),
        })
    }

    /// Gets the stored display handle
    pub fn get_display_handle(&self) -> RawDisplayHandle {
        self.display_handle
    }

    /// Sets the display handle.
    ///
    /// # Safety
    ///
    /// The passed in [`RawDisplayHandle`] must be a valid display handle.
    pub fn set_display_handle(&mut self, display_handle: RawDisplayHandle) -> &mut Self {
        self.display_handle = display_handle;

        self
    }
}

// SAFETY: `RawDisplayHandleWrapper` is just a normal "raw pointer", which doesn't impl Send/Sync. However the pointer is only
// exposed via an unsafe method that forces the user to make a call for a given platform. (ex: some platforms don't
// support doing window operations off of the main thread).
// A recommendation for this pattern (and more context) is available here:
// https://github.com/rust-windowing/raw-window-handle/issues/59
unsafe impl Send for RawDisplayHandleWrapper {}
// SAFETY: This is safe for the same reasons as the Send impl above.
unsafe impl Sync for RawDisplayHandleWrapper {}

// /// A [`RawDisplayHandleWrapper`] that cannot be sent across threads.
// ///
// /// This safely exposes [`RawDisplayHandle`], but care must be taken to ensure that the construction itself is correct.
// ///
// /// This can only be constructed via the [`RawDisplayHandleWrapper::get_handle()`] method;
// /// be sure to read the safety docs there about platform-specific limitations.
// /// In many cases, this should only be constructed on the main thread.
// pub struct ThreadLockedRawDisplayHandleWrapper(RawDisplayHandleWrapper);

impl HasDisplayHandle for RawDisplayHandleWrapper {
    fn display_handle(&self) -> Result<DisplayHandle, HandleError> {
        // SAFETY: the caller has validated that this is a valid context to get `RawDisplayHandle`
        // as otherwise an instance of this type could not have been constructed
        Ok(unsafe { DisplayHandle::borrow_raw(self.display_handle) })
    }
}
