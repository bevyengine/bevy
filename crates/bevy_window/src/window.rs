use bevy_ecs::{
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    prelude::{Component, ReflectComponent},
};
use bevy_math::{DVec2, IVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, FromReflect, Reflect};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use bevy_utils::tracing::warn;

use crate::CursorIcon;

/// Marker component for the window considered the primary window.
///
/// Currently this is assumed to only exist on 1 entity at a time.
#[derive(Default, Debug, Component, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Reflect)]
#[reflect(Component)]
pub struct PrimaryWindow;

/// Reference to a window, whether it be a direct link to a specific entity or
/// a more vague defaulting choice.
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum WindowRef {
    /// This will be linked to the primary window that is created by default
    /// in the [`WindowPlugin`](crate::WindowPlugin::primary_window).
    #[default]
    Primary,
    /// A more direct link to a window entity.
    ///
    /// Use this if you want to reference a secondary/tertiary/... window.
    ///
    /// To create a new window you can spawn an entity with a [`Window`],
    /// then you can use that entity here for usage in cameras.
    Entity(Entity),
}

impl WindowRef {
    /// Normalize the window reference so that it can be compared to other window references.
    pub fn normalize(&self, primary_window: Option<Entity>) -> Option<NormalizedWindowRef> {
        let entity = match self {
            Self::Primary => primary_window,
            Self::Entity(entity) => Some(*entity),
        };

        entity.map(NormalizedWindowRef)
    }
}

impl MapEntities for WindowRef {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        match self {
            Self::Entity(entity) => {
                *entity = entity_map.get(*entity)?;
                Ok(())
            }
            Self::Primary => Ok(()),
        }
    }
}

/// A flattened representation of a window reference for equality/hashing purposes.
///
/// For most purposes you probably want to use the unnormalized version [`WindowRef`].
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct NormalizedWindowRef(Entity);

impl NormalizedWindowRef {
    /// Fetch the entity of this window reference
    pub fn entity(&self) -> Entity {
        self.0
    }
}

/// Define how a window will be created and how it will behave.
#[derive(Component, Debug, Clone, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Component, Default)]
pub struct Window {
    /// The cursor of this window.
    pub cursor: Cursor,
    /// What presentation mode to give the window.
    pub present_mode: PresentMode,
    /// Which fullscreen or windowing mode should be used?
    pub mode: WindowMode,
    /// Where the window should be placed.
    pub position: WindowPosition,
    /// What resolution the window should have.
    pub resolution: WindowResolution,
    /// Stores the title of the window.
    pub title: String,
    /// How the alpha channel of textures should be handled while compositing.
    pub composite_alpha_mode: CompositeAlphaMode,
    /// Which size limits to give the window.
    pub resize_constraints: WindowResizeConstraints,
    /// Should the window be resizable?
    ///
    /// Note: This does not stop the program from fullscreening/setting
    /// the size programmatically.
    pub resizable: bool,
    /// Should the window have decorations enabled?
    ///
    /// (Decorations are the minimize, maximize, and close buttons on desktop apps)
    ///
    //  ## Platform-specific
    //
    //  **`iOS`**, **`Android`**, and the **`Web`** do not have decorations.
    pub decorations: bool,
    /// Should the window be transparent?
    ///
    /// Defines whether the background of the window should be transparent.
    ///
    /// ## Platform-specific
    /// - iOS / Android / Web: Unsupported.
    /// - macOS X: Not working as expected.
    /// - Windows 11: Not working as expected
    /// macOS X transparent works with winit out of the box, so this issue might be related to: <https://github.com/gfx-rs/wgpu/issues/687>
    /// Windows 11 is related to <https://github.com/rust-windowing/winit/issues/2082>
    pub transparent: bool,
    /// Should the window start focused?
    pub focused: bool,
    /// Should the window always be on top of other windows?
    ///
    /// ## Platform-specific
    ///
    /// - iOS / Android / Web / Wayland: Unsupported.
    pub window_level: WindowLevel,
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
    /// Whether or not to stop events from propagating out of the canvas element
    ///
    ///  When `true`, this will prevent common browser hotkeys like F5, F12, Ctrl+R, tab, etc.
    /// from performing their default behavior while the bevy app has focus.
    ///
    /// This value has no effect on non-web platforms.
    pub prevent_default_event_handling: bool,
    /// Stores internal state that isn't directly accessible.
    pub internal: InternalWindowState,
    /// Should the window use Input Method Editor?
    ///
    /// If enabled, the window will receive [`Ime`](crate::Ime) events instead of
    /// [`ReceivedCharacter`](crate::ReceivedCharacter) or
    /// [`KeyboardInput`](bevy_input::keyboard::KeyboardInput).
    ///
    /// IME should be enabled during text input, but not when you expect to get the exact key pressed.
    ///
    ///  ## Platform-specific
    ///
    /// - iOS / Android / Web: Unsupported.
    pub ime_enabled: bool,
    /// Sets location of IME candidate box in client area coordinates relative to the top left.
    ///
    ///  ## Platform-specific
    ///
    /// - iOS / Android / Web: Unsupported.
    pub ime_position: Vec2,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            title: "Bevy App".to_owned(),
            cursor: Default::default(),
            present_mode: Default::default(),
            mode: Default::default(),
            position: Default::default(),
            resolution: Default::default(),
            internal: Default::default(),
            composite_alpha_mode: Default::default(),
            resize_constraints: Default::default(),
            ime_enabled: Default::default(),
            ime_position: Default::default(),
            resizable: true,
            decorations: true,
            transparent: false,
            focused: true,
            window_level: Default::default(),
            fit_canvas_to_parent: false,
            prevent_default_event_handling: true,
            canvas: None,
        }
    }
}

impl Window {
    /// Setting this to true will attempt to maximize the window.
    ///
    /// Setting it to false will attempt to un-maximize the window.
    pub fn set_maximized(&mut self, maximized: bool) {
        self.internal.maximize_request = Some(maximized);
    }

    /// Setting this to true will attempt to minimize the window.
    ///
    /// Setting it to false will attempt to un-minimize the window.
    pub fn set_minimized(&mut self, minimized: bool) {
        self.internal.minimize_request = Some(minimized);
    }

    /// The window's client area width in logical pixels.
    #[inline]
    pub fn width(&self) -> f32 {
        self.resolution.width()
    }

    /// The window's client area height in logical pixels.
    #[inline]
    pub fn height(&self) -> f32 {
        self.resolution.height()
    }

    /// The window's client area width in physical pixels.
    #[inline]
    pub fn physical_width(&self) -> u32 {
        self.resolution.physical_width()
    }

    /// The window's client area height in physical pixels.
    #[inline]
    pub fn physical_height(&self) -> u32 {
        self.resolution.physical_height()
    }

    /// The window's scale factor.
    #[inline]
    pub fn scale_factor(&self) -> f64 {
        self.resolution.scale_factor()
    }

    /// The cursor position in this window
    #[inline]
    pub fn cursor_position(&self) -> Option<Vec2> {
        self.cursor
            .physical_position
            .map(|position| (position / self.scale_factor()).as_vec2())
    }

    /// The physical cursor position in this window
    #[inline]
    pub fn physical_cursor_position(&self) -> Option<Vec2> {
        self.cursor
            .physical_position
            .map(|position| position.as_vec2())
    }

    /// Set the cursor position in this window
    pub fn set_cursor_position(&mut self, position: Option<Vec2>) {
        self.cursor.physical_position = position.map(|p| p.as_dvec2() * self.scale_factor());
    }

    /// Set the physical cursor position in this window
    pub fn set_physical_cursor_position(&mut self, position: Option<DVec2>) {
        self.cursor.physical_position = position;
    }
}

/// The size limits on a window.
///
/// These values are measured in logical pixels, so the user's
/// scale factor does affect the size limits on the window.
/// Please note that if the window is resizable, then when the window is
/// maximized it may have a size outside of these limits. The functionality
/// required to disable maximizing is not yet exposed by winit.
#[derive(Debug, Clone, Copy, PartialEq, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Default)]
pub struct WindowResizeConstraints {
    /// The minimum width the window can have.
    pub min_width: f32,
    /// The minimum height the window can have.
    pub min_height: f32,
    /// The maximum width the window can have.
    pub max_width: f32,
    /// The maximum height the window can have.
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
    /// Checks if the constraints are valid.
    ///
    /// Will output warnings if it isn't.
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

/// Stores data about the window's cursor.
#[derive(Debug, Copy, Clone, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, Default)]
pub struct Cursor {
    /// Get the current [`CursorIcon`] while inside the window.
    pub icon: CursorIcon,

    /// Whether the cursor is visible or not.
    ///
    /// ## Platform-specific
    ///
    /// - **`Windows`**, **`X11`**, and **`Wayland`**: The cursor is hidden only when inside the window.
    /// To stop the cursor from leaving the window, change [`Cursor::grab_mode`] to [`CursorGrabMode::Locked`] or [`CursorGrabMode::Confined`]
    /// - **`macOS`**: The cursor is hidden only when the window is focused.
    /// - **`iOS`** and **`Android`** do not have cursors
    pub visible: bool,

    /// Whether or not the cursor is locked.
    ///
    /// ## Platform-specific
    ///
    /// - **`Windows`** doesn't support [`CursorGrabMode::Locked`]
    /// - **`macOS`** doesn't support [`CursorGrabMode::Confined`]
    /// - **`iOS/Android`** don't have cursors.
    ///
    /// Since `Windows` and `macOS` have different [`CursorGrabMode`] support, we first try to set the grab mode that was asked for. If it doesn't work then use the alternate grab mode.
    pub grab_mode: CursorGrabMode,

    /// Set whether or not mouse events within *this* window are captured or fall through to the Window below.
    ///
    /// ## Platform-specific
    ///
    /// - iOS / Android / Web / X11: Unsupported.
    pub hit_test: bool,

    /// The position of this window's cursor.
    physical_position: Option<DVec2>,
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            icon: CursorIcon::Default,
            visible: true,
            grab_mode: CursorGrabMode::None,
            hit_test: true,
            physical_position: None,
        }
    }
}

/// Defines where window should be placed at on creation.
#[derive(Default, Debug, Clone, Copy, PartialEq, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq)]
pub enum WindowPosition {
    /// Position will be set by the window manager
    #[default]
    Automatic,
    /// Window will be centered on the selected monitor
    ///
    /// Note that this does not account for window decorations.
    Centered(MonitorSelection),
    /// The window's top-left corner will be placed at the specified position (in physical pixels)
    ///
    /// (0,0) represents top-left corner of screen space.
    At(IVec2),
}

impl WindowPosition {
    /// Creates a new [`WindowPosition`] at a position.
    pub fn new(position: IVec2) -> Self {
        Self::At(position)
    }

    /// Set the position to a specific point.
    pub fn set(&mut self, position: IVec2) {
        *self = WindowPosition::At(position);
    }

    /// Set the window to a specific monitor.
    pub fn center(&mut self, monitor: MonitorSelection) {
        *self = WindowPosition::Centered(monitor);
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
#[derive(Debug, Clone, PartialEq, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Default)]
pub struct WindowResolution {
    physical_width: u32,
    physical_height: u32,
    scale_factor_override: Option<f64>,
    scale_factor: f64,
}

impl Default for WindowResolution {
    fn default() -> Self {
        WindowResolution {
            physical_width: 1280,
            physical_height: 720,
            scale_factor_override: None,
            scale_factor: 1.0,
        }
    }
}

impl WindowResolution {
    /// Creates a new [`WindowResolution`].
    pub fn new(logical_width: f32, logical_height: f32) -> Self {
        Self {
            physical_width: logical_width as u32,
            physical_height: logical_height as u32,
            ..Default::default()
        }
    }

    /// Builder method for adding a scale factor override to the resolution.
    pub fn with_scale_factor_override(mut self, scale_factor_override: f64) -> Self {
        self.scale_factor_override = Some(scale_factor_override);
        self
    }

    /// The window's client area width in logical pixels.
    #[inline]
    pub fn width(&self) -> f32 {
        (self.physical_width() as f64 / self.scale_factor()) as f32
    }

    /// The window's client area width in logical pixels.
    #[inline]
    pub fn height(&self) -> f32 {
        (self.physical_height() as f64 / self.scale_factor()) as f32
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

    /// The ratio of physical pixels to logical pixels
    ///
    /// `physical_pixels = logical_pixels * scale_factor`
    pub fn scale_factor(&self) -> f64 {
        self.scale_factor_override
            .unwrap_or_else(|| self.base_scale_factor())
    }

    /// The window scale factor as reported by the window backend.
    ///
    /// This value is unaffected by [`WindowResolution::scale_factor_override`].
    #[inline]
    pub fn base_scale_factor(&self) -> f64 {
        self.scale_factor
    }

    /// The scale factor set with [`WindowResolution::set_scale_factor_override`].
    ///
    /// This value may be different from the scale factor reported by the window backend.
    #[inline]
    pub fn scale_factor_override(&self) -> Option<f64> {
        self.scale_factor_override
    }

    /// Set the window's logical resolution.
    #[inline]
    pub fn set(&mut self, width: f32, height: f32) {
        self.set_physical_resolution(
            (width as f64 * self.scale_factor()) as u32,
            (height as f64 * self.scale_factor()) as u32,
        );
    }

    /// Set the window's physical resolution.
    ///
    /// This will ignore the scale factor setting, so most of the time you should
    /// prefer to use [`WindowResolution::set`].
    #[inline]
    pub fn set_physical_resolution(&mut self, width: u32, height: u32) {
        self.physical_width = width;
        self.physical_height = height;
    }

    /// Set the window's scale factor, this may get overridden by the backend.
    #[inline]
    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        let (width, height) = (self.width(), self.height());
        self.scale_factor = scale_factor;
        self.set(width, height);
    }

    /// Set the window's scale factor, this will be used over what the backend decides.
    #[inline]
    pub fn set_scale_factor_override(&mut self, scale_factor_override: Option<f64>) {
        let (width, height) = (self.width(), self.height());
        self.scale_factor_override = scale_factor_override;
        self.set(width, height);
    }
}

impl<I> From<(I, I)> for WindowResolution
where
    I: Into<f32>,
{
    fn from((width, height): (I, I)) -> WindowResolution {
        WindowResolution::new(width.into(), height.into())
    }
}

impl<I> From<[I; 2]> for WindowResolution
where
    I: Into<f32>,
{
    fn from([width, height]: [I; 2]) -> WindowResolution {
        WindowResolution::new(width.into(), height.into())
    }
}

impl From<bevy_math::Vec2> for WindowResolution {
    fn from(res: bevy_math::Vec2) -> WindowResolution {
        WindowResolution::new(res.x, res.y)
    }
}

impl From<bevy_math::DVec2> for WindowResolution {
    fn from(res: bevy_math::DVec2) -> WindowResolution {
        WindowResolution::new(res.x as f32, res.y as f32)
    }
}

/// Defines if and how the cursor is grabbed.
///
/// ## Platform-specific
///
/// - **`Windows`** doesn't support [`CursorGrabMode::Locked`]
/// - **`macOS`** doesn't support [`CursorGrabMode::Confined`]
/// - **`iOS/Android`** don't have cursors.
///
/// Since `Windows` and `macOS` have different [`CursorGrabMode`] support, we first try to set the grab mode that was asked for. If it doesn't work then use the alternate grab mode.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Default)]
pub enum CursorGrabMode {
    /// The cursor can freely leave the window.
    #[default]
    None,
    /// The cursor is confined to the window area.
    Confined,
    /// The cursor is locked inside the window area to a certain position.
    Locked,
}

/// Stores internal state that isn't directly accessible.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Default)]
pub struct InternalWindowState {
    /// If this is true then next frame we will ask to minimize the window.
    minimize_request: Option<bool>,
    /// If this is true then next frame we will ask to maximize/un-maximize the window depending on `maximized`.
    maximize_request: Option<bool>,
}

impl InternalWindowState {
    /// Consumes the current maximize request, if it exists. This should only be called by window backends.
    pub fn take_maximize_request(&mut self) -> Option<bool> {
        self.maximize_request.take()
    }

    /// Consumes the current minimize request, if it exists. This should only be called by window backends.
    pub fn take_minimize_request(&mut self) -> Option<bool> {
        self.minimize_request.take()
    }
}

/// Defines which monitor to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq)]
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
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Hash)]
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
    #[default]
    Fifo = 4, // NOTE: The explicit ordinal values mirror wgpu.
}

/// Specifies how the alpha channel of the textures should be handled during compositing.
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Hash)]
pub enum CompositeAlphaMode {
    /// Chooses either `Opaque` or `Inherit` automatically, depending on the
    /// `alpha_mode` that the current surface can support.
    #[default]
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

/// Defines the way a window is displayed
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq)]
pub enum WindowMode {
    /// Creates a window that uses the given size.
    #[default]
    Windowed,
    /// Creates a borderless window that uses the full size of the screen.
    BorderlessFullscreen,
    /// Creates a fullscreen window that will render at desktop resolution. The app will use the closest supported size
    /// from the given size and scale it to fit the screen.
    SizedFullscreen,
    /// Creates a fullscreen window that uses the maximum supported size.
    Fullscreen,
}

/// A window level groups windows with respect to their z-position.
///
/// The relative ordering between windows in different window levels is fixed.
/// The z-order of a window within the same window level may change dynamically on user interaction.
///
/// ## Platform-specific
///
/// - **iOS / Android / Web / Wayland:** Unsupported.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Reflect, FromReflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq)]
pub enum WindowLevel {
    /// The window will always be below normal windows.
    ///
    /// This is useful for a widget-based app.
    AlwaysOnBottom,
    /// The default.
    #[default]
    Normal,
    /// The window will always be on top of normal windows.
    AlwaysOnTop,
}
