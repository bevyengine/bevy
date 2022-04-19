use bevy_ecs::{
    prelude::{Component, With},
    query::WorldQuery,
};
use bevy_math::{DVec2, IVec2, Vec2};
use bevy_reflect::Reflect;
use bevy_utils::tracing::warn;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};

use crate::CursorIcon;

#[derive(Component, Reflect, Debug, Clone)]
pub struct WindowDescriptor {
    pub width: f32,
    pub height: f32,
    pub position: Option<Vec2>,
    pub resize_constraints: WindowResizeConstraints,
    pub scale_factor_override: Option<f64>,
    pub title: String,
    #[doc(alias = "vsync")]
    pub present_mode: PresentMode,
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
            #[cfg(target_arch = "wasm32")]
            canvas: None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub struct WindowCanvas(String);

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
#[derive(Component, Reflect, Copy, Clone, Debug, PartialEq, Eq, Hash)]
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
    SetPresentMode {
        present_mode: PresentMode,
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

#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowCursor {
    cursor_icon: CursorIcon,
    cursor_visible: bool,
    cursor_locked: bool,
}

#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowCursorPosition(DVec2);

#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowPosition(IVec2);

/// Defines the way a window is displayed
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

#[derive(Component, Reflect, Debug, Clone, PartialEq)]
pub struct WindowTitle(String);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct WindowHandle(RawWindowHandle);

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}

unsafe impl HasRawWindowHandle for WindowHandle {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0
    }
}

/// The size limits on a window.
/// These values are measured in logical pixels, so the user's
/// scale factor does affect the size limits on the window.
/// Please note that if the window is resizable, then when the window is
/// maximized it may have a size outside of these limits. The functionality
/// required to disable maximizing is not yet exposed by winit.
#[derive(Component, Reflect, Debug, Clone, Copy)]
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
}

#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowResolution {
    pub physical_width: u32,
    pub physical_height: u32,
    pub requested_width: f32,
    pub requested_height: f32,
    pub scale_factor: Option<f64>,
    pub backend_scale_factor: f64,
}

impl WindowResolution {
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

    /// Request the OS to resize the window such the the client area matches the
    /// specified width and height.
    #[allow(clippy::float_cmp)]
    pub fn set_resolution(&mut self, width: f32, height: f32) -> Option<WindowCommand> {
        if self.requested_width == width && self.requested_height == height {
            return None;
        }

        self.requested_width = width;
        self.requested_height = height;
        Some(WindowCommand::SetResolution {
            logical_resolution: (self.requested_width, self.requested_height),
            scale_factor: self.scale_factor(),
        })
    }

    /// Override the os-reported scaling factor
    #[allow(clippy::float_cmp)]
    pub fn set_scale_factor_override(
        &mut self,
        scale_factor: Option<f64>,
    ) -> Option<[WindowCommand; 2]> {
        if self.scale_factor == scale_factor {
            return None;
        }
        self.scale_factor = scale_factor;
        Some([
            WindowCommand::SetScaleFactor {
                scale_factor: self.scale_factor(),
            },
            WindowCommand::SetResolution {
                logical_resolution: (self.requested_width, self.requested_height),
                scale_factor: self.scale_factor(),
            },
        ])
    }

    /// The ratio of physical pixels to logical pixels
    ///
    /// `physical_pixels = logical_pixels * scale_factor`
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor.unwrap_or(self.backend_scale_factor)
    }
}

/// Marker for the window that is focused
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowFocused;

/// Marker for windows that are resizeable
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowResizable;

/// Marker for windows that have decorations enabled
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq)]
pub struct WindowDecorated;

#[derive(WorldQuery)]
pub struct WindowWorldQuery<'w> {
    cursor: &'w WindowCursor,
    cursor_position: Option<&'w WindowCursorPosition>,
    decorations: With<WindowDecorated>,
    focused: With<WindowFocused>,
    handle: &'w WindowHandle,
    position: Option<&'w WindowPosition>,
    resize_constraints: Option<&'w WindowResizeConstraints>,
    resizeable: With<WindowResizable>,
    resolution: &'w WindowResolution,
    title: Option<&'w WindowTitle>,
}
