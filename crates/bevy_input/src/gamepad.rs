//! The gamepad input functionality.

use crate::{Axis, ButtonInput, ButtonState};
use bevy_ecs::event::{Event, EventReader, EventWriter};
use bevy_ecs::{
    change_detection::DetectChangesMut,
    system::{Res, ResMut, Resource},
};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_utils::Duration;
use bevy_utils::{tracing::info, HashMap};
use thiserror::Error;

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

#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A gamepad with an associated `ID`.
///
/// ## Usage
///
/// The primary way to access the individual connected gamepads is done through the [`Gamepads`]
/// `bevy` resource. It is also used inside of [`GamepadConnectionEvent`]s to correspond a gamepad
/// with a connection event.
///
/// ## Note
///
/// The `ID` of a gamepad is fixed until the gamepad disconnects or the app is restarted.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct Gamepad {
    /// The `ID` of the gamepad.
    pub id: usize,
}

impl Gamepad {
    /// Creates a new [`Gamepad`].
    pub fn new(id: usize) -> Self {
        Self { id }
    }
}

/// Metadata associated with a [`Gamepad`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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

/// A collection of connected [`Gamepad`]s.
///
/// ## Usage
///
/// It is stored in a `bevy` resource which tracks all of the currently connected [`Gamepad`]s.
///
/// ## Updating
///
/// The [`Gamepad`]s are registered and deregistered in the [`gamepad_connection_system`]
/// whenever a [`GamepadConnectionEvent`] is received.
#[derive(Resource, Default, Debug)]
pub struct Gamepads {
    /// The collection of the connected [`Gamepad`]s.
    gamepads: HashMap<Gamepad, GamepadInfo>,
}

impl Gamepads {
    /// Returns `true` if the `gamepad` is connected.
    pub fn contains(&self, gamepad: Gamepad) -> bool {
        self.gamepads.contains_key(&gamepad)
    }

    /// Returns an iterator over registered [`Gamepad`]s in an arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = Gamepad> + '_ {
        self.gamepads.keys().copied()
    }

    /// The name of the gamepad if this one is connected.
    pub fn name(&self, gamepad: Gamepad) -> Option<&str> {
        self.gamepads.get(&gamepad).map(|g| g.name.as_str())
    }

    /// Registers the `gamepad`, marking it as connected.
    fn register(&mut self, gamepad: Gamepad, info: GamepadInfo) {
        self.gamepads.insert(gamepad, info);
    }

    /// Deregisters the `gamepad`, marking it as disconnected.
    fn deregister(&mut self, gamepad: Gamepad) {
        self.gamepads.remove(&gamepad);
    }
}

/// A type of a [`GamepadButton`].
///
/// ## Usage
///
/// This is used to determine which button has changed its value when receiving a
/// [`GamepadButtonChangedEvent`]. It is also used in the [`GamepadButton`]
/// which in turn is used to create the [`ButtonInput<GamepadButton>`] or
/// [`Axis<GamepadButton>`] `bevy` resources.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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

/// A button of a [`Gamepad`].
///
/// ## Usage
///
/// It is used as the generic `T` value of an [`ButtonInput`] and [`Axis`] to create `bevy` resources. These
/// resources store the data of the buttons of a gamepad and can be accessed inside of a system.
///
/// ## Updating
///
/// The gamepad button resources are updated inside of the [`gamepad_button_event_system`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadButton {
    /// The gamepad on which the button is located on.
    pub gamepad: Gamepad,
    /// The type of the button.
    pub button_type: GamepadButtonType,
}

impl GamepadButton {
    /// Creates a new [`GamepadButton`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadButton, GamepadButtonType, Gamepad};
    /// #
    /// let gamepad_button = GamepadButton::new(
    ///     Gamepad::new(1),
    ///     GamepadButtonType::South,
    /// );
    /// ```
    pub fn new(gamepad: Gamepad, button_type: GamepadButtonType) -> Self {
        Self {
            gamepad,
            button_type,
        }
    }
}

/// A gamepad button input event.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadButtonInput {
    /// The gamepad button assigned to the event.
    pub button: GamepadButton,
    /// The pressed state of the button.
    pub state: ButtonState,
}

/// A type of a [`GamepadAxis`].
///
/// ## Usage
///
/// This is used to determine which axis has changed its value when receiving a
/// [`GamepadAxisChangedEvent`]. It is also used in the [`GamepadAxis`]
/// which in turn is used to create the [`Axis<GamepadAxis>`] `bevy` resource.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
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

/// An axis of a [`Gamepad`].
///
/// ## Usage
///
/// It is used as the generic `T` value of an [`Axis`] to create `bevy` resources. These
/// resources store the data of the axes of a gamepad and can be accessed inside of a system.
///
/// ## Updating
///
/// The gamepad axes resources are updated inside of the [`gamepad_axis_event_system`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadAxis {
    /// The gamepad on which the axis is located on.
    pub gamepad: Gamepad,
    /// The type of the axis.
    pub axis_type: GamepadAxisType,
}

impl GamepadAxis {
    /// Creates a new [`GamepadAxis`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadAxis, GamepadAxisType, Gamepad};
    /// #
    /// let gamepad_axis = GamepadAxis::new(
    ///     Gamepad::new(1),
    ///     GamepadAxisType::LeftStickX,
    /// );
    /// ```
    pub fn new(gamepad: Gamepad, axis_type: GamepadAxisType) -> Self {
        Self { gamepad, axis_type }
    }
}

/// Settings for all [`Gamepad`]s.
///
/// ## Usage
///
/// It is used to create a `bevy` resource that stores the settings of every [`GamepadButton`] and
/// [`GamepadAxis`]. If no user defined [`ButtonSettings`], [`AxisSettings`], or [`ButtonAxisSettings`]
/// are defined, the default settings of each are used as a fallback accordingly.
///
/// ## Note
///
/// The [`GamepadSettings`] are used inside of `bevy_gilrs` to determine when raw gamepad events from `gilrs`,
/// should register as a [`GamepadEvent`]. Events that don't meet the change thresholds defined in [`GamepadSettings`]
/// will not register. To modify these settings, mutate the corresponding resource.
#[derive(Resource, Default, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Default))]

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
    /// Returns the [`ButtonSettings`] of the `button`.
    ///
    /// If no user defined [`ButtonSettings`] are specified the default [`ButtonSettings`] get returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::gamepad::{GamepadSettings, GamepadButton, Gamepad, GamepadButtonType};
    /// #
    /// # let settings = GamepadSettings::default();
    /// let button = GamepadButton::new(Gamepad::new(1), GamepadButtonType::South);
    /// let button_settings = settings.get_button_settings(button);
    /// ```
    pub fn get_button_settings(&self, button: GamepadButton) -> &ButtonSettings {
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
    /// # use bevy_input::gamepad::{GamepadSettings, GamepadAxis, Gamepad, GamepadAxisType};
    /// #
    /// # let settings = GamepadSettings::default();
    /// let axis = GamepadAxis::new(Gamepad::new(1), GamepadAxisType::LeftStickX);
    /// let axis_settings = settings.get_axis_settings(axis);
    /// ```
    pub fn get_axis_settings(&self, axis: GamepadAxis) -> &AxisSettings {
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
    /// # use bevy_input::gamepad::{GamepadSettings, GamepadButton, Gamepad, GamepadButtonType};
    /// #
    /// # let settings = GamepadSettings::default();
    /// let button = GamepadButton::new(Gamepad::new(1), GamepadButtonType::South);
    /// let button_axis_settings = settings.get_button_axis_settings(button);
    /// ```
    pub fn get_button_axis_settings(&self, button: GamepadButton) -> &ButtonAxisSettings {
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
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Default))]
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
/// It is used inside of the [`GamepadSettings`] to define the sensitivity range and
/// threshold for an axis.
/// Values that are higher than `livezone_upperbound` will be rounded up to 1.0.
/// Values that are lower than `livezone_lowerbound` will be rounded down to -1.0.
/// Values that are in-between `deadzone_lowerbound` and `deadzone_upperbound` will be rounded
/// to 0.0.
/// Otherwise, values will not be rounded.
///
/// The valid range is `[-1.0, 1.0]`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Default))]
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

/// Settings for a [`GamepadButton`].
///
/// It is used inside of the [`GamepadSettings`] to define the sensitivity range and
/// threshold for a button axis.
///
/// ## Logic
///
/// - Values that are higher than or equal to `high` will be rounded to 1.0.
/// - Values that are lower than or equal to `low` will be rounded to 0.0.
/// - Otherwise, values will not be rounded.
///
/// The valid range is from 0.0 to 1.0, inclusive.
///
/// ## Updating
///
/// The current value of a button is received through the [`GamepadButtonChangedEvent`].
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Default))]
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

/// Handles [`GamepadConnectionEvent`]s and updates gamepad resources.
///
/// Updates the [`Gamepads`] resource and resets and/or initializes
/// the [`Axis<GamepadButton>`] and [`ButtonInput<GamepadButton>`] resources.
///
/// ## Note
///
/// Whenever a [`Gamepad`] connects or disconnects, an information gets printed to the console using the [`info!`] macro.
pub fn gamepad_connection_system(
    mut gamepads: ResMut<Gamepads>,
    mut connection_events: EventReader<GamepadConnectionEvent>,
    mut axis: ResMut<Axis<GamepadAxis>>,
    mut button_axis: ResMut<Axis<GamepadButton>>,
    mut button_input: ResMut<ButtonInput<GamepadButton>>,
) {
    for connection_event in connection_events.read() {
        let gamepad = connection_event.gamepad;

        if let GamepadConnection::Connected(info) = &connection_event.connection {
            gamepads.register(gamepad, info.clone());
            info!("{:?} Connected", gamepad);

            for button_type in &ALL_BUTTON_TYPES {
                let gamepad_button = GamepadButton::new(gamepad, *button_type);
                button_input.reset(gamepad_button);
                button_axis.set(gamepad_button, 0.0);
            }
            for axis_type in &ALL_AXIS_TYPES {
                axis.set(GamepadAxis::new(gamepad, *axis_type), 0.0);
            }
        } else {
            gamepads.deregister(gamepad);
            info!("{:?} Disconnected", gamepad);

            for button_type in &ALL_BUTTON_TYPES {
                let gamepad_button = GamepadButton::new(gamepad, *button_type);
                button_input.reset(gamepad_button);
                button_axis.remove(gamepad_button);
            }
            for axis_type in &ALL_AXIS_TYPES {
                axis.remove(GamepadAxis::new(gamepad, *axis_type));
            }
        }
    }
}

/// The connection status of a gamepad.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum GamepadConnection {
    /// The gamepad is connected.
    Connected(GamepadInfo),
    /// The gamepad is disconnected.
    Disconnected,
}

/// A Gamepad connection event. Created when a connection to a gamepad
/// is established and when a gamepad is disconnected.
#[derive(Event, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadConnectionEvent {
    /// The gamepad whose connection status changed.
    pub gamepad: Gamepad,
    /// The change in the gamepads connection.
    pub connection: GamepadConnection,
}

impl GamepadConnectionEvent {
    /// Creates a [`GamepadConnectionEvent`].
    pub fn new(gamepad: Gamepad, connection: GamepadConnection) -> Self {
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

/// Gamepad event for when the "value" on the axis changes
/// by an amount larger than the threshold defined in [`GamepadSettings`].
#[derive(Event, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadAxisChangedEvent {
    /// The gamepad on which the axis is triggered.
    pub gamepad: Gamepad,
    /// The type of the triggered axis.
    pub axis_type: GamepadAxisType,
    /// The value of the axis.
    pub value: f32,
}

impl GamepadAxisChangedEvent {
    /// Creates a [`GamepadAxisChangedEvent`].
    pub fn new(gamepad: Gamepad, axis_type: GamepadAxisType, value: f32) -> Self {
        Self {
            gamepad,
            axis_type,
            value,
        }
    }
}

/// Gamepad event for when the "value" (amount of pressure) on the button
/// changes by an amount larger than the threshold defined in [`GamepadSettings`].
#[derive(Event, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct GamepadButtonChangedEvent {
    /// The gamepad on which the button is triggered.
    pub gamepad: Gamepad,
    /// The type of the triggered button.
    pub button_type: GamepadButtonType,
    /// The value of the button.
    pub value: f32,
}

impl GamepadButtonChangedEvent {
    /// Creates a [`GamepadButtonChangedEvent`].
    pub fn new(gamepad: Gamepad, button_type: GamepadButtonType, value: f32) -> Self {
        Self {
            gamepad,
            button_type,
            value,
        }
    }
}

/// Uses [`GamepadAxisChangedEvent`]s to update the relevant [`ButtonInput`] and [`Axis`] values.
pub fn gamepad_axis_event_system(
    mut gamepad_axis: ResMut<Axis<GamepadAxis>>,
    mut axis_events: EventReader<GamepadAxisChangedEvent>,
) {
    for axis_event in axis_events.read() {
        let axis = GamepadAxis::new(axis_event.gamepad, axis_event.axis_type);
        gamepad_axis.set(axis, axis_event.value);
    }
}

/// Uses [`GamepadButtonChangedEvent`]s to update the relevant [`ButtonInput`] and [`Axis`] values.
pub fn gamepad_button_event_system(
    mut button_changed_events: EventReader<GamepadButtonChangedEvent>,
    mut button_input: ResMut<ButtonInput<GamepadButton>>,
    mut button_input_events: EventWriter<GamepadButtonInput>,
    settings: Res<GamepadSettings>,
) {
    for button_event in button_changed_events.read() {
        let button = GamepadButton::new(button_event.gamepad, button_event.button_type);
        let value = button_event.value;
        let button_property = settings.get_button_settings(button);

        if button_property.is_released(value) {
            // Check if button was previously pressed
            if button_input.pressed(button) {
                button_input_events.send(GamepadButtonInput {
                    button,
                    state: ButtonState::Released,
                });
            }
            // We don't have to check if the button was previously pressed here
            // because that check is performed within Input<T>::release()
            button_input.release(button);
        } else if button_property.is_pressed(value) {
            // Check if button was previously not pressed
            if !button_input.pressed(button) {
                button_input_events.send(GamepadButtonInput {
                    button,
                    state: ButtonState::Pressed,
                });
            }
            button_input.press(button);
        };
    }
}

/// A gamepad event.
///
/// This event type is used over the [`GamepadConnectionEvent`],
/// [`GamepadButtonChangedEvent`] and [`GamepadAxisChangedEvent`] when
/// the in-frame relative ordering of events is important.
#[derive(Event, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
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

impl From<GamepadConnectionEvent> for GamepadEvent {
    fn from(value: GamepadConnectionEvent) -> Self {
        Self::Connection(value)
    }
}

impl From<GamepadButtonChangedEvent> for GamepadEvent {
    fn from(value: GamepadButtonChangedEvent) -> Self {
        Self::Button(value)
    }
}

impl From<GamepadAxisChangedEvent> for GamepadEvent {
    fn from(value: GamepadAxisChangedEvent) -> Self {
        Self::Axis(value)
    }
}

/// Splits the [`GamepadEvent`] event stream into its component events.
pub fn gamepad_event_system(
    mut gamepad_events: EventReader<GamepadEvent>,
    mut connection_events: EventWriter<GamepadConnectionEvent>,
    mut button_events: EventWriter<GamepadButtonChangedEvent>,
    mut axis_events: EventWriter<GamepadAxisChangedEvent>,
    mut button_input: ResMut<ButtonInput<GamepadButton>>,
) {
    button_input.bypass_change_detection().clear();
    for gamepad_event in gamepad_events.read() {
        match gamepad_event {
            GamepadEvent::Connection(connection_event) => {
                connection_events.send(connection_event.clone());
            }
            GamepadEvent::Button(button_event) => {
                button_events.send(button_event.clone());
            }
            GamepadEvent::Axis(axis_event) => {
                axis_events.send(axis_event.clone());
            }
        }
    }
}

/// An array of every [`GamepadButtonType`] variant.
const ALL_BUTTON_TYPES: [GamepadButtonType; 19] = [
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
const ALL_AXIS_TYPES: [GamepadAxisType; 6] = [
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

/// An event that controls force-feedback rumbling of a [`Gamepad`].
///
/// # Notes
///
/// Does nothing if the gamepad or platform does not support rumble.
///
/// # Example
///
/// ```
/// # use bevy_input::gamepad::{Gamepad, Gamepads, GamepadRumbleRequest, GamepadRumbleIntensity};
/// # use bevy_ecs::prelude::{EventWriter, Res};
/// # use bevy_utils::Duration;
/// fn rumble_gamepad_system(
///     mut rumble_requests: EventWriter<GamepadRumbleRequest>,
///     gamepads: Res<Gamepads>
/// ) {
///     for gamepad in gamepads.iter() {
///         rumble_requests.send(GamepadRumbleRequest::Add {
///             gamepad,
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
        gamepad: Gamepad,
    },
    /// Stop all running rumbles on the given [`Gamepad`].
    Stop {
        /// The gamepad to stop rumble.
        gamepad: Gamepad,
    },
}

impl GamepadRumbleRequest {
    /// Get the [`Gamepad`] associated with this request.
    pub fn gamepad(&self) -> Gamepad {
        match self {
            Self::Add { gamepad, .. } | Self::Stop { gamepad } => *gamepad,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gamepad::{AxisSettingsError, ButtonSettingsError};

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
}
