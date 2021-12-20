use bevy_math::{DVec2, IVec2, Vec2};
use bevy_utils::{tracing::warn, Uuid};
use raw_window_handle::RawWindowHandle;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(Uuid);

impl WindowId {
    pub fn new() -> Self {
        WindowId(Uuid::new_v4())
    }

    pub fn primary() -> Self {
        WindowId(Uuid::from_u128(0))
    }

    pub fn is_primary(&self) -> bool {
        *self == WindowId::primary()
    }
}

use crate::CursorIcon;
use std::fmt;

use crate::raw_window_handle::RawWindowHandleWrapper;

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.to_simple().fmt(f)
    }
}

impl Default for WindowId {
    fn default() -> Self {
        WindowId::primary()
    }
}

/// The size limits on a window.
/// These values are measured in logical pixels, so the user's
/// scale factor does affect the size limits on the window.
/// Please note that if the window is resizable, then when the window is
/// maximized it may have a size outside of these limits. The functionality
/// required to disable maximizing is not yet exposed by winit.
#[derive(Debug, Clone, Copy)]
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
    pub fn check_constraints(&self) -> WindowResizeConstraints {
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

/// An operating system window that can present content and receive user input.
///
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
#[derive(Debug)]
pub struct Window {
    id: WindowId,
    requested_width: f32,
    requested_height: f32,
    physical_width: u32,
    physical_height: u32,
    resize_constraints: WindowResizeConstraints,
    position: Option<IVec2>,
    scale_factor_override: Option<f64>,
    backend_scale_factor: f64,
    title: String,
    vsync: bool,
    resizable: bool,
    decorations: bool,
    cursor_icon: CursorIcon,
    cursor_visible: bool,
    cursor_locked: bool,
    physical_cursor_position: Option<DVec2>,
    raw_window_handle: RawWindowHandleWrapper,
    focused: bool,
    mode: WindowMode,
    #[cfg(target_arch = "wasm32")]
    pub canvas: Option<String>,
    command_queue: Vec<WindowCommand>,
}

#[derive(Debug)]
pub enum WindowCommand {
    SetWindowMode {
        mode: WindowMode,
        resolution: (u32, u32),
    },
    SetTitle {
        title: String,
    },
    SetScaleFactor {
        scale_factor: f64,
    },
    SetResolution {
        logical_resolution: (f32, f32),
        scale_factor: f64,
    },
    SetVsync {
        vsync: bool,
    },
    SetResizable {
        resizable: bool,
    },
    SetDecorations {
        decorations: bool,
    },
    SetCursorLockMode {
        locked: bool,
    },
    SetCursorIcon {
        icon: CursorIcon,
    },
    SetCursorVisibility {
        visible: bool,
    },
    SetCursorPosition {
        position: Vec2,
    },
    SetMaximized {
        maximized: bool,
    },
    SetMinimized {
        minimized: bool,
    },
    SetPosition {
        position: IVec2,
    },
    SetResizeConstraints {
        resize_constraints: WindowResizeConstraints,
    },
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

impl Window {
    pub fn new(
        id: WindowId,
        window_descriptor: &WindowDescriptor,
        physical_width: u32,
        physical_height: u32,
        scale_factor: f64,
        position: Option<IVec2>,
        raw_window_handle: RawWindowHandle,
    ) -> Self {
        Window {
            id,
            requested_width: window_descriptor.width,
            requested_height: window_descriptor.height,
            position,
            physical_width,
            physical_height,
            resize_constraints: window_descriptor.resize_constraints,
            scale_factor_override: window_descriptor.scale_factor_override,
            backend_scale_factor: scale_factor,
            title: window_descriptor.title.clone(),
            vsync: window_descriptor.vsync,
            resizable: window_descriptor.resizable,
            decorations: window_descriptor.decorations,
            cursor_visible: window_descriptor.cursor_visible,
            cursor_locked: window_descriptor.cursor_locked,
            cursor_icon: CursorIcon::Default,
            physical_cursor_position: None,
            raw_window_handle: RawWindowHandleWrapper::new(raw_window_handle),
            focused: true,
            mode: window_descriptor.mode,
            #[cfg(target_arch = "wasm32")]
            canvas: window_descriptor.canvas.clone(),
            command_queue: Vec::new(),
        }
    }

    #[inline]
    pub fn id(&self) -> WindowId {
        self.id
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
    /// creation or the last call to [set_resolution](Window::set_resolution).
    ///
    /// This may differ from the actual width depending on OS size limits and
    /// the scaling factor for high DPI monitors.
    #[inline]
    pub fn requested_width(&self) -> f32 {
        self.requested_width
    }

    /// The requested window client area height in logical pixels from window
    /// creation or the last call to [set_resolution](Window::set_resolution).
    ///
    /// This may differ from the actual width depending on OS size limits and
    /// the scaling factor for high DPI monitors.
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

    /// The window's client resize constraint in logical pixels.
    #[inline]
    pub fn resize_constraints(&self) -> WindowResizeConstraints {
        self.resize_constraints
    }

    /// The window's client position in physical pixels.
    #[inline]
    pub fn position(&self) -> Option<IVec2> {
        self.position
    }

    #[inline]
    pub fn set_maximized(&mut self, maximized: bool) {
        self.command_queue
            .push(WindowCommand::SetMaximized { maximized });
    }

    /// Sets the window to minimized or back.
    ///
    /// # Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - Wayland: Un-minimize is unsupported.
    #[inline]
    pub fn set_minimized(&mut self, minimized: bool) {
        self.command_queue
            .push(WindowCommand::SetMinimized { minimized });
    }

    /// Modifies the position of the window in physical pixels.
    ///
    /// Note that the top-left hand corner of the desktop is not necessarily the same as the screen.
    /// If the user uses a desktop with multiple monitors, the top-left hand corner of the
    /// desktop is the top-left hand corner of the monitor at the top-left of the desktop. This
    /// automatically un-maximizes the window if it's maximized.
    ///
    /// # Platform-specific
    ///
    /// - iOS: Can only be called on the main thread. Sets the top left coordinates of the window in
    ///   the screen space coordinate system.
    /// - Web: Sets the top-left coordinates relative to the viewport.
    /// - Android / Wayland: Unsupported.
    #[inline]
    pub fn set_position(&mut self, position: IVec2) {
        self.command_queue
            .push(WindowCommand::SetPosition { position })
    }

    /// Modifies the minimum and maximum window bounds for resizing in logical pixels.
    #[inline]
    pub fn set_resize_constraints(&mut self, resize_constraints: WindowResizeConstraints) {
        self.command_queue
            .push(WindowCommand::SetResizeConstraints { resize_constraints });
    }

    /// Request the OS to resize the window such the the client area matches the
    /// specified width and height.
    #[allow(clippy::float_cmp)]
    pub fn set_resolution(&mut self, width: f32, height: f32) {
        if self.requested_width == width && self.requested_height == height {
            return;
        }

        self.requested_width = width;
        self.requested_height = height;
        self.command_queue.push(WindowCommand::SetResolution {
            logical_resolution: (self.requested_width, self.requested_height),
            scale_factor: self.scale_factor(),
        });
    }

    /// Override the os-reported scaling factor
    #[allow(clippy::float_cmp)]
    pub fn set_scale_factor_override(&mut self, scale_factor: Option<f64>) {
        if self.scale_factor_override == scale_factor {
            return;
        }

        self.scale_factor_override = scale_factor;
        self.command_queue.push(WindowCommand::SetScaleFactor {
            scale_factor: self.scale_factor(),
        });
        self.command_queue.push(WindowCommand::SetResolution {
            logical_resolution: (self.requested_width, self.requested_height),
            scale_factor: self.scale_factor(),
        });
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

    #[allow(missing_docs)]
    #[inline]
    pub fn update_actual_position_from_backend(&mut self, position: IVec2) {
        self.position = Some(position);
    }

    /// The ratio of physical pixels to logical pixels
    ///
    /// `physical_pixels = logical_pixels * scale_factor`
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor_override
            .unwrap_or(self.backend_scale_factor)
    }

    /// The window scale factor as reported by the window backend.
    /// This value is unaffected by scale_factor_override.
    #[inline]
    pub fn backend_scale_factor(&self) -> f64 {
        self.backend_scale_factor
    }

    #[inline]
    pub fn scale_factor_override(&self) -> Option<f64> {
        self.scale_factor_override
    }

    #[inline]
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title.to_string();
        self.command_queue.push(WindowCommand::SetTitle { title });
    }

    #[inline]
    pub fn vsync(&self) -> bool {
        self.vsync
    }

    #[inline]
    pub fn set_vsync(&mut self, vsync: bool) {
        self.vsync = vsync;
        self.command_queue.push(WindowCommand::SetVsync { vsync });
    }

    #[inline]
    pub fn resizable(&self) -> bool {
        self.resizable
    }

    pub fn set_resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
        self.command_queue
            .push(WindowCommand::SetResizable { resizable });
    }

    #[inline]
    pub fn decorations(&self) -> bool {
        self.decorations
    }

    pub fn set_decorations(&mut self, decorations: bool) {
        self.decorations = decorations;
        self.command_queue
            .push(WindowCommand::SetDecorations { decorations });
    }

    #[inline]
    pub fn cursor_locked(&self) -> bool {
        self.cursor_locked
    }

    pub fn set_cursor_lock_mode(&mut self, lock_mode: bool) {
        self.cursor_locked = lock_mode;
        self.command_queue
            .push(WindowCommand::SetCursorLockMode { locked: lock_mode });
    }

    #[inline]
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    pub fn set_cursor_visibility(&mut self, visibile_mode: bool) {
        self.cursor_visible = visibile_mode;
        self.command_queue.push(WindowCommand::SetCursorVisibility {
            visible: visibile_mode,
        });
    }

    #[inline]
    pub fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon
    }

    pub fn set_cursor_icon(&mut self, icon: CursorIcon) {
        self.command_queue
            .push(WindowCommand::SetCursorIcon { icon });
    }

    /// The current mouse position, in physical pixels.
    #[inline]
    pub fn physical_cursor_position(&self) -> Option<DVec2> {
        self.physical_cursor_position
    }

    /// The current mouse position, in logical pixels, taking into account the screen scale factor.
    #[inline]
    #[doc(alias = "mouse position")]
    pub fn cursor_position(&self) -> Option<Vec2> {
        self.physical_cursor_position
            .map(|p| (p / self.scale_factor()).as_vec2())
    }

    pub fn set_cursor_position(&mut self, position: Vec2) {
        self.command_queue
            .push(WindowCommand::SetCursorPosition { position });
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn update_focused_status_from_backend(&mut self, focused: bool) {
        self.focused = focused;
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn update_cursor_physical_position_from_backend(&mut self, cursor_position: Option<DVec2>) {
        self.physical_cursor_position = cursor_position;
    }

    #[inline]
    pub fn mode(&self) -> WindowMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: WindowMode) {
        self.mode = mode;
        self.command_queue.push(WindowCommand::SetWindowMode {
            mode,
            resolution: (self.physical_width, self.physical_height),
        });
    }

    #[inline]
    pub fn drain_commands(&mut self) -> impl Iterator<Item = WindowCommand> + '_ {
        self.command_queue.drain(..)
    }

    #[inline]
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn raw_window_handle(&self) -> RawWindowHandleWrapper {
        self.raw_window_handle.clone()
    }
}

#[derive(Debug, Clone)]
pub struct WindowDescriptor {
    pub width: f32,
    pub height: f32,
    pub position: Option<Vec2>,
    pub resize_constraints: WindowResizeConstraints,
    pub scale_factor_override: Option<f64>,
    pub title: String,
    pub vsync: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub cursor_visible: bool,
    pub cursor_locked: bool,
    pub mode: WindowMode,
    /// Sets whether the background of the window should be transparent.
    /// # Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - macOS X: Not working as expected.
    /// - Windows 11: Not working as expected
    /// macOS X transparent works with winit out of the box, so this issue might be related to: <https://github.com/gfx-rs/wgpu/issues/687>
    /// Windows 11 is related to <https://github.com/rust-windowing/winit/issues/2082>
    pub transparent: bool,
    #[cfg(target_arch = "wasm32")]
    pub canvas: Option<String>,
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        WindowDescriptor {
            title: "bevy".to_string(),
            width: 1280.,
            height: 720.,
            position: None,
            resize_constraints: WindowResizeConstraints::default(),
            scale_factor_override: None,
            vsync: true,
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
