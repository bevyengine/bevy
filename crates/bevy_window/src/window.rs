use bevy_math::{DVec2, IVec2, UVec2, Vec2};
use bevy_utils::{tracing::warn, Uuid};
use raw_window_handle::RawWindowHandle;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// A unique ID for a [`Window`].
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
/// `Immediate` or `Mailbox` will gracefully fallback to `Fifo` when unavailable.
///
/// The presentation mode may be declared in the [`WindowDescriptor`](WindowDescriptor::present_mode)
/// or updated on a [`Window`](Window::set_present_mode).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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
    /// but visible tearing may be observed. Will fallback to `Fifo` if unavailable on the
    /// selected platform and backend. Not optimal for mobile.
    ///
    /// Selecting this variant will panic if not supported, it is preferred to use
    /// [`PresentMode::AutoNoVsync`].
    Immediate = 2,
    /// The presentation engine waits for the next vertical blanking period to update
    /// the current image, but frames may be submitted without delay. This is a low-latency
    /// presentation mode and visible tearing will **not** be observed. Will fallback to `Fifo`
    /// if unavailable on the selected platform and backend. Not optimal for mobile.
    ///
    /// Selecting this variant will panic if not supported, it is preferred to use
    /// [`PresentMode::AutoNoVsync`].
    Mailbox = 3,
    /// The presentation engine waits for the next vertical blanking period to update
    /// the current image. The framerate will be capped at the display refresh rate,
    /// corresponding to the `VSync`. Tearing cannot be observed. Optimal for mobile.
    Fifo = 4, // NOTE: The explicit ordinal values mirror wgpu.
}

impl WindowId {
    /// Creates a new [`WindowId`].
    pub fn new() -> Self {
        WindowId(Uuid::new_v4())
    }
    /// The [`WindowId`] for the primary window.
    pub fn primary() -> Self {
        WindowId(Uuid::from_u128(0))
    }
    /// Get whether or not this [`WindowId`] is for the primary window.
    pub fn is_primary(&self) -> bool {
        *self == WindowId::primary()
    }
}

use crate::CursorIcon;
use std::fmt;

use crate::raw_window_handle::RawWindowHandleWrapper;

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
    cursor_locked: bool,
    physical_cursor_position: Option<DVec2>,
    raw_window_handle: RawWindowHandleWrapper,
    focused: bool,
    mode: WindowMode,
    canvas: Option<String>,
    fit_canvas_to_parent: bool,
    command_queue: Vec<WindowCommand>,
}
/// A command to be sent to a window.
///
/// Bevy apps don't interact with this `enum` directly. Instead, they should use the methods on [`Window`].
/// This `enum` is meant for authors of windowing plugins. See the documentation on [`crate::WindowPlugin`] for more information.
#[derive(Debug)]
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
    SetCursorLockMode {
        locked: bool,
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
    /// Set whether or not the window is maximized.
    SetMaximized {
        maximized: bool,
    },
    /// Set whether or not the window is minimized.
    SetMinimized {
        minimized: bool,
    },
    /// Set the window's position on the screen.
    SetPosition {
        position: IVec2,
    },
    /// Modifies the position of the window to be in the center of the current monitor
    Center(MonitorSelection),
    /// Set the window's [`WindowResizeConstraints`]
    SetResizeConstraints {
        resize_constraints: WindowResizeConstraints,
    },
    Close,
}

/// Defines the way a window is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            present_mode: window_descriptor.present_mode,
            resizable: window_descriptor.resizable,
            decorations: window_descriptor.decorations,
            cursor_visible: window_descriptor.cursor_visible,
            cursor_locked: window_descriptor.cursor_locked,
            cursor_icon: CursorIcon::Default,
            physical_cursor_position: None,
            raw_window_handle: RawWindowHandleWrapper::new(raw_window_handle),
            focused: true,
            mode: window_descriptor.mode,
            canvas: window_descriptor.canvas.clone(),
            fit_canvas_to_parent: window_descriptor.fit_canvas_to_parent,
            command_queue: Vec::new(),
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
            .push(WindowCommand::SetPosition { position });
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
    /// Get whether or not the cursor is locked.
    ///
    /// ## Platform-specific
    ///
    /// - **`macOS`** doesn't support cursor lock, but most windowing plugins can emulate it. See [issue #4875](https://github.com/bevyengine/bevy/issues/4875#issuecomment-1153977546) for more information.
    /// - **`iOS/Android`** don't have cursors.
    #[inline]
    pub fn cursor_locked(&self) -> bool {
        self.cursor_locked
    }
    /// Set whether or not the cursor is locked.
    ///
    /// This doesn't hide the cursor. For that, use [`set_cursor_visibility`](Window::set_cursor_visibility)
    ///
    /// ## Platform-specific
    ///
    /// - **`macOS`** doesn't support cursor lock, but most windowing plugins can emulate it. See [issue #4875](https://github.com/bevyengine/bevy/issues/4875#issuecomment-1153977546) for more information.
    /// - **`iOS/Android`** don't have cursors.
    pub fn set_cursor_lock_mode(&mut self, lock_mode: bool) {
        self.cursor_locked = lock_mode;
        self.command_queue
            .push(WindowCommand::SetCursorLockMode { locked: lock_mode });
    }
    /// Get whether or not the cursor is visible.
    ///
    /// ## Platform-specific
    ///
    /// - **`Windows`**, **`X11`**, and **`Wayland`**: The cursor is hidden only when inside the window. To stop the cursor from leaving the window, use [`set_cursor_lock_mode`](Window::set_cursor_lock_mode).
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
    /// - **`Windows`**, **`X11`**, and **`Wayland`**: The cursor is hidden only when inside the window. To stop the cursor from leaving the window, use [`set_cursor_lock_mode`](Window::set_cursor_lock_mode).
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
    /// Get the [`RawWindowHandleWrapper`] corresponding to this window
    pub fn raw_window_handle(&self) -> RawWindowHandleWrapper {
        self.raw_window_handle.clone()
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
#[derive(Debug, Clone, Copy)]
pub enum WindowPosition {
    /// Position will be set by the window manager
    Automatic,
    /// Window will be centered on the selected monitor
    ///
    /// Note that this does not account for window decorations.
    Centered(MonitorSelection),
    /// The window's top-left corner will be placed at the specified position (in pixels)
    ///
    /// (0,0) represents top-left corner of screen space.
    At(Vec2),
}

/// Defines which monitor to use.
#[derive(Debug, Clone, Copy)]
pub enum MonitorSelection {
    /// Uses current monitor of the window.
    Current,
    /// Uses primary monitor of the system.
    Primary,
    /// Uses monitor with the specified index.
    Number(usize),
}

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
    ///
    /// May vary from the physical width due to different pixel density on different monitors.
    pub width: f32,
    /// The requested logical height of the window's client area.
    ///
    /// May vary from the physical height due to different pixel density on different monitors.
    pub height: f32,
    /// The position on the screen that the window will be placed at.
    pub position: WindowPosition,
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
    /// Sets whether the window locks the cursor inside its borders when the window has focus.
    pub cursor_locked: bool,
    /// Sets the [`WindowMode`](crate::WindowMode).
    pub mode: WindowMode,
    /// Sets whether the background of the window should be transparent.
    ///
    /// ## Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - macOS X: Not working as expected.
    /// - Windows 11: Not working as expected
    /// macOS X transparent works with winit out of the box, so this issue might be related to: <https://github.com/gfx-rs/wgpu/issues/687>
    /// Windows 11 is related to <https://github.com/rust-windowing/winit/issues/2082>
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
}

impl Default for WindowDescriptor {
    fn default() -> Self {
        WindowDescriptor {
            title: "app".to_string(),
            width: 1280.,
            height: 720.,
            position: WindowPosition::Automatic,
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
