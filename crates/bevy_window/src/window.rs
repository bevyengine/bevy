use bevy_math::{IVec2, Vec2};
use bevy_utils::Uuid;

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

use std::fmt;

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
    position: Option<IVec2>,
    scale_factor_override: Option<f64>,
    backend_scale_factor: f64,
    title: String,
    vsync: bool,
    resizable: bool,
    decorations: bool,
    cursor_visible: bool,
    cursor_locked: bool,
    cursor_position: Option<Vec2>,
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
}

/// Defines the way a window is displayed
/// The use_size option that is used in the Fullscreen variant
/// defines whether a videomode is chosen that best fits the width and height
/// in the Window structure, or if these are ignored.
/// E.g. when use_size is set to false the best video mode possible is chosen.
#[derive(Debug, Clone, Copy)]
pub enum WindowMode {
    Windowed,
    BorderlessFullscreen,
    Fullscreen { use_size: bool },
}

impl Window {
    pub fn new(
        id: WindowId,
        window_descriptor: &WindowDescriptor,
        physical_width: u32,
        physical_height: u32,
        scale_factor: f64,
        position: Option<IVec2>,
    ) -> Self {
        Window {
            id,
            requested_width: window_descriptor.width,
            requested_height: window_descriptor.height,
            position,
            physical_width,
            physical_height,
            scale_factor_override: window_descriptor.scale_factor_override,
            backend_scale_factor: scale_factor,
            title: window_descriptor.title.clone(),
            vsync: window_descriptor.vsync,
            resizable: window_descriptor.resizable,
            decorations: window_descriptor.decorations,
            cursor_visible: window_descriptor.cursor_visible,
            cursor_locked: window_descriptor.cursor_locked,
            cursor_position: None,
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
    /// Note that the top-left hand corner of the desktop is not necessarily the same as the screen. If the user uses a desktop with multiple monitors,
    /// the top-left hand corner of the desktop is the top-left hand corner of the monitor at the top-left of the desktop. This automatically un-maximizes
    /// the window if it's maximized.
    ///
    /// # Platform-specific
    ///
    /// - iOS: Can only be called on the main thread. Sets the top left coordinates of the window in the screen space coordinate system.
    /// - Web: Sets the top-left coordinates relative to the viewport.
    /// - Android / Wayland: Unsupported.
    #[inline]
    pub fn set_position(&mut self, position: IVec2) {
        self.command_queue
            .push(WindowCommand::SetPosition { position })
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
    pub fn cursor_position(&self) -> Option<Vec2> {
        self.cursor_position
    }

    pub fn set_cursor_position(&mut self, position: Vec2) {
        self.command_queue
            .push(WindowCommand::SetCursorPosition { position });
    }

    #[allow(missing_docs)]
    #[inline]
    pub fn update_cursor_position_from_backend(&mut self, cursor_position: Option<Vec2>) {
        self.cursor_position = cursor_position;
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
}

#[derive(Debug, Clone)]
pub struct WindowDescriptor {
    pub width: f32,
    pub height: f32,
    pub scale_factor_override: Option<f64>,
    pub title: String,
    pub vsync: bool,
    pub resizable: bool,
    pub decorations: bool,
    pub cursor_visible: bool,
    pub cursor_locked: bool,
    pub mode: WindowMode,
    #[cfg(target_arch = "wasm32")]
    pub canvas: Option<String>,
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        WindowDescriptor {
            title: "bevy".to_string(),
            width: 1280.,
            height: 720.,
            scale_factor_override: None,
            vsync: true,
            resizable: true,
            decorations: true,
            cursor_locked: false,
            cursor_visible: true,
            mode: WindowMode::Windowed,
            #[cfg(target_arch = "wasm32")]
            canvas: None,
        }
    }
}
