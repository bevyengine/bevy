use bevy_ecs::{
    entity::Entity,
    prelude::{Bundle, Component},
    system::{Command, Commands},
};
use bevy_math::{DVec2, IVec2, Vec2};
use bevy_utils::{tracing::warn, Uuid};
use raw_window_handle::RawWindowHandle;

use crate::{raw_window_handle::RawWindowHandleWrapper, WindowFocused};
use crate::CursorIcon;

/// Presentation mode for a window.
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
/// or updated on a [`Window`](Window::set_present_mode).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[doc(alias = "vsync")]
pub enum PresentMode {
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

/// Defines the way a window is displayed
#[derive(Debug, Clone, Copy, PartialEq)]
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

// This should only be used by the window backend, so maybe it should not be a bundle for those reasons
// The window backend is responsible for spawning the correct components that together define a whole window
#[derive(Bundle)]
pub struct WindowBundle {
    window: Window,
    cursor: WindowCursor,
    cursor_position: WindowCursorPosition,
    handle: WindowHandle,
    presentation: WindowPresentation,
    mode: WindowModeComponent,
    position: WindowPosition,
    resolution: WindowResolution,
    title: WindowTitle,
    canvas: WindowCanvas,
    resize_constraints: WindowResizeConstraints,
    focused: WindowCurrentlyFocused,
}

/// The size limits on a window.
/// These values are measured in logical pixels, so the user's
/// scale factor does affect the size limits on the window.
/// Please note that if the window is resizable, then when the window is
/// maximized it may have a size outside of these limits. The functionality
/// required to disable maximizing is not yet exposed by winit.
#[derive(Debug, Clone, Copy, Component)]
pub struct WindowResizeConstraints {
    pub min_width: f32,
    pub min_height: f32,
    pub max_width: f32,
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

    // /// The window's client resize constraint in logical pixels.
    // #[inline]
    // pub fn resize_constraints(&self) -> WindowResizeConstraints {
    //     self.resize_constraints
    // }
}

/// A marker component on an entity containing a window
#[derive(Debug, Component)]
pub struct Window;

#[derive(Component)]
pub struct WindowCursor {
    cursor_icon: CursorIcon,
    cursor_visible: bool,
    cursor_locked: bool,
}

impl WindowCursor {
    #[inline]
    pub fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon
    }

    #[inline]
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    #[inline]
    pub fn cursor_locked(&self) -> bool {
        self.cursor_locked
    }

    pub fn set_icon_from_backend(&mut self, icon: CursorIcon) {
        self.cursor_icon = icon;
    }

    pub fn set_visible_from_backend(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    pub fn set_locked_from_backend(&mut self, locked: bool) {
        self.cursor_locked = locked;
    }
}

#[derive(Component)]
pub struct WindowCursorPosition {
    // TODO: Docs
    // This is None if the cursor has left the window
    physical_cursor_position: Option<DVec2>,
}

impl WindowCursorPosition {
    /// The current mouse position, in physical pixels.
    #[inline]
    pub fn physical_cursor_position(&self) -> Option<DVec2> {
        self.physical_cursor_position
    }

    // TODO: Docs
    pub fn update_position_from_backend(&mut self, position: Option<DVec2>) {
        // TODO: Fix type inconsitencies
        self.physical_cursor_position = position;
    }
}

// TODO: Figure out how this connects to everything
#[derive(Component)]
pub struct WindowHandle {
    // TODo: What should be creating and setting this?
    raw_window_handle: RawWindowHandleWrapper,
}

impl WindowHandle {
    pub fn raw_window_handle(&self) -> RawWindowHandleWrapper {
        self.raw_window_handle.clone()
    }
}

// TODO: Find better name
#[derive(Component)]
pub struct WindowPresentation {
    present_mode: PresentMode,
}

impl WindowPresentation {
    #[inline]
    #[doc(alias = "vsync")]
    pub fn present_mode(&self) -> PresentMode {
        self.present_mode
    }

    pub fn update_present_mode_from_backend(&mut self, present_mode: PresentMode) {
        self.present_mode = present_mode;
    }
}

// TODO: Find better name
#[derive(Component)]
pub struct WindowModeComponent {
    mode: WindowMode,
}

impl WindowModeComponent {
    #[inline]
    pub fn mode(&self) -> WindowMode {
        self.mode
    }

    pub fn update_mode_from_backend(&mut self, mode: WindowMode) {
        self.mode = mode;
    }
}

#[derive(Component)]
pub struct WindowPosition {
    position: Option<IVec2>,
}

impl WindowPosition {
    /// The window's client position in physical pixels.
    #[inline]
    pub fn position(&self) -> Option<IVec2> {
        self.position
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn update_actual_position_from_backend(&mut self, position: IVec2) {
        self.position = Some(position);
    }
}

/// ## Window Sizes
///
/// There are three sizes associated with a window. The physical size which is
/// the height and width in physical pixels on the monitor. The logical size
/// which is the physical size scaled by an operating system provided factor to
/// account for monitors with differing pixel densities or user preference. And
/// the requested size, measured in logical pixels, which is the value submitted
/// to the API when creating the window, or requesting that it be resized.
///
/// The actual size, in logical pixels, of the window may not match the
/// requested size due to operating system limits on the window size, or the
/// quantization of the logical size when converting the physical size to the
/// logical size through the scaling factor.
// TODO: Make sure this is used correctly
#[derive(Component)]
pub struct WindowResolution {
    requested_width: f32,
    requested_height: f32,
    physical_width: u32,
    physical_height: u32,
    scale_factor_override: Option<f64>,
    backend_scale_factor: f64,
}

impl WindowResolution {
    /// The ratio of physical pixels to logical pixels
    ///
    /// `physical_pixels = logical_pixels * scale_factor`
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor_override
            .unwrap_or(self.backend_scale_factor)
    }

    /// The window scale factor as reported by the window backend.
    /// This value is unaffected by [`scale_factor_override`](Window::scale_factor_override).
    #[inline]
    pub fn backend_scale_factor(&self) -> f64 {
        self.backend_scale_factor
    }

    #[inline]
    pub fn scale_factor_override(&self) -> Option<f64> {
        self.scale_factor_override
    }

    /// The current logical width of the window's client area.
    #[inline]
    pub fn width(&self) -> f32 {
        (self.physical_width as f64 / self.scale_factor()) as f32
    }

    /// The current logical height of the window's client area.
    #[inline]
    pub fn height(&self) -> f32 {
        (self.physical_height as f64 / self.scale_factor()) as f32
    }

    /// The requested window client area width in logical pixels from window
    /// creation or the last call to [`set_resolution`](Window::set_resolution).
    ///
    /// This may differ from the actual width depending on OS size limits and
    /// the scaling factor for high DPI monitors.
    // TODO: This is never set
    #[inline]
    pub fn requested_width(&self) -> f32 {
        self.requested_width
    }

    /// The requested window client area height in logical pixels from window
    /// creation or the last call to [`set_resolution`](Window::set_resolution).
    ///
    /// This may differ from the actual width depending on OS size limits and
    /// the scaling factor for high DPI monitors.
    // TODO: This is never set
    #[inline]
    pub fn requested_height(&self) -> f32 {
        self.requested_height
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

    #[allow(missing_docs)]
    #[inline]
    pub fn update_scale_factor_from_backend(&mut self, scale_factor: f64) {
        self.backend_scale_factor = scale_factor;
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn update_actual_size_from_backend(&mut self, physical_width: u32, physical_height: u32) {
        self.physical_width = physical_width;
        self.physical_height = physical_height;
    }
}

#[derive(Component)]
pub struct WindowTitle {
    title: String,
}

impl WindowTitle {
    #[inline]
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn update_title_from_backend(&mut self, title: String) {
        self.title = title;
    }
}

#[derive(Component)]
pub struct WindowDecorated;

#[derive(Component)]
pub struct WindowCurrentlyFocused;

#[derive(Component)]
pub struct WindowResizable;

#[derive(Component)]
pub struct WindowTransparent;

#[derive(Component)]
pub struct WindowMinimized;

#[derive(Component)]
pub struct WindowMaximized;

#[derive(Component)]
pub struct WindowCanvas {
    canvas: Option<String>,
    fit_canvas_to_parent: bool,
}

impl WindowCanvas {
    /// The "html canvas" element selector. If set, this selector will be used to find a matching html canvas element,
    /// rather than creating a new one.   
    /// Uses the [CSS selector format](https://developer.mozilla.org/en-US/docs/Web/API/Document/querySelector).
    ///
    /// This value has no effect on non-web platforms.
    #[inline]
    pub fn canvas(&self) -> Option<&str> {
        self.canvas.as_deref()
    }

    /// Whether or not to fit the canvas element's size to its parent element's size.
    ///
    /// **Warning**: this will not behave as expected for parents that set their size according to the size of their
    /// children. This creates a "feedback loop" that will result in the canvas growing on each resize. When using this
    /// feature, ensure the parent's size is not affected by its children.
    ///
    /// This value has no effect on non-web platforms.
    #[inline]
    pub fn fit_canvas_to_parent(&self) -> bool {
        self.fit_canvas_to_parent
    }
}

//     /// Request the OS to resize the window such the the client area matches the
//     /// specified width and height.

/// Describes the information needed for creating a window.
///
/// This should be set up before adding the [`WindowPlugin`](crate::WindowPlugin).
/// Most of these settings can also later be configured through the [`Window`](crate::Window) resource.
///
/// See [`examples/window/window_settings.rs`] for usage.
///
/// [`examples/window/window_settings.rs`]: https://github.com/bevyengine/bevy/blob/latest/examples/window/window_settings.rs
#[derive(Debug, Clone)]
pub struct WindowDescriptor {
    /// The requested logical width of the window's client area.
    /// May vary from the physical width due to different pixel density on different monitors.
    pub width: f32,
    /// The requested logical height of the window's client area.
    /// May vary from the physical height due to different pixel density on different monitors.
    pub height: f32,
    /// The position on the screen that the window will be centered at.
    /// If set to `None`, some platform-specific position will be chosen.
    pub position: Option<Vec2>,
    /// Sets minimum and maximum resize limits.
    pub resize_constraints: WindowResizeConstraints,
    /// Overrides the window's ratio of physical pixels to logical pixels.
    /// If there are some scaling problems on X11 try to set this option to `Some(1.0)`.
    pub scale_factor_override: Option<f64>,
    /// Sets the title that displays on the window top bar, on the system task bar and other OS specific places.
    /// ## Platform-specific
    /// - Web: Unsupported.
    pub title: String,
    /// Controls when a frame is presented to the screen.
    #[doc(alias = "vsync")]
    pub present_mode: PresentMode,
    /// Sets whether the window is resizable.
    /// ## Platform-specific
    /// - iOS / Android / Web: Unsupported.
    pub resizable: bool,
    /// Sets whether the window should have borders and bars.
    pub decorations: bool,
    /// Sets whether the cursor is visible when the window has focus.
    pub cursor_visible: bool,
    /// Sets whether the window locks the cursor inside its borders when the window has focus.
    pub cursor_locked: bool,
    /// Sets the [`WindowMode`](crate::WindowMode).
    pub mode: WindowMode,
    /// Sets whether the background of the window should be transparent.
    /// ## Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - macOS X: Not working as expected.
    /// - Windows 11: Not working as expected
    /// macOS X transparent works with winit out of the box, so this issue might be related to: <https://github.com/gfx-rs/wgpu/issues/687>
    /// Windows 11 is related to <https://github.com/rust-windowing/winit/issues/2082>
    pub transparent: bool,
    /// The "html canvas" element selector. If set, this selector will be used to find a matching html canvas element,
    /// rather than creating a new one.   
    /// Uses the [CSS selector format](https://developer.mozilla.org/en-US/docs/Web/API/Document/querySelector).
    ///
    /// This value has no effect on non-web platforms.
    pub canvas: Option<String>,
    /// Whether or not to fit the canvas element's size to its parent element's size.
    ///
    /// **Warning**: this will not behave as expected for parents that set their size according to the size of their
    /// children. This creates a "feedback loop" that will result in the canvas growing on each resize. When using this
    /// feature, ensure the parent's size is not affected by its children.
    ///
    /// This value has no effect on non-web platforms.
    pub fit_canvas_to_parent: bool,
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
            present_mode: PresentMode::Fifo,
            resizable: true,
            decorations: true,
            cursor_locked: false,
            cursor_visible: true,
            mode: WindowMode::Windowed,
            transparent: false,
            canvas: None,
            fit_canvas_to_parent: false,
        }
    }
}
