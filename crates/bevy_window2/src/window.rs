use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::{Component, Entity};
use bevy_math::{DVec2, IVec2, Vec2};
use bevy_reflect::Reflect;
use bevy_utils::tracing::warn;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use crate::{CursorIcon, WindowCommand};

/// Resource containing entity id of the primary window
#[derive(Reflect, Debug, Deref, DerefMut, Clone, Copy, PartialEq)]
pub struct PrimaryWindow(pub Entity);

/// Resource containing entity id of the focused window
#[derive(Reflect, Debug, Deref, DerefMut, Clone, Copy, PartialEq)]
pub struct FocusedWindow(pub Entity);

/// Marker for windows that have created by the backend
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct Window;

/// Marker for the window that is currently focused
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowIsFocused;

/// Marker for windows that have decorations enabled
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowDecorated;

/// Marker for windows that are resizeable
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowResizable;

/// Component that describes how the window should be created
#[derive(Component, Reflect, Debug, Clone)]
pub struct WindowDescriptor {
    /// Sets the starting width
    pub width: f32,
    /// Sets the starting height
    pub height: f32,
    /// Sets the starting position
    pub position: Option<Vec2>,
    /// Sets the resize constraints
    pub resize_constraints: WindowResizeConstraints,
    /// Override the scale factor
    pub scale_factor_override: Option<f64>,
    /// Sets the window title
    pub title: String,
    /// Sets the window present mode
    #[doc(alias = "vsync")]
    pub present_mode: WindowPresentMode,
    /// Sets whether the window can resize
    pub resizable: bool,
    /// Sets whether the window should enable decorations, e.g. border, title bar, etc
    pub decorations: bool,
    /// Sets whether the window should hide the touse cursor
    pub cursor_visible: bool,
    /// Sets whether the window should grab the mouse cursor
    pub cursor_locked: bool,
    /// Sets the display mode of the window  
    pub mode: WindowMode,
    /// Sets whether the background of the window should be transparent.
    /// # Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - macOS X: Not working as expected.
    /// - Windows 11: Not working as expected
    /// macOS X transparent works with winit out of the box, so this issue might be related to: <https://github.com/gfx-rs/wgpu/issues/687>
    /// Windows 11 is related to <https://github.com/rust-windowing/winit/issues/2082>
    pub transparent: bool,
    /// Sets the selector used to find locate the <canvas> element,
    /// on `None` a new canvas will be appended to the document body
    #[cfg(target_arch = "wasm32")]
    pub canvas: Option<String>,
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        WindowDescriptor {
            title: "app".to_string(),
            width: 1280.,
            height: 720.,
            position: None,
            resize_constraints: WindowResizeConstraints::default(),
            scale_factor_override: None,
            present_mode: WindowPresentMode::Fifo,
            resizable: true,
            decorations: true,
            cursor_locked: false,
            cursor_visible: true,
            mode: WindowMode::Windowed,
            transparent: false,
            #[cfg(target_arch = "wasm32")]
            canvas: None,
        }
    }
}

/// Window canvas
#[cfg(target_arch = "wasm32")]
#[derive(Component, Reflect, Debug, Clone, PartialEq)]
pub struct WindowCanvas(pub String);

/// Window cursor
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowCursor {
    /// Cursor icon
    pub icon: CursorIcon,
    /// Cursor lock/grab state
    pub locked: bool,
    /// Cursor visibility
    pub visible: bool,
}

/// Window cursor position
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowCursorPosition(pub DVec2);

/// Window handle
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WindowHandle(pub RawWindowHandle);

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}
unsafe impl HasRawWindowHandle for WindowHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0
    }
}

/// Window display mode
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub enum WindowMode {
    /// Creates a window that uses the given size
    Windowed,
    /// Creates a borderless window that uses the full size of the screen
    BorderlessFullscreen,
    /// Creates a fullscreen window that will render at desktop resolution. The app will use the closest supported size
    /// from the given size and scale it to fit the screen.
    SizedFullscreen,
    /// Creates a fullscreen window that uses the maximum supported size
    Fullscreen,
}

/// Window position
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowPosition(pub IVec2);

/// Window presentation mode
///
/// The presentation mode specifies when a frame is presented to the window. The `Fifo`
/// option corresponds to a traditional `VSync`, where the framerate is capped by the
/// display refresh rate. Both `Immediate` and `Mailbox` are low-latency and are not
/// capped by the refresh rate, but may not be available on all platforms. Tearing
/// may be observed with `Immediate` mode, but will not be observed with `Mailbox` or
/// `Fifo`.
///
/// `Immediate` or `Mailbox` will gracefully fallback to `Fifo` when unavailable.
///
/// The presentation mode may be declared in the [`WindowDescriptor`](WindowDescriptor::present_mode)
/// or updated using [`WindowCommands`](crate::WindowCommands::set_present_mode).
#[repr(C)]
#[derive(Component, Reflect, Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[doc(alias = "vsync")]
pub enum WindowPresentMode {
    /// The presentation engine does **not** wait for a vertical blanking period and
    /// the request is presented immediately. This is a low-latency presentation mode,
    /// but visible tearing may be observed. Will fallback to `Fifo` if unavailable on the
    /// selected platform and backend. Not optimal for mobile.
    Immediate = 0,
    /// The presentation engine waits for the next vertical blanking period to update
    /// the current image, but frames may be submitted without delay. This is a low-latency
    /// presentation mode and visible tearing will **not** be observed. Will fallback to `Fifo`
    /// if unavailable on the selected platform and backend. Not optimal for mobile.
    Mailbox = 1,
    /// The presentation engine waits for the next vertical blanking period to update
    /// the current image. The framerate will be capped at the display refresh rate,
    /// corresponding to the `VSync`. Tearing cannot be observed. Optimal for mobile.
    Fifo = 2, // NOTE: The explicit ordinal values mirror wgpu and the vulkan spec.
}

/// Window resize constraints
///
/// The size limits on a window.
/// These values are measured in logical pixels, so the user's
/// scale factor does affect the size limits on the window.
/// Please note that if the window is resizable, then when the window is
/// maximized it may have a size outside of these limits. The functionality
/// required to disable maximizing is not yet exposed by winit.
#[derive(Component, Reflect, Debug, Clone, Copy)]
pub struct WindowResizeConstraints {
    /// Minimum resize width
    pub min_width: f32,
    /// Minimum resize height
    pub min_height: f32,
    /// Maximum resize width
    pub max_width: f32,
    /// Maximum resize height
    pub max_height: f32,
}

impl Default for WindowResizeConstraints {
    fn default() -> Self {
        Self {
            min_width: 180.,
            min_height: 120.,
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
        }
    }
}

impl WindowResizeConstraints {
    /// Check if constraints are valid
    #[must_use]
    pub fn check_constraints(&self) -> Self {
        let WindowResizeConstraints {
            mut min_width,
            mut min_height,
            mut max_width,
            mut max_height,
        } = self;
        min_width = min_width.max(1.);
        min_height = min_height.max(1.);
        if max_width < min_width {
            warn!(
                "The given maximum width {} is smaller than the minimum width {}",
                max_width, min_width
            );
            max_width = min_width;
        }
        if max_height < min_height {
            warn!(
                "The given maximum height {} is smaller than the minimum height {}",
                max_height, min_height
            );
            max_height = min_height;
        }
        WindowResizeConstraints {
            min_width,
            min_height,
            max_width,
            max_height,
        }
    }
}

/// Window resolution
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowResolution {
    physical_width: u32,
    physical_height: u32,
    requested_width: f32,
    requested_height: f32,
    scale_factor_override: Option<f64>,
    scale_factor_backend: f64,
}

impl WindowResolution {
    /// The current logical width of the window's client area.
    #[inline]
    pub fn logical_width(&self) -> f32 {
        (self.physical_width as f64 / self.scale_factor()) as f32
    }

    /// The current logical height of the window's client area.
    #[inline]
    pub fn logical_height(&self) -> f32 {
        (self.physical_height as f64 / self.scale_factor()) as f32
    }

    /// The window's client area width in physical pixels.
    #[inline]
    pub fn physical_width(&self) -> u32 {
        self.physical_width
    }

    /// The window's client area height in physical pixels.
    #[inline]
    pub fn physical_height(&self) -> u32 {
        self.physical_height
    }

    /// The requested window client area width in logical pixels from window
    /// creation or the last call to [`set_resolution`](crate::WindowCommands::set_resolution).
    ///
    /// This may differ from the actual width depending on OS size limits and
    /// the scaling factor for high DPI monitors.
    #[inline]
    pub fn requested_width(&self) -> f32 {
        self.requested_width
    }

    /// The requested window client area height in logical pixels from window
    /// creation or the last call to [`set_resolution`](crate::WindowCommands::set_resolution).
    ///
    /// This may differ from the actual width depending on OS size limits and
    /// the scaling factor for high DPI monitors.
    #[inline]
    pub fn requested_height(&self) -> f32 {
        self.requested_height
    }

    /// The ratio of physical pixels to logical pixels
    ///
    /// `physical_pixels = logical_pixels * scale_factor`
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor_override
            .unwrap_or(self.scale_factor_backend)
    }

    /// The window scale factor as reported by the window backend.
    /// This value is unaffected by [`Self::set_scale_factor_override`].
    #[inline]
    pub fn scale_factor_backend(&self) -> f64 {
        self.scale_factor_backend
    }
    /// Request the OS to resize the window such the the client area matches the
    /// specified width and height.
    ///
    /// Call [`Commands::add`](bevy_ecs::system::Commands::add) with returned command if some
    #[allow(clippy::float_cmp)]
    pub fn set_resolution(&mut self, width: f32, height: f32) -> Option<WindowCommand> {
        if self.requested_width == width && self.requested_height == height {
            return None;
        }

        self.requested_width = width;
        self.requested_height = height;

        Some(WindowCommand::SetResolution(
            (self.requested_width, self.requested_height),
            self.scale_factor(),
        ))
    }

    /// Override the os-reported scaling factor
    ///
    /// Call [`Commands::add`](bevy_ecs::system::Commands::add) on the two returned command if some
    #[allow(clippy::float_cmp)]
    pub fn set_scale_factor_override(
        &mut self,
        scale_factor: Option<f64>,
    ) -> Option<[WindowCommand; 2]> {
        if self.scale_factor_override == scale_factor {
            return None;
        }

        self.scale_factor_override = scale_factor;

        Some([
            WindowCommand::SetScaleFactor(self.scale_factor()),
            WindowCommand::SetResolution(
                (self.requested_width, self.requested_height),
                self.scale_factor(),
            ),
        ])
    }

    /// Update physical size, should only be called by a window backend
    pub fn update_physical_size_from_backend(&mut self, width: u32, height: u32) {
        self.physical_width = width;
        self.physical_height = height;
    }

    /// Update backend scale factor, should only be called by a window backend
    pub fn update_scale_factor_from_backend(&mut self, scale_factor: f64) {
        self.scale_factor_backend = scale_factor;
    }
}

/// Window title
#[derive(Component, Reflect, Debug, Clone, PartialEq)]
pub struct WindowTitle(pub String);
