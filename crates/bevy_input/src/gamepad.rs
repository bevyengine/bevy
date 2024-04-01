//! The gamepad input functionality.

use crate::{Axis, ButtonInput, ButtonState};
use bevy_ecs::{
    bundle::Bundle,
    change_detection::DetectChangesMut,
    component::Component,
    entity::{Entity, EntityHashMap},
    event::{Event, EventReader, EventWriter},
    system::{Commands, Local, Query, Res, ResMut, Resource},
};
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use bevy_utils::{
    tracing::{info, warn},
    Duration, HashMap,
};
use std::fmt::{Display, Formatter};
use thiserror::Error;

/// A gamepad event.
///
/// This event type is used over the [`GamepadConnectionEvent`],
/// [`RawGamepadButtonChangedEvent`] and [`RawGamepadAxisChangedEvent`] when
/// the in-frame relative ordering of events is important.
///
/// This event type is not used by `bevy_input`.
#[derive(Event, Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum RawGamepadEvent {
    /// A gamepad has been connected or disconnected.
    Connection(GamepadConnectionEvent),
    /// A button of the gamepad has been triggered.
    Button(RawGamepadButtonChangedEvent),
    /// An axis of the gamepad has been triggered.
    Axis(RawGamepadAxisChangedEvent),
}

impl From<GamepadConnectionEvent> for RawGamepadEvent {
    fn from(value: GamepadConnectionEvent) -> Self {
        Self::Connection(value)
    }
}

impl From<RawGamepadButtonChangedEvent> for RawGamepadEvent {
    fn from(value: RawGamepadButtonChangedEvent) -> Self {
        Self::Button(value)
    }
}

impl From<RawGamepadAxisChangedEvent> for RawGamepadEvent {
    fn from(value: RawGamepadAxisChangedEvent) -> Self {
        Self::Axis(value)
    }
}

/// Gamepad event for when the "value" (amount of pressure) on the button
/// changes by an amount larger than the threshold defined in [`GamepadSettings`].
#[derive(Event, Debug, Copy, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct RawGamepadButtonChangedEvent {
    /// The gamepad on which the button is triggered.
    pub gamepad: GamepadId,
    /// The type of the triggered button.
    pub button_type: GamepadButtonType,
    /// The value of the button.
    pub value: f32,
}

impl RawGamepadButtonChangedEvent {
    /// Creates a [`RawGamepadButtonChangedEvent`].
    pub fn new(gamepad: GamepadId, button_type: GamepadButtonType, value: f32) -> Self {
        Self {
            gamepad,
            button_type,
            value,
        }
    }
}

/// Gamepad event for when the "value" on the axis changes
/// by an amount larger than the threshold defined in [`GamepadSettings`].
#[derive(Event, Debug, Copy, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct RawGamepadAxisChangedEvent {
    /// The gamepad on which the axis is triggered.
    pub gamepad: GamepadId,
    /// The type of the triggered axis.
    pub axis_type: GamepadAxisType,
    /// The value of the axis.
    pub value: f32,
}

impl RawGamepadAxisChangedEvent {
    /// Creates a [`RawGamepadAxisChangedEvent`].
    pub fn new(gamepad: GamepadId, axis_type: GamepadAxisType, value: f32) -> Self {
        Self {
            gamepad,
            axis_type,
            value,
        }
    }
}

/// A Gamepad connection event. Created when a connection to a gamepad
/// is established and when a gamepad is disconnected.
#[derive(Event, Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadConnectionEvent {
    /// The gamepad whose connection status changed.
    pub gamepad: GamepadId,
    /// The change in the gamepads connection.
    pub connection: GamepadConnection,
}

impl GamepadConnectionEvent {
    /// Creates a [`GamepadConnectionEvent`].
    pub fn new(gamepad: GamepadId, connection: GamepadConnection) -> Self {
        Self {
            gamepad,
            connection,
        }
    }

    /// Is the gamepad connected?
    pub fn connected(&self) -> bool {
        matches!(self.connection, GamepadConnection::Connected(_))
    }

    /// Is the gamepad disconnected?
    pub fn disconnected(&self) -> bool {
        !self.connected()
    }
}

/// Gamepad button digital state changed event.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadButtonStateChanged {
    /// The entity that represents this gamepad.
    pub entity: Entity,
    /// The gamepad id of this gamepad.
    pub gamepad_id: GamepadId,
    /// The gamepad button assigned to the event.
    pub button: GamepadButtonType,
    /// The pressed state of the button.
    pub state: ButtonState,
}

impl GamepadButtonStateChanged {
    /// Creates a new [`GamepadButtonStateChanged`]
    pub fn new(
        entity: Entity,
        gamepad_id: impl AsRef<GamepadId>,
        button: GamepadButtonType,
        state: ButtonState,
    ) -> Self {
        Self {
            entity,
            gamepad_id: *gamepad_id.as_ref(),
            button,
            state,
        }
    }
}

/// Gamepad analog state changed event
#[derive(Event, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadButtonChanged {
    /// The entity that represents this gamepad.
    pub entity: Entity,
    /// The gamepad id of this gamepad.
    pub gamepad_id: GamepadId,
    /// The gamepad button assigned to the event.
    pub button: GamepadButtonType,
    /// The pressed state of the button.
    pub state: ButtonState,
    /// The analog value of the button.
    pub value: f32,
}

impl GamepadButtonChanged {
    /// Creates a new [`GamepadButtonChanged`]
    pub fn new(
        entity: Entity,
        gamepad_id: impl AsRef<GamepadId>,
        button: GamepadButtonType,
        state: ButtonState,
        value: f32,
    ) -> Self {
        Self {
            entity,
            gamepad_id: *gamepad_id.as_ref(),
            button,
            state,
            value,
        }
    }
}

/// A gamepad axis event.
#[derive(Event, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadAxisChanged {
    /// The entity that represents this gamepad.
    pub entity: Entity,
    /// The gamepad id of this gamepad.
    pub gamepad_id: GamepadId,
    /// The gamepad axis assigned to the event.
    pub axis: GamepadAxisType,
    /// The value of this axis.
    pub value: f32,
}

impl GamepadAxisChanged {
    /// Creates a new [`GamepadAxisChanged`]
    pub fn new(
        entity: Entity,
        gamepad_id: impl AsRef<GamepadId>,
        axis: GamepadAxisType,
        value: f32,
    ) -> Self {
        Self {
            entity,
            gamepad_id: *gamepad_id.as_ref(),
            axis,
            value,
        }
    }
}

/// Errors that occur when setting axis settings for gamepad input.
#[derive(Error, Debug, PartialEq)]
pub enum AxisSettingsError {
    /// The given parameter `livezone_lowerbound` was not in range -1.0..=0.0.
    #[error("invalid livezone_lowerbound {0}, expected value [-1.0..=0.0]")]
    LiveZoneLowerBoundOutOfRange(f32),
    /// The given parameter `deadzone_lowerbound` was not in range -1.0..=0.0.
    #[error("invalid deadzone_lowerbound {0}, expected value [-1.0..=0.0]")]
    DeadZoneLowerBoundOutOfRange(f32),
    /// The given parameter `deadzone_lowerbound` was not in range -1.0..=0.0.
    #[error("invalid deadzone_upperbound {0}, expected value [0.0..=1.0]")]
    DeadZoneUpperBoundOutOfRange(f32),
    /// The given parameter `deadzone_lowerbound` was not in range -1.0..=0.0.
    #[error("invalid livezone_upperbound {0}, expected value [0.0..=1.0]")]
    LiveZoneUpperBoundOutOfRange(f32),
    /// Parameter `livezone_lowerbound` was not less than or equal to parameter `deadzone_lowerbound`.
    #[error("invalid parameter values livezone_lowerbound {} deadzone_lowerbound {}, expected livezone_lowerbound <= deadzone_lowerbound", .livezone_lowerbound, .deadzone_lowerbound)]
    LiveZoneLowerBoundGreaterThanDeadZoneLowerBound {
        /// The value of the `livezone_lowerbound` parameter.
        livezone_lowerbound: f32,
        /// The value of the `deadzone_lowerbound` parameter.
        deadzone_lowerbound: f32,
    },
    ///  Parameter `deadzone_upperbound` was not less than or equal to parameter `livezone_upperbound`.
    #[error("invalid parameter values livezone_upperbound {} deadzone_upperbound {}, expected deadzone_upperbound <= livezone_upperbound", .livezone_upperbound, .deadzone_upperbound)]
    DeadZoneUpperBoundGreaterThanLiveZoneUpperBound {
        /// The value of the `livezone_upperbound` parameter.
        livezone_upperbound: f32,
        /// The value of the `deadzone_upperbound` parameter.
        deadzone_upperbound: f32,
    },
    /// The given parameter was not in range 0.0..=2.0.
    #[error("invalid threshold {0}, expected 0.0 <= threshold <= 2.0")]
    Threshold(f32),
}

/// Errors that occur when setting button settings for gamepad input.
#[derive(Error, Debug, PartialEq)]
pub enum ButtonSettingsError {
    /// The given parameter was not in range 0.0..=1.0.
    #[error("invalid release_threshold {0}, expected value [0.0..=1.0]")]
    ReleaseThresholdOutOfRange(f32),
    /// The given parameter was not in range 0.0..=1.0.
    #[error("invalid press_threshold {0}, expected [0.0..=1.0]")]
    PressThresholdOutOfRange(f32),
    /// Parameter `release_threshold` was not less than or equal to `press_threshold`.
    #[error("invalid parameter values release_threshold {} press_threshold {}, expected release_threshold <= press_threshold", .release_threshold, .press_threshold)]
    ReleaseThresholdGreaterThanPressThreshold {
        /// The value of the `press_threshold` parameter.
        press_threshold: f32,
        /// The value of the `release_threshold` parameter.
        release_threshold: f32,
    },
}

/// Gamepad [`bundle`](Bundle) with the minimum components required to represent a gamepad.
#[derive(Bundle, Debug)]
pub struct MinimalGamepad {
    /// The [`Gamepad`] component
    pub gamepad: Gamepad,
    /// The [`GamepadSettings`] component
    pub settings: GamepadSettings,
    /// The [`GamepadButtons`] component
    pub buttons: GamepadButtons,
    /// The [`GamepadAxes`] component
    pub axis: GamepadAxes,
}

impl MinimalGamepad {
    /// Creates a new minimal gamepad
    pub fn new(gamepad: Gamepad, settings: GamepadSettings) -> Self {
        Self {
            gamepad,
            settings,
            buttons: GamepadButtons::default(),
            axis: GamepadAxes::default(),
        }
    }
}

/// A gamepad with an associated `ID`.
///
/// ## Usage
///
/// It is the primary identifier for raw events. You can access the individual [`entity`](Entity)
/// belonging to a [`GamepadId`] through the [`Gamepads`] [`resource`](Res) or a [`query`](Query) with [`Gamepad`].
///
/// ## Note
///
/// The `ID` of a gamepad is fixed until the app is restarted.
/// Reconnected gamepads will try to preserve their `ID` but it's not guaranteed.
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Hash, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadId(pub usize);

impl AsRef<GamepadId> for GamepadId {
    fn as_ref(&self) -> &GamepadId {
        self
    }
}

impl Display for GamepadId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The [`Gamepad`] [`component`](Component) stores a connected gamepad's metadata such as the [`GamepadId`] identifier or `name`.
///
/// The [`entity`](Entity) representing a gamepad and its [`minimal components`](MinimalGamepad) are automatically managed.
///
/// # Usage
///
/// The only way to obtain a [`Gamepad`] is by [`query`](Query).
///
/// # Examples
///
/// ```
/// # use bevy_input::gamepad::{Gamepad};
/// # use bevy_ecs::system::Query;
/// #
/// fn gamepad_name_system(gamepads: Query<&Gamepad>) {
///     for gamepad in gamepads.iter() {
///         println!("{}", gamepad.id())
///     }
/// }
/// ```
#[derive(Component, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct Gamepad {
    id: GamepadId,
    info: GamepadInfo,
}

impl AsRef<GamepadId> for Gamepad {
    fn as_ref(&self) -> &GamepadId {
        &self.id
    }
}

impl Gamepad {
    /// Returns the [`GamepadId`] of the gamepad.
    pub fn id(&self) -> GamepadId {
        self.id
    }

    /// The name of the gamepad.
    ///
    /// This name is generally defined by the OS.
    ///
    /// For example on Windows the name may be "HID-compliant game controller".
    pub fn name(&self) -> &str {
        self.info.name.as_str()
    }
}

/// Metadata associated with a [`GamepadId`].
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadInfo {
    /// The name of the gamepad.
    ///
    /// This name is generally defined by the OS.
    ///
    /// For example on Windows the name may be "HID-compliant game controller".
    pub name: String,
}

/// A [`resource`](Resource) with the mapping of connected [`GamepadId`] and their [`Entity`].
#[derive(Debug, Default, Resource, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct Gamepads {
    /// Mapping of [`Entity`] to [`GamepadId`].
    pub(crate) entity_to_id: EntityHashMap<GamepadId>,
    /// Mapping of [`GamepadId`] to [`Entity`].
    pub(crate) id_to_entity: HashMap<GamepadId, Entity>,
}

impl Gamepads {
    /// Returns the [`Entity`] assigned to a connected [`GamepadId`].
    pub fn get_entity(&self, gamepad_id: impl AsRef<GamepadId>) -> Option<Entity> {
        self.id_to_entity.get(gamepad_id.as_ref()).copied()
    }

    /// Returns the [`GamepadId`] assigned to a gamepad [`Entity`].
    pub fn get_gamepad_id(&self, entity: Entity) -> Option<GamepadId> {
        self.entity_to_id.get(&entity).copied()
    }
}

/// A type of gamepad button.
///
/// ## Usage
///
/// This is used to determine which button has changed its value when receiving gamepad button events
/// It is also used in the [`GamepadButtons`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Reflect, PartialOrd, Ord)]
#[reflect(Debug, Hash, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum GamepadButtonType {
    /// The bottom action button of the action pad (i.e. PS: Cross, Xbox: A).
    South,
    /// The right action button of the action pad (i.e. PS: Circle, Xbox: B).
    East,
    /// The upper action button of the action pad (i.e. PS: Triangle, Xbox: Y).
    North,
    /// The left action button of the action pad (i.e. PS: Square, Xbox: X).
    West,

    /// The C button.
    C,
    /// The Z button.
    Z,

    /// The first left trigger.
    LeftTrigger,
    /// The second left trigger.
    LeftTrigger2,
    /// The first right trigger.
    RightTrigger,
    /// The second right trigger.
    RightTrigger2,
    /// The select button.
    Select,
    /// The start button.
    Start,
    /// The mode button.
    Mode,

    /// The left thumb stick button.
    LeftThumb,
    /// The right thumb stick button.
    RightThumb,

    /// The up button of the D-Pad.
    DPadUp,
    /// The down button of the D-Pad.
    DPadDown,
    /// The left button of the D-Pad.
    DPadLeft,
    /// The right button of the D-Pad.
    DPadRight,

    /// Miscellaneous buttons, considered non-standard (i.e. Extra buttons on a flight stick that do not have a gamepad equivalent).
    Other(u8),
}

/// The [`GamepadButtons`] [`component`](Component) is a collection of [`GamepadButtonType`] and their state during the current frame.
///
/// The [`entity`](Entity) representing a gamepad and its [`minimal components`](MinimalGamepad) are automatically managed.
///
/// # Usage
///
/// The only way to obtain a [`GamepadButtons`] is by [`query`](Query).
///
/// # Examples
///
/// ```
/// # use bevy_input::gamepad::{Gamepad, GamepadButtons, GamepadButtonType};
/// # use bevy_ecs::system::Query;
/// #
/// fn gamepad_button_input_system(gamepads: Query<(&Gamepad, &GamepadButtons)>) {
///     for (gamepad, buttons) in gamepads.iter() {
///         if buttons.just_pressed(GamepadButtonType::North) {
///             println!("{} just pressed North", gamepad.id())
///         }
///     }
/// }
/// ```
#[derive(Component, Debug, Default)]
pub struct GamepadButtons {
    // TODO: Change digital to 2 fixedbitsets?
    /// [`ButtonInput`] of [`GamepadButtonType`] representing their digital state
    pub(crate) digital: ButtonInput<GamepadButtonType>,
    /// [`Axis`] of [`GamepadButtonType`] representing their analog state.
    pub(crate) analog: Axis<GamepadButtonType>,
}

impl GamepadButtons {
    /// Returns the position data of the provided [`GamepadButtonType`].
    ///
    /// This will be clamped between [`Axis::MIN`] and [`Axis::MAX`] inclusive.
    pub fn get(&self, button_type: GamepadButtonType) -> Option<f32> {
        self.analog.get(button_type)
    }

    /// Returns the unclamped position data of the provided [`GamepadButtonType`].
    ///
    /// This value may be outside the [`Axis::MIN`] and [`Axis::MAX`] range.
    ///
    /// Use for things like camera zoom, where you want devices like mouse wheels to be able to
    /// exceed the normal range. If being able to move faster on one input device
    /// than another would give an unfair advantage, you should likely use [`Axis::get`] instead.
    pub fn get_unclamped(&self, button_type: GamepadButtonType) -> Option<f32> {
        self.analog.get_unclamped(button_type)
    }

    /// Returns `true` if the [`GamepadButtonType`] has been pressed.
    pub fn pressed(&self, button_type: GamepadButtonType) -> bool {
        self.digital.pressed(button_type)
    }

    /// Returns `true` if any item in [`GamepadButtonType`] has been pressed.
    pub fn any_pressed(&self, button_inputs: impl IntoIterator<Item = GamepadButtonType>) -> bool {
        button_inputs
            .into_iter()
            .any(|button_type| self.pressed(button_type))
    }

    /// Returns `true` if all items in [`GamepadButtonType`] have been pressed.
    pub fn all_pressed(&self, button_inputs: impl IntoIterator<Item = GamepadButtonType>) -> bool {
        button_inputs
            .into_iter()
            .all(|button_type| self.pressed(button_type))
    }

    /// Returns `true` if the [`GamepadButtonType`] has been pressed during the current frame.
    ///
    /// Note: This function does not imply information regarding the current state of [`ButtonInput::pressed`] or [`ButtonInput::just_released`].
    pub fn just_pressed(&self, button_type: GamepadButtonType) -> bool {
        self.digital.just_pressed(button_type)
    }

    /// Returns `true` if any item in [`GamepadButtonType`] has been pressed during the current frame.
    pub fn any_just_pressed(
        &self,
        button_inputs: impl IntoIterator<Item = GamepadButtonType>,
    ) -> bool {
        button_inputs
            .into_iter()
            .any(|button_type| self.just_pressed(button_type))
    }

    /// Returns `true` if all items in [`GamepadButtonType`] have been just pressed.
    pub fn all_just_pressed(
        &self,
        button_inputs: impl IntoIterator<Item = GamepadButtonType>,
    ) -> bool {
        button_inputs
            .into_iter()
            .all(|button_type| self.just_pressed(button_type))
    }

    /// Returns `true` if the [`GamepadButtonType`] has been released during the current frame.
    ///
    /// Note: This function does not imply information regarding the current state of [`ButtonInput::pressed`] or [`ButtonInput::just_pressed`].
    pub fn just_released(&self, button_type: GamepadButtonType) -> bool {
        self.digital.just_released(button_type)
    }

    /// Returns `true` if any item in [`GamepadButtonType`] has just been released.
    pub fn any_just_released(
        &self,
        button_inputs: impl IntoIterator<Item = GamepadButtonType>,
    ) -> bool {
        button_inputs
            .into_iter()
            .any(|button_type| self.just_released(button_type))
    }

    /// Returns `true` if all items in [`GamepadButtonType`] have just been released.
    pub fn all_just_released(
        &self,
        button_inputs: impl IntoIterator<Item = GamepadButtonType>,
    ) -> bool {
        button_inputs
            .into_iter()
            .all(|button_type| self.just_released(button_type))
    }

    /// Returns the current state of the button.
    pub fn pressed_state(&self, button: GamepadButtonType) -> ButtonState {
        ButtonState::from(self.digital.pressed_state(button))
    }
}

/// A type of gamepad axis.
///
/// ## Usage
///
/// This is used to determine which axis has changed its value when receiving a
/// gamepad axis event. It is also used in the [`GamepadAxes`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(Debug, Hash, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum GamepadAxisType {
    /// The horizontal value of the left stick.
    LeftStickX,
    /// The vertical value of the left stick.
    LeftStickY,
    /// The value of the left `Z` button.
    LeftZ,

    /// The horizontal value of the right stick.
    RightStickX,
    /// The vertical value of the right stick.
    RightStickY,
    /// The value of the right `Z` button.
    RightZ,

    /// Non-standard support for other axis types (i.e. HOTAS sliders, potentiometers, etc).
    Other(u8),
}

/// The [`GamepadAxes`] [`component`](Component) is a collection of [`GamepadAxisType`] and their state during the current frame.
///
/// The [`entity`](Entity) representing a gamepad and its [`minimal components`](MinimalGamepad) are automatically managed.
///
/// # Usage
///
/// The only way to obtain a [`GamepadAxes`] is by [`query`](Query).
///
/// # Examples
///
/// ```
/// # use bevy_input::gamepad::{Gamepad, GamepadAxes, GamepadAxisType};
/// # use bevy_ecs::system::Query;
/// #
/// fn gamepad_button_input_system(gamepads: Query<(&Gamepad, &GamepadAxes)>) {
///     for (gamepad, axis) in gamepads.iter() {
///         if let Some(left_stick_x) = axis.get(GamepadAxisType::LeftStickX)  {
///             println!("{} left stick X: {}", gamepad.id(), left_stick_x)
///         }
///     }
/// }
/// ```
#[derive(Component, Debug, Default)]
pub struct GamepadAxes {
    axis: Axis<GamepadAxisType>,
}

impl GamepadAxes {
    /// Returns the position data of the provided [`GamepadAxisType`].
    ///
    /// This will be clamped between [`Axis::MIN`] and [`Axis::MAX`] inclusive.
    pub fn get(&self, axis_type: GamepadAxisType) -> Option<f32> {
        self.axis.get(axis_type)
    }

    /// Returns the unclamped position data of the provided [`GamepadAxisType`].
    ///
    /// This value may be outside the [`Axis::MIN`] and [`Axis::MAX`] range.
    pub fn get_unclamped(&self, axis_type: GamepadAxisType) -> Option<f32> {
        self.axis.get_unclamped(axis_type)
    }

    /// Returns the left stick as a [`Vec2`]
    pub fn left_stick(&self) -> Vec2 {
        Vec2 {
            x: self.get(GamepadAxisType::LeftStickX).unwrap_or(0.0),
            y: self.get(GamepadAxisType::LeftStickY).unwrap_or(0.0),
        }
    }

    /// Returns the right stick as a [`Vec2`]
    pub fn right_stick(&self) -> Vec2 {
        Vec2 {
            x: self.get(GamepadAxisType::RightStickX).unwrap_or(0.0),
            y: self.get(GamepadAxisType::RightStickY).unwrap_or(0.0),
        }
    }
}

/// Gamepad settings component.
///
/// ## Usage
///
/// It is used to create a `bevy` component that stores the settings of [`GamepadButtonType`] in [`GamepadButtons`]
/// and [`GamepadAxisType`] in [`GamepadAxes`]. If no user defined [`ButtonSettings`], [`AxisSettings`], or [`ButtonAxisSettings`]
/// are defined, the default settings of each are used as a fallback accordingly.
///
/// ## Note
///
/// The [`GamepadSettings`] are used inside `bevy_input` to determine when raw gamepad events
/// should register. Events that don't meet the change thresholds defined in [`GamepadSettings`]
/// will not register. To modify these settings, mutate the corresponding component.
#[derive(Component, Clone, Default, Debug, Reflect)]
#[reflect(Debug, Default)]
pub struct GamepadSettings {
    /// The default button settings.
    pub default_button_settings: ButtonSettings,
    /// The default axis settings.
    pub default_axis_settings: AxisSettings,
    /// The default button axis settings.
    pub default_button_axis_settings: ButtonAxisSettings,
    /// The user defined button settings.
    pub button_settings: HashMap<GamepadButtonType, ButtonSettings>,
    /// The user defined axis settings.
    pub axis_settings: HashMap<GamepadAxisType, AxisSettings>,
    /// The user defined button axis settings.
    pub button_axis_settings: HashMap<GamepadButtonType, ButtonAxisSettings>,
}

impl GamepadSettings {
    /// Returns the [`ButtonSettings`] of the `button`.
    ///
    /// If no user defined [`ButtonSettings`] are specified the default [`ButtonSettings`] get returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadSettings, GamepadButtonType};
    /// #
    /// # let settings = GamepadSettings::default();
    /// let button_settings = settings.get_button_settings(GamepadButtonType::South);
    /// ```
    pub fn get_button_settings(&self, button: GamepadButtonType) -> &ButtonSettings {
        self.button_settings
            .get(&button)
            .unwrap_or(&self.default_button_settings)
    }

    /// Returns the [`AxisSettings`] of the `axis`.
    ///
    /// If no user defined [`AxisSettings`] are specified the default [`AxisSettings`] get returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadSettings, GamepadAxisType};
    /// #
    /// # let settings = GamepadSettings::default();
    /// let axis_settings = settings.get_axis_settings(GamepadAxisType::LeftStickX);
    /// ```
    pub fn get_axis_settings(&self, axis: GamepadAxisType) -> &AxisSettings {
        self.axis_settings
            .get(&axis)
            .unwrap_or(&self.default_axis_settings)
    }

    /// Returns the [`ButtonAxisSettings`] of the `button`.
    ///
    /// If no user defined [`ButtonAxisSettings`] are specified the default [`ButtonAxisSettings`] get returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadSettings, GamepadButtonType};
    /// #
    /// # let settings = GamepadSettings::default();
    /// let button_axis_settings = settings.get_button_axis_settings(GamepadButtonType::South);
    /// ```
    pub fn get_button_axis_settings(&self, button: GamepadButtonType) -> &ButtonAxisSettings {
        self.button_axis_settings
            .get(&button)
            .unwrap_or(&self.default_button_axis_settings)
    }
}

/// Manages settings for gamepad buttons.
///
/// It is used inside of [`GamepadSettings`] to define the threshold for a gamepad button
/// to be considered pressed or released. A button is considered pressed if the `press_threshold`
/// value is surpassed and released if the `release_threshold` value is undercut.
///
/// Allowed values: `0.0 <= ``release_threshold`` <= ``press_threshold`` <= 1.0`
#[derive(Debug, PartialEq, Clone, Reflect)]
#[reflect(Debug, Default)]
pub struct ButtonSettings {
    press_threshold: f32,
    release_threshold: f32,
}

impl Default for ButtonSettings {
    fn default() -> Self {
        ButtonSettings {
            press_threshold: 0.75,
            release_threshold: 0.65,
        }
    }
}

impl ButtonSettings {
    /// Creates a new [`ButtonSettings`] instance.
    ///
    /// # Parameters
    ///
    /// + `press_threshold` is the button input value above which the button is considered pressed.
    /// + `release_threshold` is the button input value below which the button is considered released.
    ///
    /// Restrictions:
    /// + `0.0 <= ``release_threshold`` <= ``press_threshold`` <= 1.0`
    ///
    /// # Errors
    ///
    /// If the restrictions are not met, returns one of
    /// `GamepadSettingsError::ButtonReleaseThresholdOutOfRange`,
    /// `GamepadSettingsError::ButtonPressThresholdOutOfRange`, or
    /// `GamepadSettingsError::ButtonReleaseThresholdGreaterThanPressThreshold`.
    pub fn new(
        press_threshold: f32,
        release_threshold: f32,
    ) -> Result<ButtonSettings, ButtonSettingsError> {
        if !(0.0..=1.0).contains(&release_threshold) {
            Err(ButtonSettingsError::ReleaseThresholdOutOfRange(
                release_threshold,
            ))
        } else if !(0.0..=1.0).contains(&press_threshold) {
            Err(ButtonSettingsError::PressThresholdOutOfRange(
                press_threshold,
            ))
        } else if release_threshold > press_threshold {
            Err(
                ButtonSettingsError::ReleaseThresholdGreaterThanPressThreshold {
                    press_threshold,
                    release_threshold,
                },
            )
        } else {
            Ok(ButtonSettings {
                press_threshold,
                release_threshold,
            })
        }
    }

    /// Returns `true` if the button is pressed.
    ///
    /// A button is considered pressed if the `value` passed is greater than or equal to the press threshold.
    pub fn is_pressed(&self, value: f32) -> bool {
        value >= self.press_threshold
    }

    /// Returns `true` if the button is released.
    ///
    /// A button is considered released if the `value` passed is lower than or equal to the release threshold.
    pub fn is_released(&self, value: f32) -> bool {
        value <= self.release_threshold
    }

    /// Get the button input threshold above which the button is considered pressed.
    pub fn press_threshold(&self) -> f32 {
        self.press_threshold
    }

    /// Try to set the button input threshold above which the button is considered pressed.
    ///
    /// # Errors
    ///
    /// If the value passed is outside the range [release threshold..=1.0], returns either
    /// `GamepadSettingsError::ButtonPressThresholdOutOfRange` or
    /// `GamepadSettingsError::ButtonReleaseThresholdGreaterThanPressThreshold`.
    pub fn try_set_press_threshold(&mut self, value: f32) -> Result<(), ButtonSettingsError> {
        if (self.release_threshold..=1.0).contains(&value) {
            self.press_threshold = value;
            Ok(())
        } else if !(0.0..1.0).contains(&value) {
            Err(ButtonSettingsError::PressThresholdOutOfRange(value))
        } else {
            Err(
                ButtonSettingsError::ReleaseThresholdGreaterThanPressThreshold {
                    press_threshold: value,
                    release_threshold: self.release_threshold,
                },
            )
        }
    }

    /// Try to set the button input threshold above which the button is considered pressed.
    /// If the value passed is outside the range [release threshold..=1.0], the value will not be changed.
    ///
    /// Returns the new value of the press threshold.
    pub fn set_press_threshold(&mut self, value: f32) -> f32 {
        self.try_set_press_threshold(value).ok();
        self.press_threshold
    }

    /// Get the button input threshold below which the button is considered released.
    pub fn release_threshold(&self) -> f32 {
        self.release_threshold
    }

    /// Try to set the button input threshold below which the button is considered released.
    ///
    /// # Errors
    ///
    /// If the value passed is outside the range [0.0..=press threshold], returns
    /// `ButtonSettingsError::ReleaseThresholdOutOfRange` or
    /// `ButtonSettingsError::ReleaseThresholdGreaterThanPressThreshold`.
    pub fn try_set_release_threshold(&mut self, value: f32) -> Result<(), ButtonSettingsError> {
        if (0.0..=self.press_threshold).contains(&value) {
            self.release_threshold = value;
            Ok(())
        } else if !(0.0..1.0).contains(&value) {
            Err(ButtonSettingsError::ReleaseThresholdOutOfRange(value))
        } else {
            Err(
                ButtonSettingsError::ReleaseThresholdGreaterThanPressThreshold {
                    press_threshold: self.press_threshold,
                    release_threshold: value,
                },
            )
        }
    }

    /// Try to set the button input threshold below which the button is considered released. If the
    /// value passed is outside the range [0.0..=press threshold], the value will not be changed.
    ///
    /// Returns the new value of the release threshold.
    pub fn set_release_threshold(&mut self, value: f32) -> f32 {
        self.try_set_release_threshold(value).ok();
        self.release_threshold
    }
}

/// Settings for a [`GamepadAxisType`].
///
/// It is used inside the [`GamepadSettings`] to define the sensitivity range and
/// threshold for an axis.
/// Values that are higher than `livezone_upperbound` will be rounded up to 1.0.
/// Values that are lower than `livezone_lowerbound` will be rounded down to -1.0.
/// Values that are in-between `deadzone_lowerbound` and `deadzone_upperbound` will be rounded
/// to 0.0.
/// Otherwise, values will not be rounded.
///
/// The valid range is `[-1.0, 1.0]`.
#[derive(Debug, Clone, Reflect, PartialEq)]
#[reflect(Debug, Default)]
pub struct AxisSettings {
    /// Values that are higher than `livezone_upperbound` will be rounded up to 1.0.
    livezone_upperbound: f32,
    /// Positive values that are less than `deadzone_upperbound` will be rounded down to 0.0.
    deadzone_upperbound: f32,
    /// Negative values that are greater than `deadzone_lowerbound` will be rounded up to 0.0.
    deadzone_lowerbound: f32,
    /// Values that are lower than `livezone_lowerbound` will be rounded down to -1.0.
    livezone_lowerbound: f32,
    /// `threshold` defines the minimum difference between old and new values to apply the changes.
    threshold: f32,
}

impl Default for AxisSettings {
    fn default() -> Self {
        AxisSettings {
            livezone_upperbound: 1.0,
            deadzone_upperbound: 0.05,
            deadzone_lowerbound: -0.05,
            livezone_lowerbound: -1.0,
            threshold: 0.01,
        }
    }
}

impl AxisSettings {
    /// Creates a new [`AxisSettings`] instance.
    ///
    /// # Arguments
    ///
    /// + `livezone_lowerbound` - the value below which inputs will be rounded down to -1.0.
    /// + `deadzone_lowerbound` - the value above which negative inputs will be rounded up to 0.0.
    /// + `deadzone_upperbound` - the value below which positive inputs will be rounded down to 0.0.
    /// + `livezone_upperbound` - the value above which inputs will be rounded up to 1.0.
    /// + `threshold` - the minimum value by which input must change before the change is registered.
    ///
    /// Restrictions:
    ///
    /// + `-1.0 <= livezone_lowerbound <= deadzone_lowerbound <= 0.0`
    /// + `0.0 <= deadzone_upperbound <= livezone_upperbound <= 1.0`
    /// + `0.0 <= threshold <= 2.0`
    ///
    /// # Errors
    ///
    /// Returns an [`AxisSettingsError`] if any restrictions on the zone values are not met.
    /// If the zone restrictions are met, but the `threshold` value restrictions are not met,
    /// returns [`AxisSettingsError::Threshold`].
    pub fn new(
        livezone_lowerbound: f32,
        deadzone_lowerbound: f32,
        deadzone_upperbound: f32,
        livezone_upperbound: f32,
        threshold: f32,
    ) -> Result<AxisSettings, AxisSettingsError> {
        if !(-1.0..=0.0).contains(&livezone_lowerbound) {
            Err(AxisSettingsError::LiveZoneLowerBoundOutOfRange(
                livezone_lowerbound,
            ))
        } else if !(-1.0..=0.0).contains(&deadzone_lowerbound) {
            Err(AxisSettingsError::DeadZoneLowerBoundOutOfRange(
                deadzone_lowerbound,
            ))
        } else if !(0.0..=1.0).contains(&deadzone_upperbound) {
            Err(AxisSettingsError::DeadZoneUpperBoundOutOfRange(
                deadzone_upperbound,
            ))
        } else if !(0.0..=1.0).contains(&livezone_upperbound) {
            Err(AxisSettingsError::LiveZoneUpperBoundOutOfRange(
                livezone_upperbound,
            ))
        } else if livezone_lowerbound > deadzone_lowerbound {
            Err(
                AxisSettingsError::LiveZoneLowerBoundGreaterThanDeadZoneLowerBound {
                    livezone_lowerbound,
                    deadzone_lowerbound,
                },
            )
        } else if deadzone_upperbound > livezone_upperbound {
            Err(
                AxisSettingsError::DeadZoneUpperBoundGreaterThanLiveZoneUpperBound {
                    livezone_upperbound,
                    deadzone_upperbound,
                },
            )
        } else if !(0.0..=2.0).contains(&threshold) {
            Err(AxisSettingsError::Threshold(threshold))
        } else {
            Ok(Self {
                livezone_lowerbound,
                deadzone_lowerbound,
                deadzone_upperbound,
                livezone_upperbound,
                threshold,
            })
        }
    }

    /// Get the value above which inputs will be rounded up to 1.0.
    pub fn livezone_upperbound(&self) -> f32 {
        self.livezone_upperbound
    }

    /// Try to set the value above which inputs will be rounded up to 1.0.
    ///
    /// # Errors
    ///
    /// If the value passed is less than the dead zone upper bound,
    /// returns `AxisSettingsError::DeadZoneUpperBoundGreaterThanLiveZoneUpperBound`.
    /// If the value passed is not in range [0.0..=1.0], returns `AxisSettingsError::LiveZoneUpperBoundOutOfRange`.
    pub fn try_set_livezone_upperbound(&mut self, value: f32) -> Result<(), AxisSettingsError> {
        if !(0.0..=1.0).contains(&value) {
            Err(AxisSettingsError::LiveZoneUpperBoundOutOfRange(value))
        } else if value < self.deadzone_upperbound {
            Err(
                AxisSettingsError::DeadZoneUpperBoundGreaterThanLiveZoneUpperBound {
                    livezone_upperbound: value,
                    deadzone_upperbound: self.deadzone_upperbound,
                },
            )
        } else {
            self.livezone_upperbound = value;
            Ok(())
        }
    }

    /// Try to set the value above which inputs will be rounded up to 1.0.
    /// If the value passed is negative or less than `deadzone_upperbound`,
    /// the value will not be changed.
    ///
    /// Returns the new value of `livezone_upperbound`.
    pub fn set_livezone_upperbound(&mut self, value: f32) -> f32 {
        self.try_set_livezone_upperbound(value).ok();
        self.livezone_upperbound
    }

    /// Get the value below which positive inputs will be rounded down to 0.0.
    pub fn deadzone_upperbound(&self) -> f32 {
        self.deadzone_upperbound
    }

    /// Try to set the value below which positive inputs will be rounded down to 0.0.
    ///
    /// # Errors
    ///
    /// If the value passed is greater than the live zone upper bound,
    /// returns `AxisSettingsError::DeadZoneUpperBoundGreaterThanLiveZoneUpperBound`.
    /// If the value passed is not in range [0.0..=1.0], returns `AxisSettingsError::DeadZoneUpperBoundOutOfRange`.
    pub fn try_set_deadzone_upperbound(&mut self, value: f32) -> Result<(), AxisSettingsError> {
        if !(0.0..=1.0).contains(&value) {
            Err(AxisSettingsError::DeadZoneUpperBoundOutOfRange(value))
        } else if self.livezone_upperbound < value {
            Err(
                AxisSettingsError::DeadZoneUpperBoundGreaterThanLiveZoneUpperBound {
                    livezone_upperbound: self.livezone_upperbound,
                    deadzone_upperbound: value,
                },
            )
        } else {
            self.deadzone_upperbound = value;
            Ok(())
        }
    }

    /// Try to set the value below which positive inputs will be rounded down to 0.0.
    /// If the value passed is negative or greater than `livezone_upperbound`,
    /// the value will not be changed.
    ///
    /// Returns the new value of `deadzone_upperbound`.
    pub fn set_deadzone_upperbound(&mut self, value: f32) -> f32 {
        self.try_set_deadzone_upperbound(value).ok();
        self.deadzone_upperbound
    }

    /// Get the value below which negative inputs will be rounded down to -1.0.
    pub fn livezone_lowerbound(&self) -> f32 {
        self.livezone_lowerbound
    }

    /// Try to set the value below which negative inputs will be rounded down to -1.0.
    ///
    /// # Errors
    ///
    /// If the value passed is less than the dead zone lower bound,
    /// returns `AxisSettingsError::LiveZoneLowerBoundGreaterThanDeadZoneLowerBound`.
    /// If the value passed is not in range [-1.0..=0.0], returns `AxisSettingsError::LiveZoneLowerBoundOutOfRange`.
    pub fn try_set_livezone_lowerbound(&mut self, value: f32) -> Result<(), AxisSettingsError> {
        if !(-1.0..=0.0).contains(&value) {
            Err(AxisSettingsError::LiveZoneLowerBoundOutOfRange(value))
        } else if value > self.deadzone_lowerbound {
            Err(
                AxisSettingsError::LiveZoneLowerBoundGreaterThanDeadZoneLowerBound {
                    livezone_lowerbound: value,
                    deadzone_lowerbound: self.deadzone_lowerbound,
                },
            )
        } else {
            self.livezone_lowerbound = value;
            Ok(())
        }
    }

    /// Try to set the value below which negative inputs will be rounded down to -1.0.
    /// If the value passed is positive or greater than `deadzone_lowerbound`,
    /// the value will not be changed.
    ///
    /// Returns the new value of `livezone_lowerbound`.
    pub fn set_livezone_lowerbound(&mut self, value: f32) -> f32 {
        self.try_set_livezone_lowerbound(value).ok();
        self.livezone_lowerbound
    }

    /// Get the value above which inputs will be rounded up to 0.0.
    pub fn deadzone_lowerbound(&self) -> f32 {
        self.deadzone_lowerbound
    }

    /// Try to set the value above which inputs will be rounded up to 0.0.
    ///
    /// # Errors
    ///
    /// If the value passed is less than the live zone lower bound,
    /// returns `AxisSettingsError::LiveZoneLowerBoundGreaterThanDeadZoneLowerBound`.
    /// If the value passed is not in range [-1.0..=0.0], returns `AxisSettingsError::DeadZoneLowerBoundOutOfRange`.
    pub fn try_set_deadzone_lowerbound(&mut self, value: f32) -> Result<(), AxisSettingsError> {
        if !(-1.0..=0.0).contains(&value) {
            Err(AxisSettingsError::DeadZoneLowerBoundOutOfRange(value))
        } else if self.livezone_lowerbound > value {
            Err(
                AxisSettingsError::LiveZoneLowerBoundGreaterThanDeadZoneLowerBound {
                    livezone_lowerbound: self.livezone_lowerbound,
                    deadzone_lowerbound: value,
                },
            )
        } else {
            self.deadzone_lowerbound = value;
            Ok(())
        }
    }

    /// Try to set the value above which inputs will be rounded up to 0.0.
    /// If the value passed is less than -1.0 or less than `livezone_lowerbound`,
    /// the value will not be changed.
    ///
    /// Returns the new value of `deadzone_lowerbound`.
    pub fn set_deadzone_lowerbound(&mut self, value: f32) -> f32 {
        self.try_set_deadzone_lowerbound(value).ok();
        self.deadzone_lowerbound
    }

    /// Get the minimum value by which input must change before the change is registered.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Try to set the minimum value by which input must change before the change is registered.
    ///
    /// # Errors
    ///
    /// If the value passed is not within [0.0..=2.0], returns `GamepadSettingsError::AxisThreshold`.
    pub fn try_set_threshold(&mut self, value: f32) -> Result<(), AxisSettingsError> {
        if !(0.0..=2.0).contains(&value) {
            Err(AxisSettingsError::Threshold(value))
        } else {
            self.threshold = value;
            Ok(())
        }
    }

    /// Try to set the minimum value by which input must change before the changes will be applied.
    /// If the value passed is not within [0.0..=2.0], the value will not be changed.
    ///
    /// Returns the new value of threshold.
    pub fn set_threshold(&mut self, value: f32) -> f32 {
        self.try_set_threshold(value).ok();
        self.threshold
    }

    /// Clamps the `raw_value` according to the `AxisSettings`.
    pub fn clamp(&self, new_value: f32) -> f32 {
        if self.deadzone_lowerbound <= new_value && new_value <= self.deadzone_upperbound {
            0.0
        } else if new_value >= self.livezone_upperbound {
            1.0
        } else if new_value <= self.livezone_lowerbound {
            -1.0
        } else {
            new_value
        }
    }

    /// Determines whether the change from `old_value` to `new_value` should
    /// be registered as a change, according to the [`AxisSettings`].
    fn should_register_change(&self, new_value: f32, old_value: Option<f32>) -> bool {
        if old_value.is_none() {
            return true;
        }

        f32::abs(new_value - old_value.unwrap()) > self.threshold
    }

    /// Filters the `new_value` based on the `old_value`, according to the [`AxisSettings`].
    ///
    /// Returns the clamped `new_value` if the change exceeds the settings threshold,
    /// and `None` otherwise.
    pub fn filter(&self, new_value: f32, old_value: Option<f32>) -> Option<f32> {
        let new_value = self.clamp(new_value);

        if self.should_register_change(new_value, old_value) {
            return Some(new_value);
        }
        None
    }
}

/// Settings for a [`GamepadButtonType`].
///
/// It is used inside the [`GamepadSettings`] to define the sensitivity range and
/// threshold for a button axis.
///
/// ## Logic
///
/// - Values that are higher than or equal to `high` will be rounded to 1.0.
/// - Values that are lower than or equal to `low` will be rounded to 0.0.
/// - Otherwise, values will not be rounded.
///
/// The valid range is from 0.0 to 1.0, inclusive.
#[derive(Debug, Clone, Reflect)]
#[reflect(Debug, Default)]
pub struct ButtonAxisSettings {
    /// The high value at which to apply rounding.
    pub high: f32,
    /// The low value at which to apply rounding.
    pub low: f32,
    /// The threshold to apply rounding.
    pub threshold: f32,
}

impl Default for ButtonAxisSettings {
    fn default() -> Self {
        ButtonAxisSettings {
            high: 0.95,
            low: 0.05,
            threshold: 0.01,
        }
    }
}

impl ButtonAxisSettings {
    /// Clamps the `raw_value` according to the specified settings.
    ///
    /// If the `raw_value` is:
    /// - lower than or equal to `low` it will be rounded to 0.0.
    /// - higher than or equal to `high` it will be rounded to 1.0.
    /// - Otherwise it will not be rounded.
    fn clamp(&self, raw_value: f32) -> f32 {
        if raw_value <= self.low {
            return 0.0;
        }
        if raw_value >= self.high {
            return 1.0;
        }

        raw_value
    }

    /// Determines whether the change from an `old_value` to a `new_value` should
    /// be registered as a change event, according to the specified settings.
    fn should_register_change(&self, new_value: f32, old_value: Option<f32>) -> bool {
        if old_value.is_none() {
            return true;
        }

        f32::abs(new_value - old_value.unwrap()) > self.threshold
    }

    /// Filters the `new_value` based on the `old_value`, according to the [`ButtonAxisSettings`].
    ///
    /// Returns the clamped `new_value`, according to the [`ButtonAxisSettings`], if the change
    /// exceeds the settings threshold, and `None` otherwise.
    pub fn filter(&self, new_value: f32, old_value: Option<f32>) -> Option<f32> {
        let new_value = self.clamp(new_value);

        if self.should_register_change(new_value, old_value) {
            return Some(new_value);
        }
        None
    }
}

/// Handles [`GamepadConnectionEvent`]s events.
///
/// On connection, spawns new entities with components representing a [`gamepad`](MinimalGamepad) and inserts them to the [`Gamepads`] resource.
/// On disconnection, despawns selected entities and removes them from the [`Gamepads`] resource.
///
/// ## Note
///
/// Whenever a [`GamepadId`] connects or disconnects, an information gets printed to the console using the [`info!`] macro.
pub fn gamepad_connection_system(
    mut commands: Commands,
    gamepads_settings: Query<&GamepadSettings>,
    mut gamepads: ResMut<Gamepads>,
    mut connection_events: EventReader<GamepadConnectionEvent>,
    mut preserved_settings: Local<HashMap<GamepadId, GamepadSettings>>,
) {
    for connection_event in connection_events.read() {
        let id = connection_event.gamepad;
        match &connection_event.connection {
            GamepadConnection::Connected(info) => {
                if gamepads.get_entity(id).is_some() {
                    warn!("Gamepad connection event on active gamepad. Connection event has been ignored");
                    continue;
                }
                let settings = preserved_settings
                    .get(&id)
                    .cloned()
                    .unwrap_or(GamepadSettings::default());
                let entity = commands
                    .spawn(MinimalGamepad::new(
                        Gamepad {
                            id,
                            info: info.clone(),
                        },
                        settings,
                    ))
                    .id();
                gamepads.id_to_entity.insert(id, entity);
                gamepads.entity_to_id.insert(entity, id);
                info!("{:?} Connected", id);
            }
            GamepadConnection::Disconnected => {
                if let Some(entity) = gamepads.id_to_entity.get(&id).copied() {
                    //.expect("GamepadId should exist in id_to_entity map");
                    // Preserve settings for reconnection event
                    let settings = gamepads_settings
                        .get(entity)
                        .cloned()
                        .unwrap_or(GamepadSettings::default());
                    preserved_settings.insert(id, settings);
                    gamepads.id_to_entity.remove(&id);
                    gamepads.entity_to_id.remove(&entity);
                    if let Some(mut entity_commands) = commands.get_entity(entity) {
                        entity_commands.despawn();
                    } else {
                        warn!("Gamepad entity was already de-spawned.");
                    }
                    info!("{:?} Disconnected", id);
                } else {
                    warn!("Gamepad disconnection event on inactive gamepad. Disconnection event has been ignored");
                }
            }
        }
    }
}

/// The connection status of a gamepad.
#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum GamepadConnection {
    /// The gamepad is connected.
    Connected(GamepadInfo),
    /// The gamepad is disconnected.
    Disconnected,
}

/// Consumes [`RawGamepadAxisChangedEvent`]s, filters them using their [`GamepadSettings`] and if successful, updates the [`GamepadAxes`] and sends a [`GamepadAxisChanged`] [`event`](Event).
pub fn gamepad_axis_event_system(
    // TODO: Change settings to Option<T>?
    mut gamepads_axis: Query<(&mut GamepadAxes, &GamepadSettings)>,
    gamepads_map: Res<Gamepads>,
    mut raw_events: EventReader<RawGamepadAxisChangedEvent>,
    mut filtered_events: EventWriter<GamepadAxisChanged>,
) {
    for axis_event in raw_events.read() {
        let Some(entity) = gamepads_map.get_entity(axis_event.gamepad) else {
            continue;
        };
        let Ok((mut gamepad_axis, gamepad_settings)) = gamepads_axis.get_mut(entity) else {
            continue;
        };
        let Some(filtered_value) = gamepad_settings
            .get_axis_settings(axis_event.axis_type)
            .filter(axis_event.value, gamepad_axis.get(axis_event.axis_type))
        else {
            continue;
        };

        gamepad_axis.axis.set(axis_event.axis_type, filtered_value);
        filtered_events.send(GamepadAxisChanged::new(
            entity,
            axis_event.gamepad,
            axis_event.axis_type,
            filtered_value,
        ));
    }
}

/// Consumes [`RawGamepadButtonChangedEvent`]s, filters them using their [`GamepadSettings`] and if successful, updates the [`GamepadButtons`] and sends a [`GamepadButtonStateChanged`] [`event`](Event).
pub fn gamepad_button_event_system(
    // TODO: Change settings to Option<T>?
    mut gamepads: Query<(&Gamepad, &mut GamepadButtons, &GamepadSettings)>,
    gamepads_map: Res<Gamepads>,
    mut raw_events: EventReader<RawGamepadButtonChangedEvent>,
    mut processed_digital_events: EventWriter<GamepadButtonStateChanged>,
    mut processed_analog_events: EventWriter<GamepadButtonChanged>,
) {
    // Clear digital buttons state
    for (_, mut gamepad_buttons, _) in gamepads.iter_mut() {
        gamepad_buttons.bypass_change_detection().digital.clear();
    }
    for event in raw_events.read() {
        let button = event.button_type;
        let Some(entity) = gamepads_map.get_entity(event.gamepad) else {
            continue;
        };
        let Ok((gamepad, mut buttons, settings)) = gamepads.get_mut(entity) else {
            continue;
        };
        let Some(filtered_value) = settings
            .get_button_axis_settings(button)
            .filter(event.value, buttons.get(button))
        else {
            continue;
        };
        let button_settings = settings.get_button_settings(button);
        buttons.analog.set(button, filtered_value);

        if button_settings.is_released(filtered_value) {
            // Check if button was previously pressed
            if buttons.pressed(button) {
                processed_digital_events.send(GamepadButtonStateChanged::new(
                    entity,
                    gamepad,
                    button,
                    ButtonState::Released,
                ));
            }
            // We don't have to check if the button was previously pressed here
            // because that check is performed within Input<T>::release()
            buttons.digital.release(button);
        } else if button_settings.is_pressed(filtered_value) {
            // Check if button was previously not pressed
            if !buttons.pressed(button) {
                processed_digital_events.send(GamepadButtonStateChanged::new(
                    entity,
                    gamepad,
                    button,
                    ButtonState::Pressed,
                ));
            }
            buttons.digital.press(button);
        };
        processed_analog_events.send(GamepadButtonChanged::new(
            entity,
            gamepad,
            button,
            buttons.pressed_state(button),
            filtered_value,
        ));
    }
}

/// An array of every [`GamepadButtonType`] variant.
pub const ALL_BUTTON_TYPES: [GamepadButtonType; 19] = [
    GamepadButtonType::South,
    GamepadButtonType::East,
    GamepadButtonType::North,
    GamepadButtonType::West,
    GamepadButtonType::C,
    GamepadButtonType::Z,
    GamepadButtonType::LeftTrigger,
    GamepadButtonType::LeftTrigger2,
    GamepadButtonType::RightTrigger,
    GamepadButtonType::RightTrigger2,
    GamepadButtonType::Select,
    GamepadButtonType::Start,
    GamepadButtonType::Mode,
    GamepadButtonType::LeftThumb,
    GamepadButtonType::RightThumb,
    GamepadButtonType::DPadUp,
    GamepadButtonType::DPadDown,
    GamepadButtonType::DPadLeft,
    GamepadButtonType::DPadRight,
];

/// An array of every [`GamepadAxisType`] variant.
pub const ALL_AXIS_TYPES: [GamepadAxisType; 6] = [
    GamepadAxisType::LeftStickX,
    GamepadAxisType::LeftStickY,
    GamepadAxisType::LeftZ,
    GamepadAxisType::RightStickX,
    GamepadAxisType::RightStickY,
    GamepadAxisType::RightZ,
];

/// The intensity at which a gamepad's force-feedback motors may rumble.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GamepadRumbleIntensity {
    /// The rumble intensity of the strong gamepad motor.
    ///
    /// Ranges from `0.0` to `1.0`.
    ///
    /// By convention, this is usually a low-frequency motor on the left-hand
    /// side of the gamepad, though it may vary across platforms and hardware.
    pub strong_motor: f32,
    /// The rumble intensity of the weak gamepad motor.
    ///
    /// Ranges from `0.0` to `1.0`.
    ///
    /// By convention, this is usually a high-frequency motor on the right-hand
    /// side of the gamepad, though it may vary across platforms and hardware.
    pub weak_motor: f32,
}

impl GamepadRumbleIntensity {
    /// Rumble both gamepad motors at maximum intensity.
    pub const MAX: Self = GamepadRumbleIntensity {
        strong_motor: 1.0,
        weak_motor: 1.0,
    };

    /// Rumble the weak motor at maximum intensity.
    pub const WEAK_MAX: Self = GamepadRumbleIntensity {
        strong_motor: 0.0,
        weak_motor: 1.0,
    };

    /// Rumble the strong motor at maximum intensity.
    pub const STRONG_MAX: Self = GamepadRumbleIntensity {
        strong_motor: 1.0,
        weak_motor: 0.0,
    };

    /// Creates a new rumble intensity with weak motor intensity set to the given value.
    ///
    /// Clamped within the `0.0` to `1.0` range.
    pub const fn weak_motor(intensity: f32) -> Self {
        Self {
            weak_motor: intensity,
            strong_motor: 0.0,
        }
    }

    /// Creates a new rumble intensity with strong motor intensity set to the given value.
    ///
    /// Clamped within the `0.0` to `1.0` range.
    pub const fn strong_motor(intensity: f32) -> Self {
        Self {
            strong_motor: intensity,
            weak_motor: 0.0,
        }
    }
}

/// An event that controls force-feedback rumbling of a [`GamepadId`].
///
/// # Notes
///
/// Does nothing if the gamepad or platform does not support rumble.
///
/// # Example
///
/// ```
/// # use bevy_input::gamepad::{Gamepad, GamepadRumbleRequest, GamepadRumbleIntensity, Gamepads};
/// # use bevy_ecs::prelude::{EventWriter, Res, Query};
/// # use bevy_utils::Duration;
/// fn rumble_gamepad_system(
///     mut rumble_requests: EventWriter<GamepadRumbleRequest>,
///     gamepads: Query<&Gamepad>,
/// ) {
///     for gamepad in gamepads.iter() {
///         rumble_requests.send(GamepadRumbleRequest::Add {
///             gamepad: gamepad.id(),
///             intensity: GamepadRumbleIntensity::MAX,
///             duration: Duration::from_secs_f32(0.5),
///         });
///     }
/// }
/// ```
#[doc(alias = "haptic feedback")]
#[doc(alias = "force feedback")]
#[doc(alias = "vibration")]
#[doc(alias = "vibrate")]
#[derive(Event, Clone)]
pub enum GamepadRumbleRequest {
    /// Add a rumble to the given gamepad.
    ///
    /// Simultaneous rumble effects add up to the sum of their strengths.
    ///
    /// Consequently, if two rumbles at half intensity are added at the same
    /// time, their intensities will be added up, and the controller will rumble
    /// at full intensity until one of the rumbles finishes, then the rumble
    /// will continue at the intensity of the remaining event.
    ///
    /// To replace an existing rumble, send a [`GamepadRumbleRequest::Stop`] event first.
    Add {
        /// How long the gamepad should rumble.
        duration: Duration,
        /// How intense the rumble should be.
        intensity: GamepadRumbleIntensity,
        /// The gamepad to rumble.
        gamepad: GamepadId,
    },
    /// Stop all running rumbles on the given [`GamepadId`].
    Stop {
        /// The gamepad to stop rumble.
        gamepad: GamepadId,
    },
}

impl GamepadRumbleRequest {
    /// Get the [`GamepadId`] associated with this request.
    pub fn gamepad(&self) -> GamepadId {
        match self {
            Self::Add { gamepad, .. } | Self::Stop { gamepad } => *gamepad,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gamepad::GamepadConnection::{Connected, Disconnected};
    use crate::gamepad::{
        gamepad_axis_event_system, gamepad_button_event_system, gamepad_connection_system,
        AxisSettingsError, ButtonSettingsError, Gamepad, GamepadAxes, GamepadAxisChanged,
        GamepadAxisType, GamepadButtonChanged, GamepadButtonStateChanged, GamepadButtonType,
        GamepadButtons, GamepadConnectionEvent, GamepadId, GamepadInfo, GamepadSettings, Gamepads,
        RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent,
    };
    use crate::ButtonState;
    use bevy_app::{App, PreUpdate};
    use bevy_ecs::entity::Entity;
    use bevy_ecs::event::Events;
    use bevy_ecs::schedule::IntoSystemConfigs;

    use super::{AxisSettings, ButtonAxisSettings, ButtonSettings};

    fn test_button_axis_settings_filter(
        settings: ButtonAxisSettings,
        new_value: f32,
        old_value: Option<f32>,
        expected: Option<f32>,
    ) {
        let actual = settings.filter(new_value, old_value);
        assert_eq!(
            expected, actual,
            "Testing filtering for {settings:?} with new_value = {new_value:?}, old_value = {old_value:?}",
        );
    }

    #[test]
    fn test_button_axis_settings_default_filter() {
        let cases = [
            (1.0, None, Some(1.0)),
            (0.99, None, Some(1.0)),
            (0.96, None, Some(1.0)),
            (0.95, None, Some(1.0)),
            (0.9499, None, Some(0.9499)),
            (0.84, None, Some(0.84)),
            (0.43, None, Some(0.43)),
            (0.05001, None, Some(0.05001)),
            (0.05, None, Some(0.0)),
            (0.04, None, Some(0.0)),
            (0.01, None, Some(0.0)),
            (0.0, None, Some(0.0)),
        ];

        for (new_value, old_value, expected) in cases {
            let settings = ButtonAxisSettings::default();
            test_button_axis_settings_filter(settings, new_value, old_value, expected);
        }
    }

    #[test]
    fn test_button_axis_settings_default_filter_with_old_value() {
        let cases = [
            (0.43, Some(0.44001), Some(0.43)),
            (0.43, Some(0.44), None),
            (0.43, Some(0.43), None),
            (0.43, Some(0.41999), Some(0.43)),
            (0.43, Some(0.17), Some(0.43)),
            (0.43, Some(0.84), Some(0.43)),
            (0.05, Some(0.055), Some(0.0)),
            (0.95, Some(0.945), Some(1.0)),
        ];

        for (new_value, old_value, expected) in cases {
            let settings = ButtonAxisSettings::default();
            test_button_axis_settings_filter(settings, new_value, old_value, expected);
        }
    }

    fn test_axis_settings_filter(
        settings: AxisSettings,
        new_value: f32,
        old_value: Option<f32>,
        expected: Option<f32>,
    ) {
        let actual = settings.filter(new_value, old_value);
        assert_eq!(
            expected, actual,
            "Testing filtering for {settings:?} with new_value = {new_value:?}, old_value = {old_value:?}",
        );
    }

    #[test]
    fn test_axis_settings_default_filter() {
        let cases = [
            (1.0, Some(1.0)),
            (0.99, Some(1.0)),
            (0.96, Some(1.0)),
            (0.95, Some(1.0)),
            (0.9499, Some(0.9499)),
            (0.84, Some(0.84)),
            (0.43, Some(0.43)),
            (0.05001, Some(0.05001)),
            (0.05, Some(0.0)),
            (0.04, Some(0.0)),
            (0.01, Some(0.0)),
            (0.0, Some(0.0)),
            (-1.0, Some(-1.0)),
            (-0.99, Some(-1.0)),
            (-0.96, Some(-1.0)),
            (-0.95, Some(-1.0)),
            (-0.9499, Some(-0.9499)),
            (-0.84, Some(-0.84)),
            (-0.43, Some(-0.43)),
            (-0.05001, Some(-0.05001)),
            (-0.05, Some(0.0)),
            (-0.04, Some(0.0)),
            (-0.01, Some(0.0)),
        ];

        for (new_value, expected) in cases {
            let settings = AxisSettings::new(-0.95, -0.05, 0.05, 0.95, 0.01).unwrap();
            test_axis_settings_filter(settings, new_value, None, expected);
        }
    }

    #[test]
    fn test_axis_settings_default_filter_with_old_values() {
        let cases = [
            (0.43, Some(0.44001), Some(0.43)),
            (0.43, Some(0.44), None),
            (0.43, Some(0.43), None),
            (0.43, Some(0.41999), Some(0.43)),
            (0.43, Some(0.17), Some(0.43)),
            (0.43, Some(0.84), Some(0.43)),
            (0.05, Some(0.055), Some(0.0)),
            (0.95, Some(0.945), Some(1.0)),
            (-0.43, Some(-0.44001), Some(-0.43)),
            (-0.43, Some(-0.44), None),
            (-0.43, Some(-0.43), None),
            (-0.43, Some(-0.41999), Some(-0.43)),
            (-0.43, Some(-0.17), Some(-0.43)),
            (-0.43, Some(-0.84), Some(-0.43)),
            (-0.05, Some(-0.055), Some(0.0)),
            (-0.95, Some(-0.945), Some(-1.0)),
        ];

        for (new_value, old_value, expected) in cases {
            let settings = AxisSettings::new(-0.95, -0.05, 0.05, 0.95, 0.01).unwrap();
            test_axis_settings_filter(settings, new_value, old_value, expected);
        }
    }

    #[test]
    fn test_button_settings_default_is_pressed() {
        let cases = [
            (1.0, true),
            (0.95, true),
            (0.9, true),
            (0.8, true),
            (0.75, true),
            (0.7, false),
            (0.65, false),
            (0.5, false),
            (0.0, false),
        ];

        for (value, expected) in cases {
            let settings = ButtonSettings::default();
            let actual = settings.is_pressed(value);

            assert_eq!(
                expected, actual,
                "testing ButtonSettings::is_pressed() for value: {value}",
            );
        }
    }

    #[test]
    fn test_button_settings_default_is_released() {
        let cases = [
            (1.0, false),
            (0.95, false),
            (0.9, false),
            (0.8, false),
            (0.75, false),
            (0.7, false),
            (0.65, true),
            (0.5, true),
            (0.0, true),
        ];

        for (value, expected) in cases {
            let settings = ButtonSettings::default();
            let actual = settings.is_released(value);

            assert_eq!(
                expected, actual,
                "testing ButtonSettings::is_released() for value: {value}",
            );
        }
    }

    #[test]
    fn test_new_button_settings_given_valid_parameters() {
        let cases = [
            (1.0, 0.0),
            (1.0, 1.0),
            (1.0, 0.9),
            (0.9, 0.9),
            (0.9, 0.0),
            (0.0, 0.0),
        ];

        for (press_threshold, release_threshold) in cases {
            let bs = ButtonSettings::new(press_threshold, release_threshold);
            match bs {
                Ok(button_settings) => {
                    assert_eq!(button_settings.press_threshold, press_threshold);
                    assert_eq!(button_settings.release_threshold, release_threshold);
                }
                Err(_) => {
                    panic!(
                        "ButtonSettings::new({press_threshold}, {release_threshold}) should be valid"
                    );
                }
            }
        }
    }

    #[test]
    fn test_new_button_settings_given_invalid_parameters() {
        let cases = [
            (1.1, 0.0),
            (1.1, 1.0),
            (1.0, 1.1),
            (-1.0, 0.9),
            (-1.0, 0.0),
            (-1.0, -0.4),
            (0.9, 1.0),
            (0.0, 0.1),
        ];

        for (press_threshold, release_threshold) in cases {
            let bs = ButtonSettings::new(press_threshold, release_threshold);
            match bs {
                Ok(_) => {
                    panic!(
                        "ButtonSettings::new({press_threshold}, {release_threshold}) should be invalid"
                    );
                }
                Err(err_code) => match err_code {
                    ButtonSettingsError::PressThresholdOutOfRange(_press_threshold) => {}
                    ButtonSettingsError::ReleaseThresholdGreaterThanPressThreshold {
                        press_threshold: _press_threshold,
                        release_threshold: _release_threshold,
                    } => {}
                    ButtonSettingsError::ReleaseThresholdOutOfRange(_release_threshold) => {}
                },
            }
        }
    }

    #[test]
    fn test_try_out_of_range_axis_settings() {
        let mut axis_settings = AxisSettings::default();
        assert_eq!(
            AxisSettings::new(-0.95, -0.05, 0.05, 0.95, 0.001),
            Ok(AxisSettings {
                livezone_lowerbound: -0.95,
                deadzone_lowerbound: -0.05,
                deadzone_upperbound: 0.05,
                livezone_upperbound: 0.95,
                threshold: 0.001,
            })
        );
        assert_eq!(
            Err(AxisSettingsError::LiveZoneLowerBoundOutOfRange(-2.0)),
            axis_settings.try_set_livezone_lowerbound(-2.0)
        );
        assert_eq!(
            Err(AxisSettingsError::LiveZoneLowerBoundOutOfRange(0.1)),
            axis_settings.try_set_livezone_lowerbound(0.1)
        );
        assert_eq!(
            Err(AxisSettingsError::DeadZoneLowerBoundOutOfRange(-2.0)),
            axis_settings.try_set_deadzone_lowerbound(-2.0)
        );
        assert_eq!(
            Err(AxisSettingsError::DeadZoneLowerBoundOutOfRange(0.1)),
            axis_settings.try_set_deadzone_lowerbound(0.1)
        );

        assert_eq!(
            Err(AxisSettingsError::DeadZoneUpperBoundOutOfRange(-0.1)),
            axis_settings.try_set_deadzone_upperbound(-0.1)
        );
        assert_eq!(
            Err(AxisSettingsError::DeadZoneUpperBoundOutOfRange(1.1)),
            axis_settings.try_set_deadzone_upperbound(1.1)
        );
        assert_eq!(
            Err(AxisSettingsError::LiveZoneUpperBoundOutOfRange(-0.1)),
            axis_settings.try_set_livezone_upperbound(-0.1)
        );
        assert_eq!(
            Err(AxisSettingsError::LiveZoneUpperBoundOutOfRange(1.1)),
            axis_settings.try_set_livezone_upperbound(1.1)
        );

        axis_settings.set_livezone_lowerbound(-0.7);
        axis_settings.set_deadzone_lowerbound(-0.3);
        assert_eq!(
            Err(
                AxisSettingsError::LiveZoneLowerBoundGreaterThanDeadZoneLowerBound {
                    livezone_lowerbound: -0.1,
                    deadzone_lowerbound: -0.3,
                }
            ),
            axis_settings.try_set_livezone_lowerbound(-0.1)
        );
        assert_eq!(
            Err(
                AxisSettingsError::LiveZoneLowerBoundGreaterThanDeadZoneLowerBound {
                    livezone_lowerbound: -0.7,
                    deadzone_lowerbound: -0.9
                }
            ),
            axis_settings.try_set_deadzone_lowerbound(-0.9)
        );

        axis_settings.set_deadzone_upperbound(0.3);
        axis_settings.set_livezone_upperbound(0.7);
        assert_eq!(
            Err(
                AxisSettingsError::DeadZoneUpperBoundGreaterThanLiveZoneUpperBound {
                    deadzone_upperbound: 0.8,
                    livezone_upperbound: 0.7
                }
            ),
            axis_settings.try_set_deadzone_upperbound(0.8)
        );
        assert_eq!(
            Err(
                AxisSettingsError::DeadZoneUpperBoundGreaterThanLiveZoneUpperBound {
                    deadzone_upperbound: 0.3,
                    livezone_upperbound: 0.1
                }
            ),
            axis_settings.try_set_livezone_upperbound(0.1)
        );
    }

    #[test]
    fn connect_gamepad() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_systems(PreUpdate, gamepad_connection_system);

        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                GamepadId(0),
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                GamepadId(1),
                Connected(GamepadInfo {
                    name: String::from("Gamepad test 1"),
                }),
            ));
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<(&Gamepad, &GamepadButtons, &GamepadAxes)>()
                .iter(app.world())
                .len(),
            2
        );
        assert_eq!(app.world().resource::<Gamepads>().entity_to_id.len(), 2);
        assert_eq!(app.world().resource::<Gamepads>().id_to_entity.len(), 2);
    }

    #[test]
    fn connect_existing_gamepad() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_systems(PreUpdate, gamepad_connection_system);

        let id = GamepadId(0);
        let connect_event = GamepadConnectionEvent::new(
            id,
            Connected(GamepadInfo {
                name: String::from("Gamepad test"),
            }),
        );

        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(connect_event.clone());
        app.update();

        assert_eq!(
            app.world_mut()
                .query::<(&Gamepad, &GamepadButtons, &GamepadAxes)>()
                .iter(app.world())
                .len(),
            1
        );
        assert_eq!(app.world().resource::<Gamepads>().entity_to_id.len(), 1);
        assert_eq!(app.world().resource::<Gamepads>().id_to_entity.len(), 1);

        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(connect_event);
        app.update();

        assert_eq!(
            app.world_mut()
                .query::<(&Gamepad, &GamepadButtons, &GamepadAxes)>()
                .iter(app.world())
                .len(),
            1
        );
        assert_eq!(app.world().resource::<Gamepads>().entity_to_id.len(), 1);
        assert_eq!(app.world().resource::<Gamepads>().id_to_entity.len(), 1);
    }

    #[test]
    fn disconnect_gamepad() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_systems(PreUpdate, gamepad_connection_system);

        // Spawn test entities
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                GamepadId(0),
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                GamepadId(1),
                Connected(GamepadInfo {
                    name: String::from("Gamepad test 1"),
                }),
            ));
        app.update();
        let mut query = app
            .world_mut()
            .query::<(Entity, &Gamepad, &GamepadButtons, &GamepadAxes)>();
        assert_eq!(app.world().resource::<Gamepads>().entity_to_id.len(), 2);
        assert_eq!(app.world().resource::<Gamepads>().id_to_entity.len(), 2);
        assert_eq!(query.iter(app.world()).len(), 2);
        for (entity, gamepad, _buttons, _axes) in query.iter(app.world()) {
            assert_eq!(
                app.world()
                    .resource::<Gamepads>()
                    .get_entity(gamepad)
                    .expect("Should have an entity"),
                entity
            );
            assert_eq!(
                app.world()
                    .resource::<Gamepads>()
                    .get_gamepad_id(entity)
                    .expect("Should have an id"),
                gamepad.id()
            );
        }

        // Despawn one gamepad
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(GamepadId(0), Disconnected));
        app.update();
        assert_eq!(query.iter(app.world()).len(), 1);
        assert_eq!(app.world().resource::<Gamepads>().entity_to_id.len(), 1);
        assert_eq!(app.world().resource::<Gamepads>().id_to_entity.len(), 1);
        for (entity, gamepad, _buttons, _axes) in query.iter(app.world()) {
            assert_eq!(
                app.world()
                    .resource::<Gamepads>()
                    .get_entity(gamepad)
                    .expect("Should have an entity"),
                entity
            );
            assert_eq!(
                app.world()
                    .resource::<Gamepads>()
                    .get_gamepad_id(entity)
                    .expect("Should have an id"),
                gamepad.id()
            );
        }
    }

    #[test]
    fn disconnect_nonexistant_gamepad() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_systems(PreUpdate, gamepad_connection_system);

        // Disconnection event on non-existent gamepad should be safely ignored
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(GamepadId(0), Disconnected));
        app.update();

        let mut query = app
            .world_mut()
            .query::<(Entity, &Gamepad, &GamepadButtons, &GamepadAxes)>();
        assert_eq!(query.iter(app.world()).len(), 0);
        assert_eq!(app.world().resource::<Gamepads>().entity_to_id.len(), 0);
        assert_eq!(app.world().resource::<Gamepads>().id_to_entity.len(), 0);
    }

    #[test]
    fn connection_gamepad_same_frame() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_systems(PreUpdate, gamepad_connection_system);
        let id = GamepadId(0);
        let connect_event = GamepadConnectionEvent::new(
            id,
            Connected(GamepadInfo {
                name: String::from("Gamepad test"),
            }),
        );
        let disconnect_event = GamepadConnectionEvent::new(id, Disconnected);

        // Connect and disconnect on the same frame
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send_batch([connect_event.clone(), disconnect_event.clone()]);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<(&Gamepad, &GamepadButtons, &GamepadAxes)>()
                .iter(app.world())
                .len(),
            0
        );
        assert_eq!(app.world().resource::<Gamepads>().entity_to_id.len(), 0);
        assert_eq!(app.world().resource::<Gamepads>().id_to_entity.len(), 0);

        // Reconnect on the same frame
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send_batch([connect_event.clone(), disconnect_event, connect_event]);
        app.update();
        assert_eq!(
            app.world_mut()
                .query::<(&Gamepad, &GamepadButtons, &GamepadAxes)>()
                .iter(app.world())
                .len(),
            1
        );
        assert_eq!(app.world().resource::<Gamepads>().entity_to_id.len(), 1);
        assert_eq!(app.world().resource::<Gamepads>().id_to_entity.len(), 1);
    }

    #[test]
    fn reconnected_gamepad_preserves_settings() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_systems(PreUpdate, gamepad_connection_system);
        let id = GamepadId(0);
        let mut settings = GamepadSettings::default();
        // Settings doesn't implement Eq so we will use button settings instead.
        settings.default_button_settings = ButtonSettings {
            press_threshold: settings.default_button_settings.press_threshold + 0.1,
            release_threshold: settings.default_button_settings.release_threshold - 0.1,
        };
        let connect_event = GamepadConnectionEvent::new(
            id,
            Connected(GamepadInfo {
                name: String::from("Gamepad test"),
            }),
        );
        let disconnect_event = GamepadConnectionEvent::new(id, Disconnected);

        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(connect_event.clone());
        app.update();

        // Update settings
        for mut gamepad_settings in app
            .world_mut()
            .query::<&mut GamepadSettings>()
            .iter_mut(app.world_mut())
        {
            *gamepad_settings = settings.clone();
        }
        app.update();

        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(disconnect_event);
        app.update();

        for gamepad_settings in app
            .world_mut()
            .query::<&GamepadSettings>()
            .iter(app.world())
        {
            assert_eq!(
                gamepad_settings.default_button_settings,
                settings.default_button_settings
            );
            assert_ne!(
                gamepad_settings.default_button_settings,
                GamepadSettings::default().default_button_settings
            );
        }
    }

    #[test]
    fn gamepad_axis_valid() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadAxisChangedEvent>()
            .add_event::<GamepadAxisChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_axis_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        app.world_mut()
            .resource_mut::<Events<RawGamepadAxisChangedEvent>>()
            .send_batch([
                RawGamepadAxisChangedEvent::new(id, GamepadAxisType::LeftStickY, 0.5),
                RawGamepadAxisChangedEvent::new(id, GamepadAxisType::RightStickX, 0.6),
                RawGamepadAxisChangedEvent::new(id, GamepadAxisType::RightZ, -0.4),
                RawGamepadAxisChangedEvent::new(id, GamepadAxisType::RightStickY, -0.8),
            ]);
        app.update();
        assert_eq!(
            app.world().resource::<Events<GamepadAxisChanged>>().len(),
            4
        );
    }

    #[test]
    fn gamepad_axis_threshold_filter() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadAxisChangedEvent>()
            .add_event::<GamepadAxisChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_axis_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        let settings = GamepadSettings::default().default_axis_settings;
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        // Set of events to ensure they are being properly filtered
        let base_value = 0.5;
        let events = [
            // Event above threshold
            RawGamepadAxisChangedEvent::new(id, GamepadAxisType::LeftStickX, base_value),
            // Event below threshold, should be filtered
            RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxisType::LeftStickX,
                base_value + settings.threshold - 0.01,
            ),
            // Event above threshold
            RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxisType::LeftStickX,
                base_value + settings.threshold + 0.01,
            ),
        ];
        app.world_mut()
            .resource_mut::<Events<RawGamepadAxisChangedEvent>>()
            .send_batch(events);
        app.update();
        assert_eq!(
            app.world().resource::<Events<GamepadAxisChanged>>().len(),
            2
        );
    }

    #[test]
    fn gamepad_axis_deadzone_filter() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadAxisChangedEvent>()
            .add_event::<GamepadAxisChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_axis_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        let settings = GamepadSettings::default().default_axis_settings;
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        // Set of events to ensure they are being properly filtered
        let events = [
            RawGamepadAxisChangedEvent::new(id, GamepadAxisType::LeftStickX, 0.0),
            // Event below deadzone upperbound should be filtered
            RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxisType::LeftStickX,
                settings.deadzone_upperbound - 0.01,
            ),
            // Event above deadzone lowerbound should be filtered
            RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxisType::LeftStickX,
                settings.deadzone_lowerbound + 0.01,
            ),
        ];
        app.world_mut()
            .resource_mut::<Events<RawGamepadAxisChangedEvent>>()
            .send_batch(events);
        app.update();
        assert_eq!(
            app.world().resource::<Events<GamepadAxisChanged>>().len(),
            1
        );
    }

    #[test]
    fn gamepad_axis_deadzone_rounded() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadAxisChangedEvent>()
            .add_event::<GamepadAxisChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_axis_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        let settings = GamepadSettings::default().default_axis_settings;
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        // Set of events to ensure they are being properly filtered
        let events = [
            RawGamepadAxisChangedEvent::new(id, GamepadAxisType::LeftStickX, 1.0),
            // Event below deadzone upperbound should be rounded to 0
            RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxisType::LeftStickX,
                settings.deadzone_upperbound - 0.01,
            ),
            RawGamepadAxisChangedEvent::new(id, GamepadAxisType::LeftStickX, 1.0),
            // Event above deadzone lowerbound should be rounded to 0
            RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxisType::LeftStickX,
                settings.deadzone_lowerbound + 0.01,
            ),
        ];
        let results = [1.0, 0.0, 1.0, 0.0];
        app.world_mut()
            .resource_mut::<Events<RawGamepadAxisChangedEvent>>()
            .send_batch(events);
        app.update();

        let events = app.world().resource::<Events<GamepadAxisChanged>>();
        let mut event_reader = events.get_reader();
        for (event, result) in event_reader.read(events).zip(results) {
            assert_eq!(event.value, result);
        }
        assert_eq!(
            app.world().resource::<Events<GamepadAxisChanged>>().len(),
            4
        );
    }

    #[test]
    fn gamepad_axis_livezone_filter() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadAxisChangedEvent>()
            .add_event::<GamepadAxisChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_axis_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        let settings = GamepadSettings::default().default_axis_settings;
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        // Set of events to ensure they are being properly filtered
        let events = [
            RawGamepadAxisChangedEvent::new(id, GamepadAxisType::LeftStickX, 1.0),
            // Event above livezone upperbound should be filtered
            RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxisType::LeftStickX,
                settings.livezone_upperbound + 0.01,
            ),
            RawGamepadAxisChangedEvent::new(id, GamepadAxisType::LeftStickX, -1.0),
            // Event below livezone lowerbound should be filtered
            RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxisType::LeftStickX,
                settings.livezone_lowerbound - 0.01,
            ),
        ];
        app.world_mut()
            .resource_mut::<Events<RawGamepadAxisChangedEvent>>()
            .send_batch(events);
        app.update();
        assert_eq!(
            app.world().resource::<Events<GamepadAxisChanged>>().len(),
            2
        );
    }

    #[test]
    fn gamepad_axis_livezone_rounded() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadAxisChangedEvent>()
            .add_event::<GamepadAxisChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_axis_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        let settings = GamepadSettings::default().default_axis_settings;
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        // Set of events to ensure they are being properly filtered
        let events = [
            RawGamepadAxisChangedEvent::new(id, GamepadAxisType::LeftStickX, 0.0),
            // Event above livezone upperbound should be rounded to 1
            RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxisType::LeftStickX,
                settings.livezone_upperbound + 0.01,
            ),
            // Event below livezone lowerbound should be rounded to 1
            RawGamepadAxisChangedEvent::new(
                id,
                GamepadAxisType::LeftStickX,
                settings.livezone_lowerbound - 0.01,
            ),
        ];
        let results = [0.0, 1.0, -1.0];
        app.world_mut()
            .resource_mut::<Events<RawGamepadAxisChangedEvent>>()
            .send_batch(events);
        app.update();

        let events = app.world().resource::<Events<GamepadAxisChanged>>();
        let mut event_reader = events.get_reader();
        for (event, result) in event_reader.read(events).zip(results) {
            assert_eq!(event.value, result);
        }
        assert_eq!(
            app.world().resource::<Events<GamepadAxisChanged>>().len(),
            3
        );
    }

    #[test]
    fn gamepad_buttons_pressed() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadButtonChangedEvent>()
            .add_event::<GamepadButtonChanged>()
            .add_event::<GamepadButtonStateChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_button_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        let digital_settings = GamepadSettings::default().default_button_settings;
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        let events = [RawGamepadButtonChangedEvent::new(
            id,
            GamepadButtonType::DPadDown,
            digital_settings.press_threshold,
        )];
        app.world_mut()
            .resource_mut::<Events<RawGamepadButtonChangedEvent>>()
            .send_batch(events);
        app.update();

        assert_eq!(
            app.world()
                .resource::<Events<GamepadButtonStateChanged>>()
                .len(),
            1
        );
        let events = app.world().resource::<Events<GamepadButtonStateChanged>>();
        let mut event_reader = events.get_reader();
        for event in event_reader.read(events) {
            assert_eq!(event.button, GamepadButtonType::DPadDown);
            assert_eq!(event.state, ButtonState::Pressed);
        }
        for buttons in app.world_mut().query::<&GamepadButtons>().iter(app.world()) {
            assert!(buttons.pressed(GamepadButtonType::DPadDown));
        }
        app.world_mut()
            .resource_mut::<Events<GamepadButtonStateChanged>>()
            .clear();
        app.update();

        assert_eq!(
            app.world()
                .resource::<Events<GamepadButtonStateChanged>>()
                .len(),
            0
        );
    }

    #[test]
    fn gamepad_buttons_just_pressed() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadButtonChangedEvent>()
            .add_event::<GamepadButtonChanged>()
            .add_event::<GamepadButtonStateChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_button_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        let digital_settings = GamepadSettings::default().default_button_settings;
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        app.world_mut()
            .resource_mut::<Events<RawGamepadButtonChangedEvent>>()
            .send(RawGamepadButtonChangedEvent::new(
                id,
                GamepadButtonType::DPadDown,
                digital_settings.press_threshold,
            ));
        app.update();

        // Check it is flagged for this frame
        for buttons in app.world_mut().query::<&GamepadButtons>().iter(app.world()) {
            assert!(buttons.just_pressed(GamepadButtonType::DPadDown));
        }
        app.update();

        //Check it clears next frame
        for buttons in app.world_mut().query::<&GamepadButtons>().iter(app.world()) {
            assert!(!buttons.just_pressed(GamepadButtonType::DPadDown));
        }
    }
    #[test]
    fn gamepad_buttons_released() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadButtonChangedEvent>()
            .add_event::<GamepadButtonChanged>()
            .add_event::<GamepadButtonStateChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_button_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        let digital_settings = GamepadSettings::default().default_button_settings;
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        app.world_mut()
            .resource_mut::<Events<RawGamepadButtonChangedEvent>>()
            .send(RawGamepadButtonChangedEvent::new(
                id,
                GamepadButtonType::DPadDown,
                digital_settings.press_threshold,
            ));
        app.update();

        app.world_mut()
            .resource_mut::<Events<GamepadButtonStateChanged>>()
            .clear();
        app.world_mut()
            .resource_mut::<Events<RawGamepadButtonChangedEvent>>()
            .send(RawGamepadButtonChangedEvent::new(
                id,
                GamepadButtonType::DPadDown,
                digital_settings.release_threshold - 0.01,
            ));
        app.update();
        assert_eq!(
            app.world()
                .resource::<Events<GamepadButtonStateChanged>>()
                .len(),
            1
        );
        let events = app.world().resource::<Events<GamepadButtonStateChanged>>();
        let mut event_reader = events.get_reader();
        for event in event_reader.read(events) {
            assert_eq!(event.button, GamepadButtonType::DPadDown);
            assert_eq!(event.state, ButtonState::Released);
        }
        for buttons in app.world_mut().query::<&GamepadButtons>().iter(app.world()) {
            assert!(!buttons.pressed(GamepadButtonType::DPadDown));
        }
        app.world_mut()
            .resource_mut::<Events<GamepadButtonStateChanged>>()
            .clear();
        app.update();

        assert_eq!(
            app.world()
                .resource::<Events<GamepadButtonStateChanged>>()
                .len(),
            0
        );
    }

    #[test]
    fn gamepad_buttons_just_released() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadButtonChangedEvent>()
            .add_event::<GamepadButtonChanged>()
            .add_event::<GamepadButtonStateChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_button_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        let digital_settings = GamepadSettings::default().default_button_settings;
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        app.world_mut()
            .resource_mut::<Events<RawGamepadButtonChangedEvent>>()
            .send_batch([
                RawGamepadButtonChangedEvent::new(
                    id,
                    GamepadButtonType::DPadDown,
                    digital_settings.press_threshold,
                ),
                RawGamepadButtonChangedEvent::new(
                    id,
                    GamepadButtonType::DPadDown,
                    digital_settings.release_threshold - 0.01,
                ),
            ]);
        app.update();

        // Check it is flagged for this frame
        for buttons in app.world_mut().query::<&GamepadButtons>().iter(app.world()) {
            assert!(buttons.just_released(GamepadButtonType::DPadDown));
        }
        app.update();

        //Check it clears next frame
        for buttons in app.world_mut().query::<&GamepadButtons>().iter(app.world()) {
            assert!(!buttons.just_released(GamepadButtonType::DPadDown));
        }
    }

    #[test]
    fn gamepad_buttons_axis() {
        let mut app = App::new();
        let app = app
            .init_resource::<Gamepads>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadButtonChangedEvent>()
            .add_event::<GamepadButtonChanged>()
            .add_event::<GamepadButtonStateChanged>()
            .add_systems(
                PreUpdate,
                (gamepad_connection_system, gamepad_button_event_system).chain(),
            );

        // Create test gamepad
        let id = GamepadId(0);
        let digital_settings = GamepadSettings::default().default_button_settings;
        let analog_settings = GamepadSettings::default().default_button_axis_settings;
        app.world_mut()
            .resource_mut::<Events<GamepadConnectionEvent>>()
            .send(GamepadConnectionEvent::new(
                id,
                Connected(GamepadInfo {
                    name: String::from("Gamepad test"),
                }),
            ));

        // Test events
        let events = [
            // Should trigger event
            RawGamepadButtonChangedEvent::new(
                id,
                GamepadButtonType::DPadDown,
                digital_settings.press_threshold,
            ),
            // Should trigger event
            RawGamepadButtonChangedEvent::new(
                id,
                GamepadButtonType::DPadDown,
                digital_settings.release_threshold,
            ),
            // Shouldn't trigger a state changed event
            RawGamepadButtonChangedEvent::new(
                id,
                GamepadButtonType::DPadDown,
                digital_settings.release_threshold - analog_settings.threshold * 1.01,
            ),
            // Shouldn't trigger any event
            RawGamepadButtonChangedEvent::new(
                id,
                GamepadButtonType::DPadDown,
                digital_settings.release_threshold - (analog_settings.threshold * 1.5),
            ),
            // Shouldn't trigger a state changed event
            RawGamepadButtonChangedEvent::new(
                id,
                GamepadButtonType::DPadDown,
                digital_settings.release_threshold - (analog_settings.threshold * 2.02),
            ),
        ];
        app.world_mut()
            .resource_mut::<Events<RawGamepadButtonChangedEvent>>()
            .send_batch(events);
        app.update();
        assert_eq!(
            app.world()
                .resource::<Events<GamepadButtonStateChanged>>()
                .len(),
            2
        );
        assert_eq!(
            app.world().resource::<Events<GamepadButtonChanged>>().len(),
            4
        );
    }
}
