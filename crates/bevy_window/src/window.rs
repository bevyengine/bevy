use bevy_ecs::{
    entity::{Entity, EntityMapper, MapEntities},
    prelude::{Component, ReflectComponent},
};
use bevy_math::{DVec2, IVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use bevy_utils::tracing::warn;

use crate::CursorIcon;

/// Marker [`Component`] for the window considered the primary window.
///
/// Currently this is assumed to only exist on 1 entity at a time.
///
/// [`WindowPlugin`](crate::WindowPlugin) will spawn a window entity
/// with this component if `primary_window` is `Some`.
#[derive(Default, Debug, Component, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Reflect)]
#[reflect(Component)]
pub struct PrimaryWindow;

/// Reference to a [`Window`], whether it be a direct link to a specific entity or
/// a more vague defaulting choice.
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, Reflect)]
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
    fn map_entities(&mut self, entity_mapper: &mut EntityMapper) {
        match self {
            Self::Entity(entity) => {
                *entity = entity_mapper.get_or_reserve(*entity);
            }
            Self::Primary => {}
        };
    }
}

/// A flattened representation of a window reference for equality/hashing purposes.
///
/// For most purposes you probably want to use the unnormalized version [`WindowRef`].
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
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

/// The defining [`Component`] for window entities,
/// storing information about how it should appear and behave.
///
/// Each window corresponds to an entity, and is uniquely identified by the value of their [`Entity`].
/// When the [`Window`] component is added to an entity, a new window will be opened.
/// When it is removed or the entity is despawned, the window will close.
///
/// This component is synchronized with `winit` through `bevy_winit`:
/// it will reflect the current state of the window and can be modified to change this state.
#[derive(Component, Debug, Clone, Reflect)]
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
    /// Which fullscreen or windowing mode should be used.
    pub mode: WindowMode,
    /// Where the window should be placed.
    pub position: WindowPosition,
    /// What resolution the window should have.
    pub resolution: WindowResolution,
    /// Stores the title of the window.
    pub title: String,
    /// How the alpha channel of textures should be handled while compositing.
    pub composite_alpha_mode: CompositeAlphaMode,
    /// The limits of the window's logical size
    /// (found in its [`resolution`](WindowResolution)) when resizing.
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
    ///
    /// macOS X transparent works with winit out of the box, so this issue might be related to: <https://github.com/gfx-rs/wgpu/issues/687>.
    /// You should also set the window `composite_alpha_mode` to `CompositeAlphaMode::PostMultiplied`.
    pub transparent: bool,
    /// Get/set whether the window is focused.
    pub focused: bool,
    /// Where should the window appear relative to other overlapping window.
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
    /// Sets a specific theme for the window.
    ///
    /// If `None` is provided, the window will use the system theme.
    ///
    /// ## Platform-specific
    ///
    /// - iOS / Android / Web: Unsupported.
    pub window_theme: Option<WindowTheme>,
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
            window_theme: None,
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
    ///
    /// See [`WindowResolution`] for an explanation about logical/physical sizes.
    #[inline]
    pub fn width(&self) -> f32 {
        self.resolution.width()
    }

    /// The window's client area height in logical pixels.
    ///
    /// See [`WindowResolution`] for an explanation about logical/physical sizes.
    #[inline]
    pub fn height(&self) -> f32 {
        self.resolution.height()
    }

    /// The window's client area width in physical pixels.
    ///
    /// See [`WindowResolution`] for an explanation about logical/physical sizes.
    #[inline]
    pub fn physical_width(&self) -> u32 {
        self.resolution.physical_width()
    }

    /// The window's client area height in physical pixels.
    ///
    /// See [`WindowResolution`] for an explanation about logical/physical sizes.
    #[inline]
    pub fn physical_height(&self) -> u32 {
        self.resolution.physical_height()
    }

    /// The window's scale factor.
    ///
    /// Ratio of physical size to logical size, see [`WindowResolution`].
    #[inline]
    pub fn scale_factor(&self) -> f64 {
        self.resolution.scale_factor()
    }

    /// The cursor position in this window in logical pixels.
    ///
    /// Returns `None` if the cursor is outside the window area.
    ///
    /// See [`WindowResolution`] for an explanation about logical/physical sizes.
    #[inline]
    pub fn cursor_position(&self) -> Option<Vec2> {
        self.internal
            .physical_cursor_position
            .map(|position| (position / self.scale_factor()).as_vec2())
    }

    /// The cursor position in this window in physical pixels.
    ///
    /// Returns `None` if the cursor is outside the window area.
    ///
    /// See [`WindowResolution`] for an explanation about logical/physical sizes.
    #[inline]
    pub fn physical_cursor_position(&self) -> Option<Vec2> {
        self.internal
            .physical_cursor_position
            .map(|position| position.as_vec2())
    }

    /// Set the cursor position in this window in logical pixels.
    ///
    /// See [`WindowResolution`] for an explanation about logical/physical sizes.
    pub fn set_cursor_position(&mut self, position: Option<Vec2>) {
        self.internal.physical_cursor_position =
            position.map(|p| p.as_dvec2() * self.scale_factor());
    }

    /// Set the cursor position in this window in physical pixels.
    ///
    /// See [`WindowResolution`] for an explanation about logical/physical sizes.
    pub fn set_physical_cursor_position(&mut self, position: Option<DVec2>) {
        self.internal.physical_cursor_position = position;
    }
}

/// The size limits on a [`Window`].
///
/// These values are measured in logical pixels (see [`WindowResolution`]), so the user's
/// scale factor does affect the size limits on the window.
///
/// Please note that if the window is resizable, then when the window is
/// maximized it may have a size outside of these limits. The functionality
/// required to disable maximizing is not yet exposed by winit.
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
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

/// Cursor data for a [`Window`].
#[derive(Debug, Copy, Clone, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, Default)]
pub struct Cursor {
    /// What the cursor should look like while inside the window.
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

    /// Whether or not the cursor is locked by or confined within the window.
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
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor {
            icon: CursorIcon::Default,
            visible: true,
            grab_mode: CursorGrabMode::None,
            hit_test: true,
        }
    }
}

/// Defines where a [`Window`] should be placed on the screen.
#[derive(Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq)]
pub enum WindowPosition {
    /// Position will be set by the window manager.
    /// Bevy will delegate this decision to the window manager and no guarantees can be made about where the window will be placed.
    ///
    /// Used at creation but will be changed to [`At`](WindowPosition::At).
    #[default]
    Automatic,
    /// Window will be centered on the selected monitor.
    ///
    /// Note that this does not account for window decorations.
    ///
    /// Used at creation or for update but will be changed to [`At`](WindowPosition::At)
    Centered(MonitorSelection),
    /// The window's top-left corner should be placed at the specified position (in physical pixels).
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

/// Controls the size of a [`Window`]
///
/// ## Physical, logical and requested sizes
///
/// There are three sizes associated with a window:
/// - the physical size,
/// which represents the actual height and width in physical pixels
/// the window occupies on the monitor,
/// - the logical size,
/// which represents the size that should be used to scale elements
/// inside the window, measured in logical pixels,
/// - the requested size,
/// measured in logical pixels, which is the value submitted
/// to the API when creating the window, or requesting that it be resized.
///
/// ## Scale factor
///
/// The reason logical size and physical size are separated and can be different
/// is to account for the cases where:
/// - several monitors have different pixel densities,
/// - the user has set up a pixel density preference in its operating system,
/// - the Bevy `App` has specified a specific scale factor between both.
///
/// The factor between physical size and logical size can be retrieved with
/// [`WindowResolution::scale_factor`].
///
/// For the first two cases, a scale factor is set automatically by the operating
/// system through the window backend. You can get it with
/// [`WindowResolution::base_scale_factor`].
///
/// For the third case, you can override this automatic scale factor with
/// [`WindowResolution::set_scale_factor_override`].
///
/// ## Requested and obtained sizes
///
/// The logical size should be equal to the requested size after creating/resizing,
/// when possible.
/// The reason the requested size and logical size might be different
/// is because the corresponding physical size might exceed limits (either the
/// size limits of the monitor, or limits defined in [`WindowResizeConstraints`]).
///
/// Note: The requested size is not kept in memory, for example requesting a size
/// too big for the screen, making the logical size different from the requested size,
/// and then setting a scale factor that makes the previous requested size within
/// the limits of the screen will not get back that previous requested size.

#[derive(Debug, Clone, PartialEq, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Default)]
pub struct WindowResolution {
    /// Width of the window in physical pixels.
    physical_width: u32,
    /// Height of the window in physical pixels.
    physical_height: u32,
    /// Code-provided ratio of physical size to logical size.
    ///
    /// Should be used instead `scale_factor` when set.
    scale_factor_override: Option<f64>,
    /// OS-provided ratio of physical size to logical size.
    ///
    /// Set automatically depending on the pixel density of the screen.
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

    /// The window's client area height in logical pixels.
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

    /// The ratio of physical pixels to logical pixels.
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
    ///
    /// This can change the logical and physical sizes if the resulting physical
    /// size is not within the limits.
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

/// Defines if and how the [`Cursor`] is grabbed by a [`Window`].
///
/// ## Platform-specific
///
/// - **`Windows`** doesn't support [`CursorGrabMode::Locked`]
/// - **`macOS`** doesn't support [`CursorGrabMode::Confined`]
/// - **`iOS/Android`** don't have cursors.
///
/// Since `Windows` and `macOS` have different [`CursorGrabMode`] support, we first try to set the grab mode that was asked for. If it doesn't work then use the alternate grab mode.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
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

/// Stores internal [`Window`] state that isn't directly accessible.
#[derive(Default, Debug, Copy, Clone, PartialEq, Reflect)]
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
    /// Unscaled cursor position.
    physical_cursor_position: Option<DVec2>,
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

/// References a screen monitor.
///
/// Used when centering a [`Window`] on a monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq)]
pub enum MonitorSelection {
    /// Uses the current monitor of the window.
    ///
    /// If [`WindowPosition::Centered(MonitorSelection::Current)`](WindowPosition::Centered) is used when creating a window,
    /// the window doesn't have a monitor yet, this will fall back to [`WindowPosition::Automatic`].
    Current,
    /// Uses the primary monitor of the system.
    Primary,
    /// Uses the monitor with the specified index.
    Index(usize),
}

/// Presentation mode for a [`Window`].
///
/// The presentation mode specifies when a frame is presented to the window. The [`Fifo`]
/// option corresponds to a traditional `VSync`, where the framerate is capped by the
/// display refresh rate. Both [`Immediate`] and [`Mailbox`] are low-latency and are not
/// capped by the refresh rate, but may not be available on all platforms. Tearing
/// may be observed with [`Immediate`] mode, but will not be observed with [`Mailbox`] or
/// [`Fifo`].
///
/// [`AutoVsync`] or [`AutoNoVsync`] will gracefully fallback to [`Fifo`] when unavailable.
///
/// [`Immediate`] or [`Mailbox`] will panic if not supported by the platform.
///
/// [`Fifo`]: PresentMode::Fifo
/// [`Immediate`]: PresentMode::Immediate
/// [`Mailbox`]: PresentMode::Mailbox
/// [`AutoVsync`]: PresentMode::AutoVsync
/// [`AutoNoVsync`]: PresentMode::AutoNoVsync
///
#[repr(C)]
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq, Hash, Reflect)]
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

/// Specifies how the alpha channel of the textures should be handled during compositing, for a [`Window`].
#[repr(C)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Hash)]
pub enum CompositeAlphaMode {
    /// Chooses either [`Opaque`](CompositeAlphaMode::Opaque) or [`Inherit`](CompositeAlphaMode::Inherit)
    /// automatically, depending on the `alpha_mode` that the current surface can support.
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

/// Defines the way a [`Window`] is displayed.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq)]
pub enum WindowMode {
    /// The window should take a portion of the screen, using the window resolution size.
    #[default]
    Windowed,
    /// The window should appear fullscreen by being borderless and using the full
    /// size of the screen.
    ///
    /// When setting this, the window's physical size will be modified to match the size
    /// of the current monitor resolution, and the logical size will follow based
    /// on the scale factor, see [`WindowResolution`].
    BorderlessFullscreen,
    /// The window should be in "true"/"legacy" Fullscreen mode.
    ///
    /// When setting this, the operating system will be requested to use the
    /// **closest** resolution available for the current monitor to match as
    /// closely as possible the window's physical size.
    /// After that, the window's physical size will be modified to match
    /// that monitor resolution, and the logical size will follow based on the
    /// scale factor, see [`WindowResolution`].
    SizedFullscreen,
    /// The window should be in "true"/"legacy" Fullscreen mode.
    ///
    /// When setting this, the operating system will be requested to use the
    /// **biggest** resolution available for the current monitor.
    /// After that, the window's physical size will be modified to match
    /// that monitor resolution, and the logical size will follow based on the
    /// scale factor, see [`WindowResolution`].
    Fullscreen,
}

/// Specifies where a [`Window`] should appear relative to other overlapping windows (on top or under) .
///
/// Levels are groups of windows with respect to their z-position.
///
/// The relative ordering between windows in different window levels is fixed.
/// The z-order of windows within the same window level may change dynamically on user interaction.
///
/// ## Platform-specific
///
/// - **iOS / Android / Web / Wayland:** Unsupported.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq)]
pub enum WindowLevel {
    /// The window will always be below [`WindowLevel::Normal`] and [`WindowLevel::AlwaysOnTop`] windows.
    ///
    /// This is useful for a widget-based app.
    AlwaysOnBottom,
    /// The default group.
    #[default]
    Normal,
    /// The window will always be on top of [`WindowLevel::Normal`] and [`WindowLevel::AlwaysOnBottom`] windows.
    AlwaysOnTop,
}

/// The [`Window`] theme variant to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq)]
pub enum WindowTheme {
    /// Use the light variant.
    Light,

    /// Use the dark variant.
    Dark,
}
