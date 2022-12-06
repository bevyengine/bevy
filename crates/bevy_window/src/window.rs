use bevy_math::{DVec2, IVec2, UVec2, Vec2};
use bevy_reflect::{FromReflect, Reflect};
use bevy_utils::{tracing::warn, Uuid};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Reflect, FromReflect)]
#[reflect_value(PartialEq, Hash)]
/// A unique ID for a [`Window`].
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowId(Uuid);

/// Presentation mode for a window.
///
/// The presentation mode specifies when a frame is presented to the window. The `Fifo`
/// option corresponds to a traditional `VSync`, where the framerate is capped by the
/// display refresh rate. Both `Immediate` and `Mailbox` are low-latency and are not
/// capped by the refresh rate, but may not be available on all platforms. Tearing
/// may be observed with `Immediate` mode, but will not be observed with `Mailbox` or
/// `Fifo`.
///
/// `AutoVsync` or `AutoNoVsync` will gracefully fallback to `Fifo` when unavailable.
///
/// `Immediate` or `Mailbox` will panic if not supported by the platform.
///
/// The presentation mode may be declared in the [`WindowDescriptor`](WindowDescriptor) using [`WindowDescriptor::present_mode`](WindowDescriptor::present_mode)
/// or updated on a [`Window`](Window) using [`set_present_mode`](Window::set_present_mode).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(alias = "vsync")]
pub enum PresentMode {
    /// Chooses FifoRelaxed -> Fifo based on availability.
    ///
    /// Because of the fallback behavior, it is supported everywhere.
    AutoVsync = 0,
    /// Chooses Immediate -> Mailbox -> Fifo (on web) based on availability.
    ///
    /// Because of the fallback behavior, it is supported everywhere.
    AutoNoVsync = 1,
    /// The presentation engine does **not** wait for a vertical blanking period and
    /// the request is presented immediately. This is a low-latency presentation mode,
    /// but visible tearing may be observed. Not optimal for mobile.
    ///
    /// Selecting this variant will panic if not supported, it is preferred to use
    /// [`PresentMode::AutoNoVsync`].
    Immediate = 2,
    /// The presentation engine waits for the next vertical blanking period to update
    /// the current image, but frames may be submitted without delay. This is a low-latency
    /// presentation mode and visible tearing will **not** be observed. Not optimal for mobile.
    ///
    /// Selecting this variant will panic if not supported, it is preferred to use
    /// [`PresentMode::AutoNoVsync`].
    Mailbox = 3,
    /// The presentation engine waits for the next vertical blanking period to update
    /// the current image. The framerate will be capped at the display refresh rate,
    /// corresponding to the `VSync`. Tearing cannot be observed. Optimal for mobile.
    Fifo = 4, // NOTE: The explicit ordinal values mirror wgpu.
}

/// Specifies how the alpha channel of the textures should be handled during compositing.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum CompositeAlphaMode {
    /// Chooses either `Opaque` or `Inherit` automatically，depending on the
    /// `alpha_mode` that the current surface can support.
    Auto = 0,
    /// The alpha channel, if it exists, of the textures is ignored in the
    /// compositing process. Instead, the textures is treated as if it has a
    /// constant alpha of 1.0.
    Opaque = 1,
    /// The alpha channel, if it exists, of the textures is respected in the
    /// compositing process. The non-alpha channels of the textures are
    /// expected to already be multiplied by the alpha channel by the
    /// application.
    PreMultiplied = 2,
    /// The alpha channel, if it exists, of the textures is respected in the
    /// compositing process. The non-alpha channels of the textures are not
    /// expected to already be multiplied by the alpha channel by the
    /// application; instead, the compositor will multiply the non-alpha
    /// channels of the texture by the alpha channel during compositing.
    PostMultiplied = 3,
    /// The alpha channel, if it exists, of the textures is unknown for processing
    /// during compositing. Instead, the application is responsible for setting
    /// the composite alpha blending mode using native WSI command. If not set,
    /// then a platform-specific default will be used.
    Inherit = 4,
}

impl WindowId {
    /// Creates a new [`WindowId`].
    pub fn new() -> Self {
        WindowId(Uuid::new_v4())
    }
    /// The [`WindowId`] for the primary window.
    pub const fn primary() -> Self {
        WindowId(Uuid::from_u128(0))
    }
    /// Get whether or not this [`WindowId`] is for the primary window.
    pub fn is_primary(&self) -> bool {
        *self == WindowId::primary()
    }
}

use crate::CursorIcon;
use std::fmt;

use crate::raw_handle::RawHandleWrapper;

impl fmt::Display for WindowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.as_simple().fmt(f)
    }
}

impl Default for WindowId {
    fn default() -> Self {
        WindowId::primary()
    }
}

/// The size limits on a window.
///
/// These values are measured in logical pixels, so the user's
/// scale factor does affect the size limits on the window.
/// Please note that if the window is resizable, then when the window is
/// maximized it may have a size outside of these limits. The functionality
/// required to disable maximizing is not yet exposed by winit.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
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

/// An operating system window that can present content and receive user input.
///
/// To create a window, use a [`EventWriter<CreateWindow>`](`crate::CreateWindow`).
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
///
/// ## Accessing a `Window` from a system
///
/// To access a `Window` from a system, use [`bevy_ecs::change_detection::ResMut`]`<`[`crate::Windows`]`>`.
///
/// ### Example
/// ```no_run
/// # use bevy_app::App;
/// # use bevy_window::Windows;
/// # use bevy_ecs::change_detection::ResMut;
/// # fn main(){
/// # App::new().add_system(access_window_system).run();
/// # }
/// fn access_window_system(mut windows: ResMut<Windows>){
///     for mut window in windows.iter_mut() {
///         window.set_title(String::from("Yay, I'm a window!"));
///     }
/// }
/// ```
/// To test code that uses `Window`s, one can test it with varying `Window` parameters by
/// creating `WindowResizeConstraints` or `WindowDescriptor` structures.
/// values by setting
///
/// ```
/// # use bevy_utils::default;
/// # use bevy_window::{Window, WindowCommand, WindowDescriptor, WindowId, WindowResizeConstraints};
/// # fn compute_window_area(w: &Window) -> f32 {
/// #   w.width() * w.height()
/// # }
/// # fn grow_window_to_text_size(_window: &mut Window, _text: &str) {}
/// # fn set_new_title(window: &mut Window, text: String) { window.set_title(text); }
/// # fn a_window_resize_test() {
/// let resize_constraints = WindowResizeConstraints {
///                             min_width: 400.0,
///                             min_height: 300.0,
///                             max_width: 1280.0,
///                             max_height: 1024.0,
/// };
/// let window_descriptor = WindowDescriptor {
///     width: 800.0,
///     height: 600.0,
///     resizable: true,
///     resize_constraints,
///     ..default()
/// };
/// let mut window = Window::new(
///    WindowId::new(),
///    &window_descriptor,
///    100, // physical_width
///    100, // physical_height
///    1.0, // scale_factor
///    None, None);
///
/// let area = compute_window_area(&window);
/// assert_eq!(area, 100.0 * 100.0);
///
/// grow_window_to_text_size(&mut window, "very long text that does not wrap");
/// assert_eq!(window.physical_width(), window.requested_width() as u32);
/// grow_window_to_text_size(&mut window, "very long text that does wrap, creating a maximum width window");
/// assert_eq!(window.physical_width(), window.requested_width() as u32);
///
/// set_new_title(&mut window, "new title".to_string());
/// let mut found_command = false;
/// for command in window.drain_commands() {
///     if command == (WindowCommand::SetTitle{ title: "new title".to_string() }) {
///         found_command = true;
///         break;
///     }
/// }
/// assert_eq!(found_command, true);
/// }
/// ```
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
    present_mode: PresentMode,
    resizable: bool,
    decorations: bool,
    cursor_icon: CursorIcon,
    cursor_visible: bool,
    cursor_grab_mode: CursorGrabMode,
    hittest: bool,
    physical_cursor_position: Option<DVec2>,
    raw_handle: Option<RawHandleWrapper>,
    focused: bool,
    mode: WindowMode,
    canvas: Option<String>,
    fit_canvas_to_parent: bool,
    command_queue: Vec<WindowCommand>,
    alpha_mode: CompositeAlphaMode,
    always_on_top: bool,
}
/// A command to be sent to a window.
///
/// Bevy apps don't interact with this `enum` directly. Instead, they should use the methods on [`Window`].
/// This `enum` is meant for authors of windowing plugins. See the documentation on [`crate::WindowPlugin`] for more information.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum WindowCommand {
    /// Set the window's [`WindowMode`].
    SetWindowMode {
        mode: WindowMode,
        resolution: UVec2,
    },
    /// Set the window's title.
    SetTitle {
        title: String,
    },
    /// Set the window's scale factor.
    SetScaleFactor {
        scale_factor: f64,
    },
    /// Set the window's resolution.
    SetResolution {
        logical_resolution: Vec2,
        scale_factor: f64,
    },
    /// Set the window's [`PresentMode`].
    SetPresentMode {
        present_mode: PresentMode,
    },
    /// Set whether or not the window is resizable.
    SetResizable {
        resizable: bool,
    },
    /// Set whether or not the window has decorations.
    ///
    /// Examples of decorations include the close, full screen, and minimize buttons
    SetDecorations {
        decorations: bool,
    },
    /// Set whether or not the cursor's position is locked.
    SetCursorGrabMode {
        grab_mode: CursorGrabMode,
    },
    /// Set the cursor's [`CursorIcon`].
    SetCursorIcon {
        icon: CursorIcon,
    },
    /// Set whether or not the cursor is visible.
    SetCursorVisibility {
        visible: bool,
    },
    /// Set the cursor's position.
    SetCursorPosition {
        position: Vec2,
    },
    /// Set whether or not mouse events within *this* window are captured, or fall through to the Window below.
    SetCursorHitTest {
        hittest: bool,
    },
    /// Set whether or not the window is maximized.
    SetMaximized {
        maximized: bool,
    },
    /// Set whether or not the window is minimized.
    SetMinimized {
        minimized: bool,
    },
    /// Set the window's position on the selected monitor.
    SetPosition {
        monitor_selection: MonitorSelection,
        position: IVec2,
    },
    /// Sets the position of the window to be in the center of the selected monitor.
    Center(MonitorSelection),
    /// Set the window's [`WindowResizeConstraints`]
    SetResizeConstraints {
        resize_constraints: WindowResizeConstraints,
    },
    /// Set whether the window is always on top.
    SetAlwaysOnTop {
        always_on_top: bool,
    },
    Close,
}

/// Defines if and how the cursor is grabbed.
///
/// Use this enum with [`Window::set_cursor_grab_mode`] to grab the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum CursorGrabMode {
    /// The cursor can freely leave the window.
    None,
    /// The cursor is confined to the window area.
    Confined,
    /// The cursor is locked inside the window area to a certain position.
    Locked,
}

/// Defines the way a window is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum WindowMode {
    /// Creates a window that uses the given size.
    Windowed,
    /// Creates a borderless window that uses the full size of the screen.
    BorderlessFullscreen,
    /// Creates a fullscreen window that will render at desktop resolution.
    ///
    /// The app will use the closest supported size from the given size and scale it to fit the screen.
    SizedFullscreen,
    /// Creates a fullscreen window that uses the maximum supported size.
    Fullscreen,
}

impl Window {
    /// Creates a new [`Window`].
    pub fn new(
        id: WindowId,
        window_descriptor: &WindowDescriptor,
        physical_width: u32,
        physical_height: u32,
        scale_factor: f64,
        position: Option<IVec2>,
        raw_handle: Option<RawHandleWrapper>,
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
            present_mode: window_descriptor.present_mode,
            resizable: window_descriptor.resizable,
            decorations: window_descriptor.decorations,
            cursor_visible: window_descriptor.cursor_visible,
            cursor_grab_mode: window_descriptor.cursor_grab_mode,
            cursor_icon: CursorIcon::Default,
            hittest: true,
            physical_cursor_position: None,
            raw_handle,
            focused: false,
            mode: window_descriptor.mode,
            canvas: window_descriptor.canvas.clone(),
            fit_canvas_to_parent: window_descriptor.fit_canvas_to_parent,
            command_queue: Vec::new(),
            alpha_mode: window_descriptor.alpha_mode,
            always_on_top: window_descriptor.always_on_top,
        }
    }
    /// Get the window's [`WindowId`].
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
    /// creation or the last call to [`set_resolution`](Window::set_resolution).
    ///
    /// This may differ from the actual width depending on OS size limits and
    /// the scaling factor for high DPI monitors.
    #[inline]
    pub fn requested_width(&self) -> f32 {
        self.requested_width
    }

    /// The requested window client area height in logical pixels from window
    /// creation or the last call to [`set_resolution`](Window::set_resolution).
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
    /// Set whether or not the window is maximized.
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

    /// Sets the `position` of the window on the selected `monitor` in physical pixels.
    ///
    /// This automatically un-maximizes the window if it's maximized.
    ///
    /// # Platform-specific
    ///
    /// - iOS: Can only be called on the main thread. Sets the top left coordinates of the window in
    ///   the screen space coordinate system.
    /// - Web: Sets the top-left coordinates relative to the viewport.
    /// - Android / Wayland: Unsupported.
    #[inline]
    pub fn set_position(&mut self, monitor: MonitorSelection, position: IVec2) {
        self.command_queue.push(WindowCommand::SetPosition {
            monitor_selection: monitor,
            position,
        });
    }

    /// Modifies the position of the window to be in the center of the current monitor
    ///
    /// # Platform-specific
    /// - iOS: Can only be called on the main thread.
    /// - Web / Android / Wayland: Unsupported.
    #[inline]
    pub fn center_window(&mut self, monitor_selection: MonitorSelection) {
        self.command_queue
            .push(WindowCommand::Center(monitor_selection));
    }

    /// Modifies the minimum and maximum window bounds for resizing in logical pixels.
    #[inline]
    pub fn set_resize_constraints(&mut self, resize_constraints: WindowResizeConstraints) {
        self.command_queue
            .push(WindowCommand::SetResizeConstraints { resize_constraints });
    }

    /// Request the OS to resize the window such the client area matches the specified
    /// width and height.
    #[allow(clippy::float_cmp)]
    pub fn set_resolution(&mut self, width: f32, height: f32) {
        if self.requested_width == width && self.requested_height == height {
            return;
        }

        self.requested_width = width;
        self.requested_height = height;
        self.command_queue.push(WindowCommand::SetResolution {
            logical_resolution: Vec2::new(self.requested_width, self.requested_height),
            scale_factor: self.scale_factor(),
        });
    }

    /// Override the os-reported scaling factor.
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
            logical_resolution: Vec2::new(self.requested_width, self.requested_height),
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
    ///
    /// This value is unaffected by [`scale_factor_override`](Window::scale_factor_override).
    #[inline]
    pub fn backend_scale_factor(&self) -> f64 {
        self.backend_scale_factor
    }
    /// The scale factor set with [`set_scale_factor_override`](Window::set_scale_factor_override).
    ///
    /// This value may be different from the scale factor reported by the window backend.
    #[inline]
    pub fn scale_factor_override(&self) -> Option<f64> {
        self.scale_factor_override
    }
    /// Get the window's title.
    #[inline]
    pub fn title(&self) -> &str {
        &self.title
    }
    /// Set the window's title.
    pub fn set_title(&mut self, title: String) {
        self.title = title.to_string();
        self.command_queue.push(WindowCommand::SetTitle { title });
    }

    #[inline]
    #[doc(alias = "vsync")]
    /// Get the window's [`PresentMode`].
    pub fn present_mode(&self) -> PresentMode {
        self.present_mode
    }

    #[inline]
    /// Get the window's [`CompositeAlphaMode`].
    pub fn alpha_mode(&self) -> CompositeAlphaMode {
        self.alpha_mode
    }

    #[inline]
    #[doc(alias = "set_vsync")]
    /// Set the window's [`PresentMode`].
    pub fn set_present_mode(&mut self, present_mode: PresentMode) {
        self.present_mode = present_mode;
        self.command_queue
            .push(WindowCommand::SetPresentMode { present_mode });
    }
    /// Get whether or not the window is resizable.
    #[inline]
    pub fn resizable(&self) -> bool {
        self.resizable
    }
    /// Set whether or not the window is resizable.
    pub fn set_resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
        self.command_queue
            .push(WindowCommand::SetResizable { resizable });
    }
    /// Get whether or not decorations are enabled.
    ///
    /// (Decorations are the minimize, maximize, and close buttons on desktop apps)
    ///
    /// ## Platform-specific
    ///
    /// **`iOS`**, **`Android`**, and the **`Web`** do not have decorations.
    #[inline]
    pub fn decorations(&self) -> bool {
        self.decorations
    }
    /// Set whether or not decorations are enabled.
    ///
    /// (Decorations are the minimize, maximize, and close buttons on desktop apps)
    ///
    /// ## Platform-specific
    ///
    /// **`iOS`**, **`Android`**, and the **`Web`** do not have decorations.
    pub fn set_decorations(&mut self, decorations: bool) {
        self.decorations = decorations;
        self.command_queue
            .push(WindowCommand::SetDecorations { decorations });
    }
    /// Get whether or how the cursor is grabbed.
    ///
    /// ## Platform-specific
    ///
    /// - **`Windows`** doesn't support [`CursorGrabMode::Locked`]
    /// - **`macOS`** doesn't support [`CursorGrabMode::Confined`]
    /// - **`iOS/Android`** don't have cursors.
    ///
    /// Since `Windows` and `macOS` have different [`CursorGrabMode`] support, it's possible the value returned here is not the same as the one actually sent to winit.
    #[inline]
    pub fn cursor_grab_mode(&self) -> CursorGrabMode {
        self.cursor_grab_mode
    }
    /// Set whether and how the cursor is grabbed.
    ///
    /// This doesn't hide the cursor. For that, use [`set_cursor_visibility`](Window::set_cursor_visibility)
    ///
    /// ## Platform-specific
    ///
    /// - **`Windows`** doesn't support [`CursorGrabMode::Locked`]
    /// - **`macOS`** doesn't support [`CursorGrabMode::Confined`]
    /// - **`iOS/Android`** don't have cursors.
    ///
    /// Since `Windows` and `macOS` have different [`CursorGrabMode`] support, we first try to set the grab mode that was asked for. If it doesn't work then use the alternate grab mode.
    pub fn set_cursor_grab_mode(&mut self, grab_mode: CursorGrabMode) {
        self.cursor_grab_mode = grab_mode;
        self.command_queue
            .push(WindowCommand::SetCursorGrabMode { grab_mode });
    }
    /// Get whether or not the cursor is visible.
    ///
    /// ## Platform-specific
    ///
    /// - **`Windows`**, **`X11`**, and **`Wayland`**: The cursor is hidden only when inside the window. To stop the cursor from leaving the window, use [`set_cursor_grab_mode`](Window::set_cursor_grab_mode).
    /// - **`macOS`**: The cursor is hidden only when the window is focused.
    /// - **`iOS`** and **`Android`** do not have cursors
    #[inline]
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }
    /// Set whether or not the cursor is visible.
    ///
    /// ## Platform-specific
    ///
    /// - **`Windows`**, **`X11`**, and **`Wayland`**: The cursor is hidden only when inside the window. To stop the cursor from leaving the window, use [`set_cursor_grab_mode`](Window::set_cursor_grab_mode).
    /// - **`macOS`**: The cursor is hidden only when the window is focused.
    /// - **`iOS`** and **`Android`** do not have cursors
    pub fn set_cursor_visibility(&mut self, visible_mode: bool) {
        self.cursor_visible = visible_mode;
        self.command_queue.push(WindowCommand::SetCursorVisibility {
            visible: visible_mode,
        });
    }
    /// Get the current [`CursorIcon`]
    #[inline]
    pub fn cursor_icon(&self) -> CursorIcon {
        self.cursor_icon
    }
    /// Set the [`CursorIcon`]
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
    /// Set the cursor's position
    pub fn set_cursor_position(&mut self, position: Vec2) {
        self.command_queue
            .push(WindowCommand::SetCursorPosition { position });
    }
    /// Modifies whether the window catches cursor events.
    ///
    /// If true, the window will catch the cursor events.
    /// If false, events are passed through the window such that any other window behind it receives them. By default hittest is enabled.
    pub fn set_cursor_hittest(&mut self, hittest: bool) {
        self.hittest = hittest;
        self.command_queue
            .push(WindowCommand::SetCursorHitTest { hittest });
    }
    /// Get whether or not the hittest is active.
    #[inline]
    pub fn hittest(&self) -> bool {
        self.hittest
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
    /// Get the window's [`WindowMode`]
    #[inline]
    pub fn mode(&self) -> WindowMode {
        self.mode
    }
    /// Set the window's [`WindowMode`]
    pub fn set_mode(&mut self, mode: WindowMode) {
        self.mode = mode;
        self.command_queue.push(WindowCommand::SetWindowMode {
            mode,
            resolution: UVec2::new(self.physical_width, self.physical_height),
        });
    }
    /// Get whether or not the window is always on top.
    #[inline]
    pub fn always_on_top(&self) -> bool {
        self.always_on_top
    }

    /// Set whether of not the window is always on top.
    pub fn set_always_on_top(&mut self, always_on_top: bool) {
        self.always_on_top = always_on_top;
        self.command_queue
            .push(WindowCommand::SetAlwaysOnTop { always_on_top });
    }
    /// Close the operating system window corresponding to this [`Window`].
    ///
    /// This will also lead to this [`Window`] being removed from the
    /// [`Windows`] resource.
    ///
    /// If the default [`WindowPlugin`] is used, when no windows are
    /// open, the [app will exit](bevy_app::AppExit).
    /// To disable this behaviour, set `exit_on_all_closed` on the [`WindowPlugin`]
    /// to `false`
    ///
    /// [`Windows`]: crate::Windows
    /// [`WindowPlugin`]: crate::WindowPlugin
    pub fn close(&mut self) {
        self.command_queue.push(WindowCommand::Close);
    }
    #[inline]
    pub fn drain_commands(&mut self) -> impl Iterator<Item = WindowCommand> + '_ {
        self.command_queue.drain(..)
    }
    /// Get whether or not the window has focus.
    ///
    /// A window loses focus when the user switches to another window, and regains focus when the user uses the window again
    #[inline]
    pub fn is_focused(&self) -> bool {
        self.focused
    }
    /// Get the [`RawHandleWrapper`] corresponding to this window if set.
    ///
    /// During normal use, this can be safely unwrapped; the value should only be [`None`] when synthetically constructed for tests.
    pub fn raw_handle(&self) -> Option<RawHandleWrapper> {
        self.raw_handle.as_ref().cloned()
    }

    /// The "html canvas" element selector.
    ///
    /// If set, this selector will be used to find a matching html canvas element,
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

/// Defines where window should be placed at on creation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum WindowPosition {
    /// The position will be set by the window manager.
    Automatic,
    /// Center the window on the monitor.
    ///
    /// The monitor to center the window on can be selected with the `monitor` field in `WindowDescriptor`.
    Centered,
    /// The window's top-left corner will be placed at the specified position in pixels.
    ///
    /// (0,0) represents top-left corner of the monitor.
    ///
    /// The monitor to position the window on can be selected with the `monitor` field in `WindowDescriptor`.
    At(Vec2),
}

/// Defines which monitor to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum MonitorSelection {
    /// Uses current monitor of the window.
    ///
    /// Will fall back to the system default if the window has not yet been created.
    Current,
    /// Uses primary monitor of the system.
    Primary,
    /// Uses monitor with the specified index.
    Index(usize),
}

/// Describes the information needed for creating a window.
///
/// This should be set up before adding the [`WindowPlugin`](crate::WindowPlugin).
/// Most of these settings can also later be configured through the [`Window`](crate::Window) resource.
///
/// See [`examples/window/window_settings.rs`] for usage.
///
/// [`examples/window/window_settings.rs`]: https://github.com/bevyengine/bevy/blob/latest/examples/window/window_settings.rs
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowDescriptor {
    /// The requested logical width of the window's client area.
    ///
    /// May vary from the physical width due to different pixel density on different monitors.
    pub width: f32,
    /// The requested logical height of the window's client area.
    ///
    /// May vary from the physical height due to different pixel density on different monitors.
    pub height: f32,
    /// The position on the screen that the window will be placed at.
    ///
    /// The monitor to place the window on can be selected with the `monitor` field.
    ///
    /// Ignored if `mode` is set to something other than [`WindowMode::Windowed`]
    ///
    /// `WindowPosition::Automatic` will be overridden with `WindowPosition::At(Vec2::ZERO)` if a specific monitor is selected.
    pub position: WindowPosition,
    /// The monitor to place the window on.
    pub monitor: MonitorSelection,
    /// Sets minimum and maximum resize limits.
    pub resize_constraints: WindowResizeConstraints,
    /// Overrides the window's ratio of physical pixels to logical pixels.
    ///
    /// If there are some scaling problems on X11 try to set this option to `Some(1.0)`.
    pub scale_factor_override: Option<f64>,
    /// Sets the title that displays on the window top bar, on the system task bar and other OS specific places.
    ///
    /// ## Platform-specific
    /// - Web: Unsupported.
    pub title: String,
    /// Controls when a frame is presented to the screen.
    #[doc(alias = "vsync")]
    /// The window's [`PresentMode`].
    ///
    /// Used to select whether or not VSync is used
    pub present_mode: PresentMode,
    /// Sets whether the window is resizable.
    ///
    /// ## Platform-specific
    /// - iOS / Android / Web: Unsupported.
    pub resizable: bool,
    /// Sets whether the window should have borders and bars.
    pub decorations: bool,
    /// Sets whether the cursor is visible when the window has focus.
    pub cursor_visible: bool,
    /// Sets whether and how the window grabs the cursor.
    pub cursor_grab_mode: CursorGrabMode,
    /// Sets whether or not the window listens for 'hits' of mouse activity over _this_ window.
    pub hittest: bool,
    /// Sets the [`WindowMode`](crate::WindowMode).
    ///
    /// The monitor to go fullscreen on can be selected with the `monitor` field.
    pub mode: WindowMode,
    /// Sets whether the background of the window should be transparent.
    ///
    /// ## Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - macOS: Not working as expected. See [Bevy #6330](https://github.com/bevyengine/bevy/issues/6330).
    /// - Linux (Wayland): Not working as expected. See [Bevy #5779](https://github.com/bevyengine/bevy/issues/5779).
    pub transparent: bool,
    /// The "html canvas" element selector.
    ///
    /// If set, this selector will be used to find a matching html canvas element,
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
    /// Specifies how the alpha channel of the textures should be handled during compositing.
    pub alpha_mode: CompositeAlphaMode,
    /// Sets the window to always be on top of other windows.
    ///
    /// ## Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - Linux (Wayland): Unsupported.
    pub always_on_top: bool,
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        WindowDescriptor {
            title: "app".to_string(),
            width: 1280.,
            height: 720.,
            position: WindowPosition::Automatic,
            monitor: MonitorSelection::Current,
            resize_constraints: WindowResizeConstraints::default(),
            scale_factor_override: None,
            present_mode: PresentMode::Fifo,
            resizable: true,
            decorations: true,
            cursor_grab_mode: CursorGrabMode::None,
            cursor_visible: true,
            hittest: true,
            mode: WindowMode::Windowed,
            transparent: false,
            canvas: None,
            fit_canvas_to_parent: false,
            alpha_mode: CompositeAlphaMode::Auto,
            always_on_top: false,
        }
    }
}
