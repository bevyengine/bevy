//! The gamepad input functionality.

use core::{ops::RangeInclusive, time::Duration};

use crate::{Axis, ButtonInput, ButtonState};
use alloc::string::String;
#[cfg(feature = "bevy_reflect")]
use bevy_ecs::prelude::ReflectComponent;
use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    entity::Entity,
    event::{BufferedEvent, EventReader, EventWriter},
    name::Name,
    system::{Commands, Query},
};
use bevy_math::ops;
use bevy_math::Vec2;
use bevy_platform::collections::HashMap;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use derive_more::derive::From;
use log::{info, warn};
use thiserror::Error;

/// A gamepad event.
///
/// This event type is used over the [`GamepadConnectionEvent`],
/// [`GamepadButtonChangedEvent`] and [`GamepadAxisChangedEvent`] when
/// the in-frame relative ordering of events is important.
///
/// This event is produced by `bevy_input`.
#[derive(BufferedEvent, Debug, Clone, PartialEq, From)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum GamepadEvent {
    /// A gamepad has been connected or disconnected.
    Connection(GamepadConnectionEvent),
    /// A button of the gamepad has been triggered.
    Button(GamepadButtonChangedEvent),
    /// An axis of the gamepad has been triggered.
    Axis(GamepadAxisChangedEvent),
}

/// A raw gamepad event.
///
/// This event type is used over the [`GamepadConnectionEvent`],
/// [`RawGamepadButtonChangedEvent`] and [`RawGamepadAxisChangedEvent`] when
/// the in-frame relative ordering of events is important.
///
/// This event type is used by `bevy_input` to feed its components.
#[derive(BufferedEvent, Debug, Clone, PartialEq, From)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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

/// [`GamepadButton`] changed event unfiltered by [`GamepadSettings`].
#[derive(BufferedEvent, Debug, Copy, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct RawGamepadButtonChangedEvent {
    /// The gamepad on which the button is triggered.
    pub gamepad: Entity,
    /// The type of the triggered button.
    pub button: GamepadButton,
    /// The value of the button.
    pub value: f32,
}

impl RawGamepadButtonChangedEvent {
    /// Creates a [`RawGamepadButtonChangedEvent`].
    pub fn new(gamepad: Entity, button_type: GamepadButton, value: f32) -> Self {
        Self {
            gamepad,
            button: button_type,
            value,
        }
    }
}

/// [`GamepadAxis`] changed event unfiltered by [`GamepadSettings`].
#[derive(BufferedEvent, Debug, Copy, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct RawGamepadAxisChangedEvent {
    /// The gamepad on which the axis is triggered.
    pub gamepad: Entity,
    /// The type of the triggered axis.
    pub axis: GamepadAxis,
    /// The value of the axis.
    pub value: f32,
}

impl RawGamepadAxisChangedEvent {
    /// Creates a [`RawGamepadAxisChangedEvent`].
    pub fn new(gamepad: Entity, axis_type: GamepadAxis, value: f32) -> Self {
        Self {
            gamepad,
            axis: axis_type,
            value,
        }
    }
}

/// A [`Gamepad`] connection event. Created when a connection to a gamepad
/// is established and when a gamepad is disconnected.
#[derive(BufferedEvent, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadConnectionEvent {
    /// The gamepad whose connection status changed.
    pub gamepad: Entity,
    /// The change in the gamepads connection.
    pub connection: GamepadConnection,
}

impl GamepadConnectionEvent {
    /// Creates a [`GamepadConnectionEvent`].
    pub fn new(gamepad: Entity, connection: GamepadConnection) -> Self {
        Self {
            gamepad,
            connection,
        }
    }

    /// Whether the gamepad is connected.
    pub fn connected(&self) -> bool {
        matches!(self.connection, GamepadConnection::Connected { .. })
    }

    /// Whether the gamepad is disconnected.
    pub fn disconnected(&self) -> bool {
        !self.connected()
    }
}

/// [`GamepadButton`] event triggered by a digital state change.
#[derive(BufferedEvent, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadButtonStateChangedEvent {
    /// The entity that represents this gamepad.
    pub entity: Entity,
    /// The gamepad button assigned to the event.
    pub button: GamepadButton,
    /// The pressed state of the button.
    pub state: ButtonState,
}

impl GamepadButtonStateChangedEvent {
    /// Creates a new [`GamepadButtonStateChangedEvent`].
    pub fn new(entity: Entity, button: GamepadButton, state: ButtonState) -> Self {
        Self {
            entity,
            button,
            state,
        }
    }
}

/// [`GamepadButton`] event triggered by an analog state change.
#[derive(BufferedEvent, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadButtonChangedEvent {
    /// The entity that represents this gamepad.
    pub entity: Entity,
    /// The gamepad button assigned to the event.
    pub button: GamepadButton,
    /// The pressed state of the button.
    pub state: ButtonState,
    /// The analog value of the button (rescaled to be in the 0.0..=1.0 range).
    pub value: f32,
}

impl GamepadButtonChangedEvent {
    /// Creates a new [`GamepadButtonChangedEvent`].
    pub fn new(entity: Entity, button: GamepadButton, state: ButtonState, value: f32) -> Self {
        Self {
            entity,
            button,
            state,
            value,
        }
    }
}

/// [`GamepadAxis`] event triggered by an analog state change.
#[derive(BufferedEvent, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(
    all(feature = "bevy_reflect", feature = "serialize"),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadAxisChangedEvent {
    /// The entity that represents this gamepad.
    pub entity: Entity,
    /// The gamepad axis assigned to the event.
    pub axis: GamepadAxis,
    /// The value of this axis (rescaled to account for axis settings).
    pub value: f32,
}

impl GamepadAxisChangedEvent {
    /// Creates a new [`GamepadAxisChangedEvent`].
    pub fn new(entity: Entity, axis: GamepadAxis, value: f32) -> Self {
        Self {
            entity,
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
    #[error("invalid parameter values livezone_lowerbound {} deadzone_lowerbound {}, expected livezone_lowerbound <= deadzone_lowerbound", livezone_lowerbound, deadzone_lowerbound)]
    LiveZoneLowerBoundGreaterThanDeadZoneLowerBound {
        /// The value of the `livezone_lowerbound` parameter.
        livezone_lowerbound: f32,
        /// The value of the `deadzone_lowerbound` parameter.
        deadzone_lowerbound: f32,
    },
    ///  Parameter `deadzone_upperbound` was not less than or equal to parameter `livezone_upperbound`.
    #[error("invalid parameter values livezone_upperbound {} deadzone_upperbound {}, expected deadzone_upperbound <= livezone_upperbound", livezone_upperbound, deadzone_upperbound)]
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
    #[error("invalid parameter values release_threshold {} press_threshold {}, expected release_threshold <= press_threshold", release_threshold, press_threshold)]
    ReleaseThresholdGreaterThanPressThreshold {
        /// The value of the `press_threshold` parameter.
        press_threshold: f32,
        /// The value of the `release_threshold` parameter.
        release_threshold: f32,
    },
}

/// Stores a connected gamepad's metadata such as the name and its [`GamepadButton`] and [`GamepadAxis`].
///
/// An entity with this component is spawned automatically after [`GamepadConnectionEvent`]
/// and updated by [`gamepad_event_processing_system`].
///
/// See also [`GamepadSettings`] for configuration.
///
/// # Examples
///
/// ```
/// # use bevy_input::gamepad::{Gamepad, GamepadAxis, GamepadButton};
/// # use bevy_ecs::system::Query;
/// # use bevy_ecs::name::Name;
/// #
/// fn gamepad_usage_system(gamepads: Query<(&Name, &Gamepad)>) {
///     for (name, gamepad) in &gamepads {
///         println!("{name}");
///
///         if gamepad.just_pressed(GamepadButton::North) {
///             println!("{} just pressed North", name)
///         }
///
///         if let Some(left_stick_x) = gamepad.get(GamepadAxis::LeftStickX)  {
///             println!("left stick X: {}", left_stick_x)
///         }
///     }
/// }
/// ```
#[derive(Component, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Component, Default)
)]
#[require(GamepadSettings)]
pub struct Gamepad {
    /// The USB vendor ID as assigned by the USB-IF, if available.
    pub(crate) vendor_id: Option<u16>,

    /// The USB product ID as assigned by the [vendor][Self::vendor_id], if available.
    pub(crate) product_id: Option<u16>,

    /// [`ButtonInput`] of [`GamepadButton`] representing their digital state.
    pub(crate) digital: ButtonInput<GamepadButton>,

    /// [`Axis`] of [`GamepadButton`] representing their analog state.
    pub(crate) analog: Axis<GamepadInput>,
}

impl Gamepad {
    /// Returns the USB vendor ID as assigned by the USB-IF, if available.
    pub fn vendor_id(&self) -> Option<u16> {
        self.vendor_id
    }

    /// Returns the USB product ID as assigned by the [vendor], if available.
    ///
    /// [vendor]: Self::vendor_id
    pub fn product_id(&self) -> Option<u16> {
        self.product_id
    }

    /// Returns the analog data of the provided [`GamepadAxis`] or [`GamepadButton`].
    ///
    /// This will be clamped between [[`Axis::MIN`],[`Axis::MAX`]].
    pub fn get(&self, input: impl Into<GamepadInput>) -> Option<f32> {
        self.analog.get(input.into())
    }

    /// Returns the unclamped analog data of the provided [`GamepadAxis`] or [`GamepadButton`].
    ///
    /// This value may be outside the [`Axis::MIN`] and [`Axis::MAX`] range.
    pub fn get_unclamped(&self, input: impl Into<GamepadInput>) -> Option<f32> {
        self.analog.get_unclamped(input.into())
    }

    /// Returns the left stick as a [`Vec2`].
    pub fn left_stick(&self) -> Vec2 {
        Vec2 {
            x: self.get(GamepadAxis::LeftStickX).unwrap_or(0.0),
            y: self.get(GamepadAxis::LeftStickY).unwrap_or(0.0),
        }
    }

    /// Returns the right stick as a [`Vec2`].
    pub fn right_stick(&self) -> Vec2 {
        Vec2 {
            x: self.get(GamepadAxis::RightStickX).unwrap_or(0.0),
            y: self.get(GamepadAxis::RightStickY).unwrap_or(0.0),
        }
    }

    /// Returns the directional pad as a [`Vec2`].
    pub fn dpad(&self) -> Vec2 {
        Vec2 {
            x: self.get(GamepadButton::DPadRight).unwrap_or(0.0)
                - self.get(GamepadButton::DPadLeft).unwrap_or(0.0),
            y: self.get(GamepadButton::DPadUp).unwrap_or(0.0)
                - self.get(GamepadButton::DPadDown).unwrap_or(0.0),
        }
    }

    /// Returns `true` if the [`GamepadButton`] has been pressed.
    pub fn pressed(&self, button_type: GamepadButton) -> bool {
        self.digital.pressed(button_type)
    }

    /// Returns `true` if any item in the [`GamepadButton`] iterator has been pressed.
    pub fn any_pressed(&self, button_inputs: impl IntoIterator<Item = GamepadButton>) -> bool {
        self.digital.any_pressed(button_inputs)
    }

    /// Returns `true` if all items in the [`GamepadButton`] iterator have been pressed.
    pub fn all_pressed(&self, button_inputs: impl IntoIterator<Item = GamepadButton>) -> bool {
        self.digital.all_pressed(button_inputs)
    }

    /// Returns `true` if the [`GamepadButton`] has been pressed during the current frame.
    ///
    /// Note: This function does not imply information regarding the current state of [`ButtonInput::pressed`] or [`ButtonInput::just_released`].
    pub fn just_pressed(&self, button_type: GamepadButton) -> bool {
        self.digital.just_pressed(button_type)
    }

    /// Returns `true` if any item in the [`GamepadButton`] iterator has been pressed during the current frame.
    pub fn any_just_pressed(&self, button_inputs: impl IntoIterator<Item = GamepadButton>) -> bool {
        self.digital.any_just_pressed(button_inputs)
    }

    /// Returns `true` if all items in the [`GamepadButton`] iterator have been just pressed.
    pub fn all_just_pressed(&self, button_inputs: impl IntoIterator<Item = GamepadButton>) -> bool {
        self.digital.all_just_pressed(button_inputs)
    }

    /// Returns `true` if the [`GamepadButton`] has been released during the current frame.
    ///
    /// Note: This function does not imply information regarding the current state of [`ButtonInput::pressed`] or [`ButtonInput::just_pressed`].
    pub fn just_released(&self, button_type: GamepadButton) -> bool {
        self.digital.just_released(button_type)
    }

    /// Returns `true` if any item in the [`GamepadButton`] iterator has just been released.
    pub fn any_just_released(
        &self,
        button_inputs: impl IntoIterator<Item = GamepadButton>,
    ) -> bool {
        self.digital.any_just_released(button_inputs)
    }

    /// Returns `true` if all items in the [`GamepadButton`] iterator have just been released.
    pub fn all_just_released(
        &self,
        button_inputs: impl IntoIterator<Item = GamepadButton>,
    ) -> bool {
        self.digital.all_just_released(button_inputs)
    }

    /// Returns an iterator over all digital [button]s that are pressed.
    ///
    /// [button]: GamepadButton
    pub fn get_pressed(&self) -> impl Iterator<Item = &GamepadButton> {
        self.digital.get_pressed()
    }

    /// Returns an iterator over all digital [button]s that were just pressed.
    ///
    /// [button]: GamepadButton
    pub fn get_just_pressed(&self) -> impl Iterator<Item = &GamepadButton> {
        self.digital.get_just_pressed()
    }

    /// Returns an iterator over all digital [button]s that were just released.
    ///
    /// [button]: GamepadButton
    pub fn get_just_released(&self) -> impl Iterator<Item = &GamepadButton> {
        self.digital.get_just_released()
    }

    /// Returns an iterator over all analog [axes][GamepadInput].
    pub fn get_analog_axes(&self) -> impl Iterator<Item = &GamepadInput> {
        self.analog.all_axes()
    }

    /// [`ButtonInput`] of [`GamepadButton`] representing their digital state.
    pub fn digital(&self) -> &ButtonInput<GamepadButton> {
        &self.digital
    }

    /// Mutable [`ButtonInput`] of [`GamepadButton`] representing their digital state. Useful for mocking inputs.
    pub fn digital_mut(&mut self) -> &mut ButtonInput<GamepadButton> {
        &mut self.digital
    }

    /// [`Axis`] of [`GamepadButton`] representing their analog state.
    pub fn analog(&self) -> &Axis<GamepadInput> {
        &self.analog
    }

    /// Mutable [`Axis`] of [`GamepadButton`] representing their analog state. Useful for mocking inputs.
    pub fn analog_mut(&mut self) -> &mut Axis<GamepadInput> {
        &mut self.analog
    }
}

impl Default for Gamepad {
    fn default() -> Self {
        let mut analog = Axis::default();
        for button in GamepadButton::all().iter().copied() {
            analog.set(button, 0.0);
        }
        for axis_type in GamepadAxis::all().iter().copied() {
            analog.set(axis_type, 0.0);
        }

        Self {
            vendor_id: None,
            product_id: None,
            digital: Default::default(),
            analog,
        }
    }
}

/// Represents gamepad input types that are mapped in the range [0.0, 1.0].
///
/// ## Usage
///
/// This is used to determine which button has changed its value when receiving gamepad button events.
/// It is also used in the [`Gamepad`] component.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum GamepadButton {
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

impl GamepadButton {
    /// Returns an array of all the standard [`GamepadButton`].
    pub const fn all() -> [GamepadButton; 19] {
        [
            GamepadButton::South,
            GamepadButton::East,
            GamepadButton::North,
            GamepadButton::West,
            GamepadButton::C,
            GamepadButton::Z,
            GamepadButton::LeftTrigger,
            GamepadButton::LeftTrigger2,
            GamepadButton::RightTrigger,
            GamepadButton::RightTrigger2,
            GamepadButton::Select,
            GamepadButton::Start,
            GamepadButton::Mode,
            GamepadButton::LeftThumb,
            GamepadButton::RightThumb,
            GamepadButton::DPadUp,
            GamepadButton::DPadDown,
            GamepadButton::DPadLeft,
            GamepadButton::DPadRight,
        ]
    }
}

/// Represents gamepad input types that are mapped in the range [-1.0, 1.0].
///
/// ## Usage
///
/// This is used to determine which axis has changed its value when receiving a
/// gamepad axis event. It is also used in the [`Gamepad`] component.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Hash, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum GamepadAxis {
    /// The horizontal value of the left stick.
    LeftStickX,
    /// The vertical value of the left stick.
    LeftStickY,
    /// Generally the throttle axis of a HOTAS setup.
    /// Refer to [`GamepadButton::LeftTrigger2`] for the analog trigger on a gamepad controller.
    LeftZ,
    /// The horizontal value of the right stick.
    RightStickX,
    /// The vertical value of the right stick.
    RightStickY,
    /// The yaw of the main joystick, not supported on common gamepads.
    /// Refer to [`GamepadButton::RightTrigger2`] for the analog trigger on a gamepad controller.
    RightZ,
    /// Non-standard support for other axis types (i.e. HOTAS sliders, potentiometers, etc).
    Other(u8),
}

impl GamepadAxis {
    /// Returns an array of all the standard [`GamepadAxis`].
    pub const fn all() -> [GamepadAxis; 6] {
        [
            GamepadAxis::LeftStickX,
            GamepadAxis::LeftStickY,
            GamepadAxis::LeftZ,
            GamepadAxis::RightStickX,
            GamepadAxis::RightStickY,
            GamepadAxis::RightZ,
        ]
    }
}

/// Encapsulation over [`GamepadAxis`] and [`GamepadButton`].
// This is done so Gamepad can share a single Axis<T> and simplifies the API by having only one get/get_unclamped method
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq, From)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq, Clone)
)]
pub enum GamepadInput {
    /// A [`GamepadAxis`].
    Axis(GamepadAxis),
    /// A [`GamepadButton`].
    Button(GamepadButton),
}

/// Gamepad settings component.
///
/// ## Usage
///
/// It is used to create a `bevy` component that stores the settings of [`GamepadButton`] and [`GamepadAxis`] in [`Gamepad`].
/// If no user defined [`ButtonSettings`], [`AxisSettings`], or [`ButtonAxisSettings`]
/// are defined, the default settings of each are used as a fallback accordingly.
///
/// ## Note
///
/// The [`GamepadSettings`] are used to determine when raw gamepad events
/// should register. Events that don't meet the change thresholds defined in [`GamepadSettings`]
/// will not register. To modify these settings, mutate the corresponding component.
#[derive(Component, Clone, Default, Debug)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Default, Component, Clone)
)]
pub struct GamepadSettings {
    /// The default button settings.
    pub default_button_settings: ButtonSettings,
    /// The default axis settings.
    pub default_axis_settings: AxisSettings,
    /// The default button axis settings.
    pub default_button_axis_settings: ButtonAxisSettings,
    /// The user defined button settings.
    pub button_settings: HashMap<GamepadButton, ButtonSettings>,
    /// The user defined axis settings.
    pub axis_settings: HashMap<GamepadAxis, AxisSettings>,
    /// The user defined button axis settings.
    pub button_axis_settings: HashMap<GamepadButton, ButtonAxisSettings>,
}

impl GamepadSettings {
    /// Returns the [`ButtonSettings`] of the [`GamepadButton`].
    ///
    /// If no user defined [`ButtonSettings`] are specified the default [`ButtonSettings`] get returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadSettings, GamepadButton};
    /// #
    /// # let settings = GamepadSettings::default();
    /// let button_settings = settings.get_button_settings(GamepadButton::South);
    /// ```
    pub fn get_button_settings(&self, button: GamepadButton) -> &ButtonSettings {
        self.button_settings
            .get(&button)
            .unwrap_or(&self.default_button_settings)
    }

    /// Returns the [`AxisSettings`] of the [`GamepadAxis`].
    ///
    /// If no user defined [`AxisSettings`] are specified the default [`AxisSettings`] get returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadSettings, GamepadAxis};
    /// #
    /// # let settings = GamepadSettings::default();
    /// let axis_settings = settings.get_axis_settings(GamepadAxis::LeftStickX);
    /// ```
    pub fn get_axis_settings(&self, axis: GamepadAxis) -> &AxisSettings {
        self.axis_settings
            .get(&axis)
            .unwrap_or(&self.default_axis_settings)
    }

    /// Returns the [`ButtonAxisSettings`] of the [`GamepadButton`].
    ///
    /// If no user defined [`ButtonAxisSettings`] are specified the default [`ButtonAxisSettings`] get returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadSettings, GamepadButton};
    /// #
    /// # let settings = GamepadSettings::default();
    /// let button_axis_settings = settings.get_button_axis_settings(GamepadButton::South);
    /// ```
    pub fn get_button_axis_settings(&self, button: GamepadButton) -> &ButtonAxisSettings {
        self.button_axis_settings
            .get(&button)
            .unwrap_or(&self.default_button_axis_settings)
    }
}

/// Manages settings for gamepad buttons.
///
/// It is used inside [`GamepadSettings`] to define the threshold for a [`GamepadButton`]
/// to be considered pressed or released. A button is considered pressed if the `press_threshold`
/// value is surpassed and released if the `release_threshold` value is undercut.
///
/// Allowed values: `0.0 <= ``release_threshold`` <= ``press_threshold`` <= 1.0`
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Default, Clone)
)]
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

/// Settings for a [`GamepadAxis`].
///
/// It is used inside the [`GamepadSettings`] to define the sensitivity range and
/// threshold for an axis.
/// Values that are higher than `livezone_upperbound` will be rounded up to 1.0.
/// Values that are lower than `livezone_lowerbound` will be rounded down to -1.0.
/// Values that are in-between `deadzone_lowerbound` and `deadzone_upperbound` will be rounded to 0.0.
/// Otherwise, values will be linearly rescaled to fit into the sensitivity range.
/// For example, a value that is one fourth of the way from `deadzone_upperbound` to `livezone_upperbound` will be scaled to 0.25.
///
/// The valid range is `[-1.0, 1.0]`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Default, Clone)
)]
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
    /// If the value passed is less than the deadzone upper bound,
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
    /// If the value passed is less than the deadzone lower bound,
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
    pub fn clamp(&self, raw_value: f32) -> f32 {
        if self.deadzone_lowerbound <= raw_value && raw_value <= self.deadzone_upperbound {
            0.0
        } else if raw_value >= self.livezone_upperbound {
            1.0
        } else if raw_value <= self.livezone_lowerbound {
            -1.0
        } else {
            raw_value
        }
    }

    /// Determines whether the change from `old_raw_value` to `new_raw_value` should
    /// be registered as a change, according to the [`AxisSettings`].
    fn should_register_change(&self, new_raw_value: f32, old_raw_value: Option<f32>) -> bool {
        match old_raw_value {
            None => true,
            Some(old_raw_value) => ops::abs(new_raw_value - old_raw_value) >= self.threshold,
        }
    }

    /// Filters the `new_raw_value` based on the `old_raw_value`, according to the [`AxisSettings`].
    ///
    /// Returns the clamped and scaled `new_raw_value` if the change exceeds the settings threshold,
    /// and `None` otherwise.
    fn filter(
        &self,
        new_raw_value: f32,
        old_raw_value: Option<f32>,
    ) -> Option<FilteredAxisPosition> {
        let clamped_unscaled = self.clamp(new_raw_value);
        match self.should_register_change(clamped_unscaled, old_raw_value) {
            true => Some(FilteredAxisPosition {
                scaled: self.get_axis_position_from_value(clamped_unscaled),
                raw: new_raw_value,
            }),
            false => None,
        }
    }

    #[inline(always)]
    fn get_axis_position_from_value(&self, value: f32) -> ScaledAxisWithDeadZonePosition {
        if value < self.deadzone_upperbound && value > self.deadzone_lowerbound {
            ScaledAxisWithDeadZonePosition::Dead
        } else if value > self.livezone_upperbound {
            ScaledAxisWithDeadZonePosition::AboveHigh
        } else if value < self.livezone_lowerbound {
            ScaledAxisWithDeadZonePosition::BelowLow
        } else if value >= self.deadzone_upperbound {
            ScaledAxisWithDeadZonePosition::High(linear_remapping(
                value,
                self.deadzone_upperbound..=self.livezone_upperbound,
                0.0..=1.0,
            ))
        } else if value <= self.deadzone_lowerbound {
            ScaledAxisWithDeadZonePosition::Low(linear_remapping(
                value,
                self.livezone_lowerbound..=self.deadzone_lowerbound,
                -1.0..=0.0,
            ))
        } else {
            unreachable!();
        }
    }
}

/// A linear remapping of `value` from `old` to `new`.
fn linear_remapping(value: f32, old: RangeInclusive<f32>, new: RangeInclusive<f32>) -> f32 {
    // https://stackoverflow.com/a/929104
    ((value - old.start()) / (old.end() - old.start())) * (new.end() - new.start()) + new.start()
}

#[derive(Debug, Clone, Copy)]
/// Deadzone-aware axis position.
enum ScaledAxisWithDeadZonePosition {
    /// The input clipped below the valid range of the axis.
    BelowLow,
    /// The input is lower than the deadzone.
    Low(f32),
    /// The input falls within the deadzone, meaning it is counted as 0.
    Dead,
    /// The input is higher than the deadzone.
    High(f32),
    /// The input clipped above the valid range of the axis.
    AboveHigh,
}

struct FilteredAxisPosition {
    scaled: ScaledAxisWithDeadZonePosition,
    raw: f32,
}

impl ScaledAxisWithDeadZonePosition {
    /// Converts the value into a float in the range [-1, 1].
    fn to_f32(self) -> f32 {
        match self {
            ScaledAxisWithDeadZonePosition::BelowLow => -1.,
            ScaledAxisWithDeadZonePosition::Low(scaled)
            | ScaledAxisWithDeadZonePosition::High(scaled) => scaled,
            ScaledAxisWithDeadZonePosition::Dead => 0.,
            ScaledAxisWithDeadZonePosition::AboveHigh => 1.,
        }
    }
}

#[derive(Debug, Clone, Copy)]
/// Low/High-aware axis position.
enum ScaledAxisPosition {
    /// The input fell short of the "low" value.
    ClampedLow,
    /// The input was in the normal range.
    Scaled(f32),
    /// The input surpassed the "high" value.
    ClampedHigh,
}

struct FilteredButtonAxisPosition {
    scaled: ScaledAxisPosition,
    raw: f32,
}

impl ScaledAxisPosition {
    /// Converts the value into a float in the range [0, 1].
    fn to_f32(self) -> f32 {
        match self {
            ScaledAxisPosition::ClampedLow => 0.,
            ScaledAxisPosition::Scaled(scaled) => scaled,
            ScaledAxisPosition::ClampedHigh => 1.,
        }
    }
}

/// Settings for a [`GamepadButton`].
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
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Default, Clone)
)]
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

    /// Determines whether the change from an `old_raw_value` to a `new_raw_value` should
    /// be registered as a change event, according to the specified settings.
    fn should_register_change(&self, new_raw_value: f32, old_raw_value: Option<f32>) -> bool {
        match old_raw_value {
            None => true,
            Some(old_raw_value) => ops::abs(new_raw_value - old_raw_value) >= self.threshold,
        }
    }

    /// Filters the `new_raw_value` based on the `old_raw_value`, according to the [`ButtonAxisSettings`].
    ///
    /// Returns the clamped and scaled `new_raw_value`, according to the [`ButtonAxisSettings`], if the change
    /// exceeds the settings threshold, and `None` otherwise.
    fn filter(
        &self,
        new_raw_value: f32,
        old_raw_value: Option<f32>,
    ) -> Option<FilteredButtonAxisPosition> {
        let clamped_unscaled = self.clamp(new_raw_value);
        match self.should_register_change(clamped_unscaled, old_raw_value) {
            true => Some(FilteredButtonAxisPosition {
                scaled: self.get_axis_position_from_value(clamped_unscaled),
                raw: new_raw_value,
            }),
            false => None,
        }
    }

    /// Clamps and scales the `value` according to the specified settings.
    ///
    /// If the `value` is:
    /// - lower than or equal to `low` it will be rounded to 0.0.
    /// - higher than or equal to `high` it will be rounded to 1.0.
    /// - Otherwise, it will be scaled from (low, high) to (0, 1).
    fn get_axis_position_from_value(&self, value: f32) -> ScaledAxisPosition {
        if value <= self.low {
            ScaledAxisPosition::ClampedLow
        } else if value >= self.high {
            ScaledAxisPosition::ClampedHigh
        } else {
            ScaledAxisPosition::Scaled(linear_remapping(value, self.low..=self.high, 0.0..=1.0))
        }
    }
}

/// Handles [`GamepadConnectionEvent`]s events.
///
/// On connection, adds the components representing a [`Gamepad`] to the entity.
/// On disconnection, removes the [`Gamepad`] and other related components.
/// Entities are left alive and might leave components like [`GamepadSettings`] to preserve state in the case of a reconnection.
///
/// ## Note
///
/// Whenever a [`Gamepad`] connects or disconnects, an information gets printed to the console using the [`info!`] macro.
pub fn gamepad_connection_system(
    mut commands: Commands,
    mut connection_events: EventReader<GamepadConnectionEvent>,
) {
    for connection_event in connection_events.read() {
        let id = connection_event.gamepad;
        match &connection_event.connection {
            GamepadConnection::Connected {
                name,
                vendor_id,
                product_id,
            } => {
                let Ok(mut gamepad) = commands.get_entity(id) else {
                    warn!("Gamepad {id} removed before handling connection event.");
                    continue;
                };
                gamepad.insert((
                    Name::new(name.clone()),
                    Gamepad {
                        vendor_id: *vendor_id,
                        product_id: *product_id,
                        ..Default::default()
                    },
                ));
                info!("Gamepad {id} connected.");
            }
            GamepadConnection::Disconnected => {
                let Ok(mut gamepad) = commands.get_entity(id) else {
                    warn!("Gamepad {id} removed before handling disconnection event. You can ignore this if you manually removed it.");
                    continue;
                };
                // Gamepad entities are left alive to preserve their state (e.g. [`GamepadSettings`]).
                // Instead of despawning, we remove Gamepad components that don't need to preserve state
                // and re-add them if they ever reconnect.
                gamepad.remove::<Gamepad>();
                info!("Gamepad {id} disconnected.");
            }
        }
    }
}

// Note that we don't expose `gilrs::Gamepad::uuid` due to
// https://gitlab.com/gilrs-project/gilrs/-/issues/153.
//
/// The connection status of a gamepad.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum GamepadConnection {
    /// The gamepad is connected.
    Connected {
        /// The name of the gamepad.
        ///
        /// This name is generally defined by the OS.
        ///
        /// For example on Windows the name may be "HID-compliant game controller".
        name: String,

        /// The USB vendor ID as assigned by the USB-IF, if available.
        vendor_id: Option<u16>,

        /// The USB product ID as assigned by the vendor, if available.
        product_id: Option<u16>,
    },
    /// The gamepad is disconnected.
    Disconnected,
}

/// Consumes [`RawGamepadEvent`] events, filters them using their [`GamepadSettings`] and if successful,
/// updates the [`Gamepad`] and sends [`GamepadAxisChangedEvent`], [`GamepadButtonStateChangedEvent`], [`GamepadButtonChangedEvent`] events.
pub fn gamepad_event_processing_system(
    mut gamepads: Query<(&mut Gamepad, &GamepadSettings)>,
    mut raw_events: EventReader<RawGamepadEvent>,
    mut processed_events: EventWriter<GamepadEvent>,
    mut processed_axis_events: EventWriter<GamepadAxisChangedEvent>,
    mut processed_digital_events: EventWriter<GamepadButtonStateChangedEvent>,
    mut processed_analog_events: EventWriter<GamepadButtonChangedEvent>,
) {
    // Clear digital buttons state
    for (mut gamepad, _) in gamepads.iter_mut() {
        gamepad.bypass_change_detection().digital.clear();
    }

    for event in raw_events.read() {
        match event {
            // Connections require inserting/removing components so they are done in a separate system
            RawGamepadEvent::Connection(send_event) => {
                processed_events.write(GamepadEvent::from(send_event.clone()));
            }
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent {
                gamepad,
                axis,
                value,
            }) => {
                let (gamepad, axis, value) = (*gamepad, *axis, *value);
                let Ok((mut gamepad_axis, gamepad_settings)) = gamepads.get_mut(gamepad) else {
                    continue;
                };
                let Some(filtered_value) = gamepad_settings
                    .get_axis_settings(axis)
                    .filter(value, gamepad_axis.get(axis))
                else {
                    continue;
                };
                gamepad_axis.analog.set(axis, filtered_value.raw);
                let send_event =
                    GamepadAxisChangedEvent::new(gamepad, axis, filtered_value.scaled.to_f32());
                processed_axis_events.write(send_event);
                processed_events.write(GamepadEvent::from(send_event));
            }
            RawGamepadEvent::Button(RawGamepadButtonChangedEvent {
                gamepad,
                button,
                value,
            }) => {
                let (gamepad, button, value) = (*gamepad, *button, *value);
                let Ok((mut gamepad_buttons, settings)) = gamepads.get_mut(gamepad) else {
                    continue;
                };
                let Some(filtered_value) = settings
                    .get_button_axis_settings(button)
                    .filter(value, gamepad_buttons.get(button))
                else {
                    continue;
                };
                let button_settings = settings.get_button_settings(button);
                gamepad_buttons.analog.set(button, filtered_value.raw);

                if button_settings.is_released(filtered_value.raw) {
                    // Check if button was previously pressed
                    if gamepad_buttons.pressed(button) {
                        processed_digital_events.write(GamepadButtonStateChangedEvent::new(
                            gamepad,
                            button,
                            ButtonState::Released,
                        ));
                    }
                    // We don't have to check if the button was previously pressed here
                    // because that check is performed within Input<T>::release()
                    gamepad_buttons.digital.release(button);
                } else if button_settings.is_pressed(filtered_value.raw) {
                    // Check if button was previously not pressed
                    if !gamepad_buttons.pressed(button) {
                        processed_digital_events.write(GamepadButtonStateChangedEvent::new(
                            gamepad,
                            button,
                            ButtonState::Pressed,
                        ));
                    }
                    gamepad_buttons.digital.press(button);
                };

                let button_state = if gamepad_buttons.digital.pressed(button) {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                };
                let send_event = GamepadButtonChangedEvent::new(
                    gamepad,
                    button,
                    button_state,
                    filtered_value.scaled.to_f32(),
                );
                processed_analog_events.write(send_event);
                processed_events.write(GamepadEvent::from(send_event));
            }
        }
    }
}

/// The intensity at which a gamepad's force-feedback motors may rumble.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
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

/// An event that controls force-feedback rumbling of a [`Gamepad`] [`entity`](Entity).
///
/// # Notes
///
/// Does nothing if the gamepad or platform does not support rumble.
///
/// # Example
///
/// ```
/// # use bevy_input::gamepad::{Gamepad, GamepadRumbleRequest, GamepadRumbleIntensity};
/// # use bevy_ecs::prelude::{EventWriter, Res, Query, Entity, With};
/// # use core::time::Duration;
/// fn rumble_gamepad_system(
///     mut rumble_requests: EventWriter<GamepadRumbleRequest>,
///     gamepads: Query<Entity, With<Gamepad>>,
/// ) {
///     for entity in gamepads.iter() {
///         rumble_requests.write(GamepadRumbleRequest::Add {
///             gamepad: entity,
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
#[derive(BufferedEvent, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Clone))]
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
        gamepad: Entity,
    },
    /// Stop all running rumbles on the given [`Entity`].
    Stop {
        /// The gamepad to stop rumble.
        gamepad: Entity,
    },
}

impl GamepadRumbleRequest {
    /// Get the [`Entity`] associated with this request.
    pub fn gamepad(&self) -> Entity {
        match self {
            Self::Add { gamepad, .. } | Self::Stop { gamepad } => *gamepad,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        gamepad_connection_system, gamepad_event_processing_system, AxisSettings,
        AxisSettingsError, ButtonAxisSettings, ButtonSettings, ButtonSettingsError, Gamepad,
        GamepadAxis, GamepadAxisChangedEvent, GamepadButton, GamepadButtonChangedEvent,
        GamepadButtonStateChangedEvent,
        GamepadConnection::{Connected, Disconnected},
        GamepadConnectionEvent, GamepadEvent, GamepadSettings, RawGamepadAxisChangedEvent,
        RawGamepadButtonChangedEvent, RawGamepadEvent,
    };
    use crate::ButtonState;
    use alloc::string::ToString;
    use bevy_app::{App, PreUpdate};
    use bevy_ecs::entity::Entity;
    use bevy_ecs::event::Events;
    use bevy_ecs::schedule::IntoScheduleConfigs;

    fn test_button_axis_settings_filter(
        settings: ButtonAxisSettings,
        new_raw_value: f32,
        old_raw_value: Option<f32>,
        expected: Option<f32>,
    ) {
        let actual = settings
            .filter(new_raw_value, old_raw_value)
            .map(|f| f.scaled.to_f32());
        assert_eq!(
            expected, actual,
            "Testing filtering for {settings:?} with new_raw_value = {new_raw_value:?}, old_raw_value = {old_raw_value:?}",
        );
    }

    #[test]
    fn test_button_axis_settings_default_filter() {
        let cases = [
            // clamped
            (1.0, None, Some(1.0)),
            (0.99, None, Some(1.0)),
            (0.96, None, Some(1.0)),
            (0.95, None, Some(1.0)),
            // linearly rescaled from 0.05..=0.95 to 0.0..=1.0
            (0.9499, None, Some(0.9998889)),
            (0.84, None, Some(0.87777776)),
            (0.43, None, Some(0.42222223)),
            (0.05001, None, Some(0.000011109644)),
            // clamped
            (0.05, None, Some(0.0)),
            (0.04, None, Some(0.0)),
            (0.01, None, Some(0.0)),
            (0.0, None, Some(0.0)),
        ];

        for (new_raw_value, old_raw_value, expected) in cases {
            let settings = ButtonAxisSettings::default();
            test_button_axis_settings_filter(settings, new_raw_value, old_raw_value, expected);
        }
    }

    #[test]
    fn test_button_axis_settings_default_filter_with_old_raw_value() {
        let cases = [
            // 0.43 gets rescaled to 0.42222223 (0.05..=0.95 -> 0.0..=1.0)
            (0.43, Some(0.44001), Some(0.42222223)),
            (0.43, Some(0.44), None),
            (0.43, Some(0.43), None),
            (0.43, Some(0.41999), Some(0.42222223)),
            (0.43, Some(0.17), Some(0.42222223)),
            (0.43, Some(0.84), Some(0.42222223)),
            (0.05, Some(0.055), Some(0.0)),
            (0.95, Some(0.945), Some(1.0)),
        ];

        for (new_raw_value, old_raw_value, expected) in cases {
            let settings = ButtonAxisSettings::default();
            test_button_axis_settings_filter(settings, new_raw_value, old_raw_value, expected);
        }
    }

    fn test_axis_settings_filter(
        settings: AxisSettings,
        new_raw_value: f32,
        old_raw_value: Option<f32>,
        expected: Option<f32>,
    ) {
        let actual = settings.filter(new_raw_value, old_raw_value);
        assert_eq!(
            expected, actual.map(|f| f.scaled.to_f32()),
            "Testing filtering for {settings:?} with new_raw_value = {new_raw_value:?}, old_raw_value = {old_raw_value:?}",
        );
    }

    #[test]
    fn test_axis_settings_default_filter() {
        // new (raw), expected (rescaled linearly)
        let cases = [
            // high enough to round to 1.0
            (1.0, Some(1.0)),
            (0.99, Some(1.0)),
            (0.96, Some(1.0)),
            (0.95, Some(1.0)),
            // for the following, remember that 0.05 is the "low" value and 0.95 is the "high" value
            // barely below the high value means barely below 1 after scaling
            (0.9499, Some(0.9998889)), // scaled as: (0.9499 - 0.05) / (0.95 - 0.05)
            (0.84, Some(0.87777776)),  // scaled as: (0.84 - 0.05) / (0.95 - 0.05)
            (0.43, Some(0.42222223)),  // scaled as: (0.43 - 0.05) / (0.95 - 0.05)
            // barely above the low value means barely above 0 after scaling
            (0.05001, Some(0.000011109644)), // scaled as: (0.05001 - 0.05) / (0.95 - 0.05)
            // low enough to be rounded to 0 (dead zone)
            (0.05, Some(0.0)),
            (0.04, Some(0.0)),
            (0.01, Some(0.0)),
            (0.0, Some(0.0)),
            // same exact tests as above, but below 0 (bottom half of the dead zone and live zone)
            // low enough to be rounded to -1
            (-1.0, Some(-1.0)),
            (-0.99, Some(-1.0)),
            (-0.96, Some(-1.0)),
            (-0.95, Some(-1.0)),
            // scaled inputs
            (-0.9499, Some(-0.9998889)), // scaled as: (-0.9499 - -0.05) / (-0.95 - -0.05)
            (-0.84, Some(-0.87777776)),  // scaled as: (-0.84 - -0.05) / (-0.95 - -0.05)
            (-0.43, Some(-0.42222226)),  // scaled as: (-0.43 - -0.05) / (-0.95 - -0.05)
            (-0.05001, Some(-0.000011146069)), // scaled as: (-0.05001 - -0.05) / (-0.95 - -0.05)
            // high enough to be rounded to 0 (dead zone)
            (-0.05, Some(0.0)),
            (-0.04, Some(0.0)),
            (-0.01, Some(0.0)),
        ];

        for (new_raw_value, expected) in cases {
            let settings = AxisSettings::new(-0.95, -0.05, 0.05, 0.95, 0.01).unwrap();
            test_axis_settings_filter(settings, new_raw_value, None, expected);
        }
    }

    #[test]
    fn test_axis_settings_default_filter_with_old_raw_values() {
        let threshold = 0.01;
        // expected values are hardcoded to be rescaled to from 0.05..=0.95 to 0.0..=1.0
        // new (raw), old (raw), expected
        let cases = [
            // enough increase to change
            (0.43, Some(0.43 + threshold * 1.1), Some(0.42222223)),
            // enough decrease to change
            (0.43, Some(0.43 - threshold * 1.1), Some(0.42222223)),
            // not enough increase to change
            (0.43, Some(0.43 + threshold * 0.9), None),
            // not enough decrease to change
            (0.43, Some(0.43 - threshold * 0.9), None),
            // enough increase to change
            (-0.43, Some(-0.43 + threshold * 1.1), Some(-0.42222226)),
            // enough decrease to change
            (-0.43, Some(-0.43 - threshold * 1.1), Some(-0.42222226)),
            // not enough increase to change
            (-0.43, Some(-0.43 + threshold * 0.9), None),
            // not enough decrease to change
            (-0.43, Some(-0.43 - threshold * 0.9), None),
            // test upper deadzone logic
            (0.05, Some(0.0), None),
            (0.06, Some(0.0), Some(0.0111111095)),
            // test lower deadzone logic
            (-0.05, Some(0.0), None),
            (-0.06, Some(0.0), Some(-0.011111081)),
            // test upper livezone logic
            (0.95, Some(1.0), None),
            (0.94, Some(1.0), Some(0.9888889)),
            // test lower livezone logic
            (-0.95, Some(-1.0), None),
            (-0.94, Some(-1.0), Some(-0.9888889)),
        ];

        for (new_raw_value, old_raw_value, expected) in cases {
            let settings = AxisSettings::new(-0.95, -0.05, 0.05, 0.95, threshold).unwrap();
            test_axis_settings_filter(settings, new_raw_value, old_raw_value, expected);
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

    struct TestContext {
        pub app: App,
    }

    impl TestContext {
        pub fn new() -> Self {
            let mut app = App::new();
            app.add_systems(
                PreUpdate,
                (
                    gamepad_connection_system,
                    gamepad_event_processing_system.after(gamepad_connection_system),
                ),
            )
            .add_event::<GamepadEvent>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<RawGamepadButtonChangedEvent>()
            .add_event::<GamepadButtonChangedEvent>()
            .add_event::<GamepadButtonStateChangedEvent>()
            .add_event::<GamepadAxisChangedEvent>()
            .add_event::<RawGamepadAxisChangedEvent>()
            .add_event::<RawGamepadEvent>();
            Self { app }
        }

        pub fn update(&mut self) {
            self.app.update();
        }

        pub fn send_gamepad_connection_event(&mut self, gamepad: Option<Entity>) -> Entity {
            let gamepad = gamepad.unwrap_or_else(|| self.app.world_mut().spawn_empty().id());
            self.app
                .world_mut()
                .resource_mut::<Events<GamepadConnectionEvent>>()
                .write(GamepadConnectionEvent::new(
                    gamepad,
                    Connected {
                        name: "Test gamepad".to_string(),
                        vendor_id: None,
                        product_id: None,
                    },
                ));
            gamepad
        }

        pub fn send_gamepad_disconnection_event(&mut self, gamepad: Entity) {
            self.app
                .world_mut()
                .resource_mut::<Events<GamepadConnectionEvent>>()
                .write(GamepadConnectionEvent::new(gamepad, Disconnected));
        }

        pub fn send_raw_gamepad_event(&mut self, event: RawGamepadEvent) {
            self.app
                .world_mut()
                .resource_mut::<Events<RawGamepadEvent>>()
                .write(event);
        }

        pub fn send_raw_gamepad_event_batch(
            &mut self,
            events: impl IntoIterator<Item = RawGamepadEvent>,
        ) {
            self.app
                .world_mut()
                .resource_mut::<Events<RawGamepadEvent>>()
                .write_batch(events);
        }
    }

    #[test]
    fn connection_event() {
        let mut ctx = TestContext::new();
        assert_eq!(
            ctx.app
                .world_mut()
                .query::<&Gamepad>()
                .iter(ctx.app.world())
                .len(),
            0
        );
        ctx.send_gamepad_connection_event(None);
        ctx.update();
        assert_eq!(
            ctx.app
                .world_mut()
                .query::<(&Gamepad, &GamepadSettings)>()
                .iter(ctx.app.world())
                .len(),
            1
        );
    }

    #[test]
    fn disconnection_event() {
        let mut ctx = TestContext::new();
        assert_eq!(
            ctx.app
                .world_mut()
                .query::<&Gamepad>()
                .iter(ctx.app.world())
                .len(),
            0
        );
        let entity = ctx.send_gamepad_connection_event(None);
        ctx.update();
        assert_eq!(
            ctx.app
                .world_mut()
                .query::<(&Gamepad, &GamepadSettings)>()
                .iter(ctx.app.world())
                .len(),
            1
        );
        ctx.send_gamepad_disconnection_event(entity);
        ctx.update();
        // Gamepad component should be removed
        assert!(ctx
            .app
            .world_mut()
            .query::<&Gamepad>()
            .get(ctx.app.world(), entity)
            .is_err());
        // Settings should be kept
        assert!(ctx
            .app
            .world_mut()
            .query::<&GamepadSettings>()
            .get(ctx.app.world(), entity)
            .is_ok());

        // Mistakenly sending a second disconnection event shouldn't break anything
        ctx.send_gamepad_disconnection_event(entity);
        ctx.update();
        assert!(ctx
            .app
            .world_mut()
            .query::<&Gamepad>()
            .get(ctx.app.world(), entity)
            .is_err());
        assert!(ctx
            .app
            .world_mut()
            .query::<&GamepadSettings>()
            .get(ctx.app.world(), entity)
            .is_ok());
    }

    #[test]
    fn connection_disconnection_frame_event() {
        let mut ctx = TestContext::new();
        assert_eq!(
            ctx.app
                .world_mut()
                .query::<&Gamepad>()
                .iter(ctx.app.world())
                .len(),
            0
        );
        let entity = ctx.send_gamepad_connection_event(None);
        ctx.send_gamepad_disconnection_event(entity);
        ctx.update();
        // Gamepad component should be removed
        assert!(ctx
            .app
            .world_mut()
            .query::<&Gamepad>()
            .get(ctx.app.world(), entity)
            .is_err());
        // Settings should be kept
        assert!(ctx
            .app
            .world_mut()
            .query::<&GamepadSettings>()
            .get(ctx.app.world(), entity)
            .is_ok());
    }

    #[test]
    fn reconnection_event() {
        let button_settings = ButtonSettings::new(0.7, 0.2).expect("correct parameters");
        let mut ctx = TestContext::new();
        assert_eq!(
            ctx.app
                .world_mut()
                .query::<&Gamepad>()
                .iter(ctx.app.world())
                .len(),
            0
        );
        let entity = ctx.send_gamepad_connection_event(None);
        ctx.update();
        let mut settings = ctx
            .app
            .world_mut()
            .query::<&mut GamepadSettings>()
            .get_mut(ctx.app.world_mut(), entity)
            .expect("be alive");
        assert_ne!(settings.default_button_settings, button_settings);
        settings.default_button_settings = button_settings.clone();
        ctx.send_gamepad_disconnection_event(entity);
        ctx.update();
        assert_eq!(
            ctx.app
                .world_mut()
                .query::<&Gamepad>()
                .iter(ctx.app.world())
                .len(),
            0
        );
        ctx.send_gamepad_connection_event(Some(entity));
        ctx.update();
        let settings = ctx
            .app
            .world_mut()
            .query::<&GamepadSettings>()
            .get(ctx.app.world(), entity)
            .expect("be alive");
        assert_eq!(settings.default_button_settings, button_settings);
    }

    #[test]
    fn reconnection_same_frame_event() {
        let mut ctx = TestContext::new();
        assert_eq!(
            ctx.app
                .world_mut()
                .query::<&Gamepad>()
                .iter(ctx.app.world())
                .len(),
            0
        );
        let entity = ctx.send_gamepad_connection_event(None);
        ctx.send_gamepad_disconnection_event(entity);
        ctx.update();
        assert_eq!(
            ctx.app
                .world_mut()
                .query::<&Gamepad>()
                .iter(ctx.app.world())
                .len(),
            0
        );
        assert!(ctx
            .app
            .world_mut()
            .query::<(Entity, &GamepadSettings)>()
            .get(ctx.app.world(), entity)
            .is_ok());
    }

    #[test]
    fn gamepad_axis_valid() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        ctx.app
            .world_mut()
            .resource_mut::<Events<RawGamepadEvent>>()
            .write_batch([
                RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                    entity,
                    GamepadAxis::LeftStickY,
                    0.5,
                )),
                RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                    entity,
                    GamepadAxis::RightStickX,
                    0.6,
                )),
                RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                    entity,
                    GamepadAxis::RightZ,
                    -0.4,
                )),
                RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                    entity,
                    GamepadAxis::RightStickY,
                    -0.8,
                )),
            ]);
        ctx.update();
        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadAxisChangedEvent>>()
                .len(),
            4
        );
    }

    #[test]
    fn gamepad_axis_threshold_filter() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        let settings = GamepadSettings::default().default_axis_settings;
        // Set of events to ensure they are being properly filtered
        let base_value = 0.5;
        let events = [
            // Event above threshold
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                base_value,
            )),
            // Event below threshold, should be filtered
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                base_value + settings.threshold - 0.01,
            )),
            // Event above threshold
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                base_value + settings.threshold + 0.01,
            )),
        ];
        ctx.app
            .world_mut()
            .resource_mut::<Events<RawGamepadEvent>>()
            .write_batch(events);
        ctx.update();
        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadAxisChangedEvent>>()
                .len(),
            2
        );
    }

    #[test]
    fn gamepad_axis_deadzone_filter() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        let settings = GamepadSettings::default().default_axis_settings;

        // Set of events to ensure they are being properly filtered
        let events = [
            // Event below deadzone upperbound should be filtered
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                settings.deadzone_upperbound - 0.01,
            )),
            // Event above deadzone lowerbound should be filtered
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                settings.deadzone_lowerbound + 0.01,
            )),
        ];
        ctx.app
            .world_mut()
            .resource_mut::<Events<RawGamepadEvent>>()
            .write_batch(events);
        ctx.update();
        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadAxisChangedEvent>>()
                .len(),
            0
        );
    }

    #[test]
    fn gamepad_axis_deadzone_rounded() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        let settings = GamepadSettings::default().default_axis_settings;

        // Set of events to ensure they are being properly filtered
        let events = [
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                1.0,
            )),
            // Event below deadzone upperbound should be rounded to 0
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                settings.deadzone_upperbound - 0.01,
            )),
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                1.0,
            )),
            // Event above deadzone lowerbound should be rounded to 0
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                settings.deadzone_lowerbound + 0.01,
            )),
        ];
        let results = [1.0, 0.0, 1.0, 0.0];
        ctx.app
            .world_mut()
            .resource_mut::<Events<RawGamepadEvent>>()
            .write_batch(events);
        ctx.update();

        let events = ctx
            .app
            .world()
            .resource::<Events<GamepadAxisChangedEvent>>();
        let mut event_reader = events.get_cursor();
        for (event, result) in event_reader.read(events).zip(results) {
            assert_eq!(event.value, result);
        }
        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadAxisChangedEvent>>()
                .len(),
            4
        );
    }

    #[test]
    fn gamepad_axis_livezone_filter() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        let settings = GamepadSettings::default().default_axis_settings;

        // Set of events to ensure they are being properly filtered
        let events = [
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                1.0,
            )),
            // Event above livezone upperbound should be filtered
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                settings.livezone_upperbound + 0.01,
            )),
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                -1.0,
            )),
            // Event below livezone lowerbound should be filtered
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                settings.livezone_lowerbound - 0.01,
            )),
        ];
        ctx.app
            .world_mut()
            .resource_mut::<Events<RawGamepadEvent>>()
            .write_batch(events);
        ctx.update();
        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadAxisChangedEvent>>()
                .len(),
            2
        );
    }

    #[test]
    fn gamepad_axis_livezone_rounded() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        let settings = GamepadSettings::default().default_axis_settings;

        // Set of events to ensure they are being properly filtered
        let events = [
            // Event above livezone upperbound should be rounded to 1
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                settings.livezone_upperbound + 0.01,
            )),
            // Event below livezone lowerbound should be rounded to -1
            RawGamepadEvent::Axis(RawGamepadAxisChangedEvent::new(
                entity,
                GamepadAxis::LeftStickX,
                settings.livezone_lowerbound - 0.01,
            )),
        ];
        let results = [1.0, -1.0];
        ctx.app
            .world_mut()
            .resource_mut::<Events<RawGamepadEvent>>()
            .write_batch(events);
        ctx.update();

        let events = ctx
            .app
            .world()
            .resource::<Events<GamepadAxisChangedEvent>>();
        let mut event_reader = events.get_cursor();
        for (event, result) in event_reader.read(events).zip(results) {
            assert_eq!(event.value, result);
        }
        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadAxisChangedEvent>>()
                .len(),
            2
        );
    }

    #[test]
    fn gamepad_buttons_pressed() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        let digital_settings = GamepadSettings::default().default_button_settings;

        let events = [RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            entity,
            GamepadButton::DPadDown,
            digital_settings.press_threshold,
        ))];
        ctx.app
            .world_mut()
            .resource_mut::<Events<RawGamepadEvent>>()
            .write_batch(events);
        ctx.update();

        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadButtonStateChangedEvent>>()
                .len(),
            1
        );
        let events = ctx
            .app
            .world()
            .resource::<Events<GamepadButtonStateChangedEvent>>();
        let mut event_reader = events.get_cursor();
        for event in event_reader.read(events) {
            assert_eq!(event.button, GamepadButton::DPadDown);
            assert_eq!(event.state, ButtonState::Pressed);
        }
        assert!(ctx
            .app
            .world_mut()
            .query::<&Gamepad>()
            .get(ctx.app.world(), entity)
            .unwrap()
            .pressed(GamepadButton::DPadDown));

        ctx.app
            .world_mut()
            .resource_mut::<Events<GamepadButtonStateChangedEvent>>()
            .clear();
        ctx.update();

        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadButtonStateChangedEvent>>()
                .len(),
            0
        );
        assert!(ctx
            .app
            .world_mut()
            .query::<&Gamepad>()
            .get(ctx.app.world(), entity)
            .unwrap()
            .pressed(GamepadButton::DPadDown));
    }

    #[test]
    fn gamepad_buttons_just_pressed() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        let digital_settings = GamepadSettings::default().default_button_settings;

        ctx.send_raw_gamepad_event(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            entity,
            GamepadButton::DPadDown,
            digital_settings.press_threshold,
        )));
        ctx.update();

        // Check it is flagged for this frame
        assert!(ctx
            .app
            .world_mut()
            .query::<&Gamepad>()
            .get(ctx.app.world(), entity)
            .unwrap()
            .just_pressed(GamepadButton::DPadDown));
        ctx.update();

        //Check it clears next frame
        assert!(!ctx
            .app
            .world_mut()
            .query::<&Gamepad>()
            .get(ctx.app.world(), entity)
            .unwrap()
            .just_pressed(GamepadButton::DPadDown));
    }
    #[test]
    fn gamepad_buttons_released() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        let digital_settings = GamepadSettings::default().default_button_settings;

        ctx.send_raw_gamepad_event(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            entity,
            GamepadButton::DPadDown,
            digital_settings.press_threshold,
        )));
        ctx.update();

        ctx.app
            .world_mut()
            .resource_mut::<Events<GamepadButtonStateChangedEvent>>()
            .clear();
        ctx.send_raw_gamepad_event(RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
            entity,
            GamepadButton::DPadDown,
            digital_settings.release_threshold - 0.01,
        )));
        ctx.update();
        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadButtonStateChangedEvent>>()
                .len(),
            1
        );
        let events = ctx
            .app
            .world()
            .resource::<Events<GamepadButtonStateChangedEvent>>();
        let mut event_reader = events.get_cursor();
        for event in event_reader.read(events) {
            assert_eq!(event.button, GamepadButton::DPadDown);
            assert_eq!(event.state, ButtonState::Released);
        }
        assert!(!ctx
            .app
            .world_mut()
            .query::<&Gamepad>()
            .get(ctx.app.world(), entity)
            .unwrap()
            .pressed(GamepadButton::DPadDown));
        ctx.app
            .world_mut()
            .resource_mut::<Events<GamepadButtonStateChangedEvent>>()
            .clear();
        ctx.update();

        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadButtonStateChangedEvent>>()
                .len(),
            0
        );
    }

    #[test]
    fn gamepad_buttons_just_released() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        let digital_settings = GamepadSettings::default().default_button_settings;

        ctx.send_raw_gamepad_event_batch([
            RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                entity,
                GamepadButton::DPadDown,
                digital_settings.press_threshold,
            )),
            RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                entity,
                GamepadButton::DPadDown,
                digital_settings.release_threshold - 0.01,
            )),
        ]);
        ctx.update();

        // Check it is flagged for this frame
        assert!(ctx
            .app
            .world_mut()
            .query::<&Gamepad>()
            .get(ctx.app.world(), entity)
            .unwrap()
            .just_released(GamepadButton::DPadDown));
        ctx.update();

        //Check it clears next frame
        assert!(!ctx
            .app
            .world_mut()
            .query::<&Gamepad>()
            .get(ctx.app.world(), entity)
            .unwrap()
            .just_released(GamepadButton::DPadDown));
    }

    #[test]
    fn gamepad_buttons_axis() {
        let mut ctx = TestContext::new();

        // Create test gamepad
        let entity = ctx.send_gamepad_connection_event(None);
        let digital_settings = GamepadSettings::default().default_button_settings;
        let analog_settings = GamepadSettings::default().default_button_axis_settings;

        // Test events
        let events = [
            // Should trigger event
            RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                entity,
                GamepadButton::DPadDown,
                digital_settings.press_threshold,
            )),
            // Should trigger event
            RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                entity,
                GamepadButton::DPadDown,
                digital_settings.release_threshold,
            )),
            // Shouldn't trigger a state changed event
            RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                entity,
                GamepadButton::DPadDown,
                digital_settings.release_threshold - analog_settings.threshold * 1.01,
            )),
            // Shouldn't trigger any event
            RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                entity,
                GamepadButton::DPadDown,
                digital_settings.release_threshold - (analog_settings.threshold * 1.5),
            )),
            // Shouldn't trigger a state changed event
            RawGamepadEvent::Button(RawGamepadButtonChangedEvent::new(
                entity,
                GamepadButton::DPadDown,
                digital_settings.release_threshold - (analog_settings.threshold * 2.02),
            )),
        ];
        ctx.send_raw_gamepad_event_batch(events);
        ctx.update();
        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadButtonStateChangedEvent>>()
                .len(),
            2
        );
        assert_eq!(
            ctx.app
                .world()
                .resource::<Events<GamepadButtonChangedEvent>>()
                .len(),
            4
        );
    }
}
