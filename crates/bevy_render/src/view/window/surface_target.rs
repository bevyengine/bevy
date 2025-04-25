#![expect(
    unsafe_code,
    reason = "This module interacts with wgpu's unsafe interfaces; thus, we have to use unsafe code here."
)]
use alloc::sync::Arc;
use bevy_derive::Deref;
use bevy_ecs::component::Component;
use thiserror::Error;
use wgpu::rwh::{DisplayHandle, HasDisplayHandle, HasWindowHandle, WindowHandle};

pub use wgpu::SurfaceTarget;
pub use wgpu::SurfaceTargetUnsafe;

use crate::renderer::WgpuWrapper;

pub(crate) type SurfaceTargetSourceHandle = Arc<dyn HasSurfaceTarget + Send + Sync + 'static>;

/// Holds a reference to a window or view that is capabable of returning a surface target.
#[derive(Clone, Component)]
pub struct SurfaceTargetSource {
    source: SurfaceTargetSourceHandle,
    /// Set to `true` if surfaces must only be initialized and used on the main thread.
    non_send: bool,
}

/// An error returned by [`SurfaceTargetSource::surface_target()`] when a surface target is unavailable.
#[derive(Error, Debug, Clone)]
pub enum SurfaceTargetError {
    #[error("SurfaceTargetSource did not return a surface target")]
    NoSurfaceTarget,
    #[error("This surface target can only be accessed from the main thread")]
    InvalidThread,
}

/// An error returned by [`SurfaceTargetSource::create_surface()`] when surface creation fails.
#[derive(Error, Debug, Clone)]
pub enum SurfaceCreationError {
    #[error("The SurfaceTargetSource did not return a surface target")]
    NoSurfaceTarget,
    #[error("A surface on this surface target can only be created from the main thread")]
    InvalidThread,
    #[error("Unable to create surface: {0:?}")]
    Failed(wgpu::CreateSurfaceError),
}

impl From<SurfaceTargetError> for SurfaceCreationError {
    fn from(value: SurfaceTargetError) -> Self {
        match value {
            SurfaceTargetError::InvalidThread => Self::InvalidThread,
            SurfaceTargetError::NoSurfaceTarget => Self::NoSurfaceTarget,
        }
    }
}

impl From<wgpu::CreateSurfaceError> for SurfaceCreationError {
    fn from(value: wgpu::CreateSurfaceError) -> Self {
        SurfaceCreationError::Failed(value)
    }
}

/// Wraps a [`wgpu::Surface`] and holds a handle to the source window / view to ensure the surface does not outlive it.
#[derive(Deref)]
pub struct RenderSurface {
    _source: SurfaceTargetSourceHandle,
    #[deref]
    surface: WgpuWrapper<wgpu::Surface<'static>>,
}

impl RenderSurface {
    /// Returns the underlying [`wgpu::Surface`].
    ///
    /// ## Safety
    ///
    /// The caller must ensure the returned surface is only used when the window / view is still alive and valid.
    pub unsafe fn into_inner(self) -> wgpu::Surface<'static> {
        self.surface.into_inner()
    }
}

/// An internal wrapper that allows a main-thread-only surface target to be sent across threads.
///
/// ## Safety
///
/// The wrapped surface target must only be used on the main thread.
struct NonSendHasSurfaceTarget<T: HasSurfaceTarget + 'static>(T);

// SAFETY: This is an internal type that is only used on the main thread.
unsafe impl<T: HasSurfaceTarget + 'static> Send for NonSendHasSurfaceTarget<T> {}
// SAFETY: This is an internal type that is only used on the main thread.
unsafe impl<T: HasSurfaceTarget + 'static> Sync for NonSendHasSurfaceTarget<T> {}

impl<T: HasSurfaceTarget + 'static> HasSurfaceTarget for NonSendHasSurfaceTarget<T> {
    unsafe fn surface_target(&self) -> Option<SurfaceTargetWrapper> {
        self.0.surface_target()
    }
}

impl SurfaceTargetSource {
    /// Creates a new surface target source that's safe to be used across threads.
    ///
    /// ## Safety
    ///
    /// If the surface target source strictly only allows main-thread access (e.g. UiKit, AppKit),
    /// you *must* set `main_thread_only` or use [`SurfaceTargetSource::new_non_send`] instead.
    pub fn new<T: HasSurfaceTarget + Send + Sync + 'static>(
        main_thread_only: bool,
        source: T,
    ) -> Self {
        Self {
            source: Arc::new(source),
            non_send: main_thread_only,
        }
    }

    /// Creates a new surface target source that requires main-thread-only access.
    pub fn new_non_send<T: HasSurfaceTarget + 'static>(source: T) -> Self {
        Self {
            source: Arc::new(NonSendHasSurfaceTarget(source)),
            non_send: true,
        }
    }

    /// Returns `true` if this window / view may only be used on the main thread.
    pub fn is_non_send(&self) -> bool {
        self.non_send
    }

    /// Returns the surface target for the window or view (if available).
    pub fn surface_target(
        &self,
        is_main_thread: bool,
    ) -> Result<SurfaceTargetWrapper<'_>, SurfaceTargetError> {
        if self.non_send && !is_main_thread {
            return Err(SurfaceTargetError::InvalidThread);
        }

        // SAFETY: We verify the thread above.
        unsafe { self.source.surface_target() }.ok_or(SurfaceTargetError::NoSurfaceTarget)
    }

    /// Creates a surface.
    pub fn create_surface(
        &self,
        instance: &wgpu::Instance,
        is_main_thread: bool,
    ) -> Result<RenderSurface, SurfaceCreationError> {
        let surface_target = self
            .surface_target(is_main_thread)
            .map_err(|err| SurfaceCreationError::from(err))?;

        let surface = match surface_target {
            SurfaceTargetWrapper::SurfaceTarget(surface_target) => {
                // SAFETY: The returned surface is returned with window/view handle that ensures it lives at least as long as the surface does.
                let static_surface_target = unsafe {
                    std::mem::transmute::<SurfaceTarget<'_>, SurfaceTarget<'static>>(surface_target)
                };
                instance
                    .create_surface(static_surface_target)
                    .map_err(|err| SurfaceCreationError::from(err))?
            }
            SurfaceTargetWrapper::SurfaceTargetUnsafe(surface_target_unsafe) => {
                // SAFETY:
                // - The returned surface is returned with window/view handle that ensures it lives at least as long as the surface does.
                // - The surface target source is expected to only return valid surface targets.
                unsafe { instance.create_surface_unsafe(surface_target_unsafe) }
                    .map_err(|err| SurfaceCreationError::from(err))?
            }
        };

        Ok(RenderSurface {
            _source: self.source.clone(),
            surface: WgpuWrapper::new(surface),
        })
    }
}

pub trait HasSurfaceTarget {
    /// Returns the surface target for the window or view (if available).
    ///
    /// ## Safety
    ///
    /// **It's up to the caller to ensure:**
    ///
    /// - The returned surface is only used on an appropriate thread. Certain platforms / surface targets
    ///   like UiKit and AppKit require access ONLY on the main thread. It is up to the caller to know
    ///   platform conventions and abide by them.
    ///
    /// **It's up to the trait implementor to ensure:**
    ///
    /// - The returned surface is valid.
    unsafe fn surface_target(&self) -> Option<SurfaceTargetWrapper>;
}

impl<T> HasSurfaceTarget for T
where
    T: wgpu::WindowHandle,
{
    unsafe fn surface_target(&self) -> Option<SurfaceTargetWrapper> {
        Some(SurfaceTargetWrapper::SurfaceTarget(SurfaceTarget::from(
            self,
        )))
    }
}

/// A wrapper over [`wgpu::SurfaceTarget`] and [`wgpu::SurfaceTargetUnsafe`].
///
/// This is inherently thread-locked due to the inner types not being `Send` nor `Sync`.
pub enum SurfaceTargetWrapper<'a> {
    SurfaceTarget(SurfaceTarget<'a>),
    SurfaceTargetUnsafe(SurfaceTargetUnsafe),
}

impl HasDisplayHandle for SurfaceTargetWrapper<'_> {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, wgpu::rwh::HandleError> {
        match self {
            Self::SurfaceTarget(surface_target) => match &surface_target {
                SurfaceTarget::Window(window) => window.display_handle(),
                _ => Err(wgpu::rwh::HandleError::NotSupported),
            },
            Self::SurfaceTargetUnsafe(surface_target_unsafe) => match surface_target_unsafe {
                SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle,
                    raw_window_handle: _,
                } => {
                    // SAFETY: TODO
                    Ok(unsafe { DisplayHandle::borrow_raw(*raw_display_handle) })
                }
                _ => Err(wgpu::rwh::HandleError::NotSupported),
            },
        }
    }
}

impl HasWindowHandle for SurfaceTargetWrapper<'_> {
    fn window_handle(&self) -> Result<WindowHandle<'_>, wgpu::rwh::HandleError> {
        match self {
            Self::SurfaceTarget(surface_target) => match &surface_target {
                SurfaceTarget::Window(window) => window.window_handle(),
                _ => Err(wgpu::rwh::HandleError::NotSupported),
            },
            Self::SurfaceTargetUnsafe(surface_target_unsafe) => match surface_target_unsafe {
                SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: _,
                    raw_window_handle,
                } => {
                    // SAFETY: TODO
                    Ok(unsafe { WindowHandle::borrow_raw(*raw_window_handle) })
                }
                _ => Err(wgpu::rwh::HandleError::NotSupported),
            },
        }
    }
}
