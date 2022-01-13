use crate::gamepad::{GamepadAxis, GamepadButton};
use bevy_utils::HashMap;

/// A setting for all [`Gamepad`](crate::gamepad::Gamepad)s.
///
/// ## Usage
///
/// It is used to create a `Bevy` resource that stores the settings of every [`GamepadButton`] and
/// [`GamepadAxis`]. If no user defined [`ButtonSettings`], [`AxisSettings`], or [`ButtonAxisSettings`]
/// are defined, the default settings of each are used as a fallback accordingly.
///
/// ## Access
///
/// To access the resource use one of the following:
/// - Non-mutable access of the gamepad settings: `Res<GamepadSettings>`
/// - Mutable access of the gamepad settings: `ResMut<GamepadSettings>`
///
/// ## Usage
///
/// The [`GamepadSettings`] are used inside of the [`gamepad_event_system`][crate::gamepad::gamepad_event_system],
/// but are never written to inside of `Bevy`. To insert user defined settings it is required to mutably access
/// the resource and insert the settings as needed.
#[derive(Default, Debug)]
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

    /// Returns the [`AxisSettings`] of the [`GamepadAxis`].
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

    /// Returns the [`ButtonAxisSettings`] of the [`GamepadButton`].
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

/// A setting for a [`GamepadButton`].
///
/// ## Usage
///
/// It is used inside of the [`GamepadSettings`] to define the threshold for a gamepad button
/// to be considered pressed or released. A button is considered pressed if the `press`
/// value is surpassed and released if the `release` value is undercut.
///
/// ## Updating
///
/// The current value of a button is received through the [`GamepadEvent`](crate::gamepad::GamepadEvent)s
/// or [`GamepadEventRaw`](crate::gamepad::GamepadEventRaw)s.
#[derive(Debug, Clone)]
pub struct ButtonSettings {
    /// The threshold for the button to be considered as pressed.
    pub press: f32,
    /// The threshold for the button to be considered as released.
    pub release: f32,
}

impl Default for ButtonSettings {
    fn default() -> Self {
        ButtonSettings {
            press: 0.75,
            release: 0.65,
        }
    }
}

impl ButtonSettings {
    /// Returns `true` if the button is pressed.
    ///
    /// A button is considered pressed if the `value` passed is greater than or equal to the `press` threshold.
    pub(crate) fn is_pressed(&self, value: f32) -> bool {
        value >= self.press
    }

    /// Returns `true` if the button is released.
    ///
    /// A button is considered released if the `value` passed is lower than or equal to the `release` threshold.
    pub(crate) fn is_released(&self, value: f32) -> bool {
        value <= self.release
    }
}

/// A setting for a [`GamepadAxis`].
///
/// It is used inside of the [`GamepadSettings`] to define the sensitivity range and
/// threshold for an axis.
///
/// ## Logic
///
/// - Values that are in-between `negative_low` and `positive_low` will be rounded to 0.0.
/// - Values that are higher than or equal to `positive_high` will be rounded to 1.0.
/// - Values that are lower than or equal to `negative_high` will be rounded to -1.0.
/// - Otherwise, values will not be rounded.
///
/// The valid range is from -1.0 to 1.0, inclusive.
///
/// ## Updating
///
/// The current value of an axis is received through the [`GamepadEvent`](crate::gamepad::GamepadEvent)s
/// or [`GamepadEventRaw`](crate::gamepad::GamepadEventRaw)s.
#[derive(Debug, Clone)]
pub struct AxisSettings {
    /// The positive high value at which to apply rounding.
    pub positive_high: f32,
    /// The positive low value at which to apply rounding.
    pub positive_low: f32,
    /// The negative high value at which to apply rounding.
    pub negative_high: f32,
    /// The negative low value at which to apply rounding.
    pub negative_low: f32,
    /// The threshold defining the minimum difference between the old and new values to apply the changes.
    pub threshold: f32,
}

impl Default for AxisSettings {
    fn default() -> Self {
        AxisSettings {
            positive_high: 0.95,
            positive_low: 0.05,
            negative_high: -0.95,
            negative_low: -0.05,
            threshold: 0.01,
        }
    }
}

impl AxisSettings {
    /// Filters the `new_value` according to the specified settings.
    ///
    /// If the `new_value` is:
    /// - in-between `negative_low` and `positive_low` it will be rounded to 0.0.
    /// - higher than or equal to `positive_high` it will be rounded to 1.0.
    /// - lower than or equal to `negative_high` it will be rounded to -1.0.
    /// - Otherwise it will not be rounded.
    ///
    /// If the difference between the calculated value and the `old_value` is lower or
    /// equal to the `threshold`, [`None`] will be returned.
    pub(crate) fn filter(&self, new_value: f32, old_value: Option<f32>) -> Option<f32> {
        let new_value = if new_value <= self.positive_low && new_value >= self.negative_low {
            0.0
        } else if new_value >= self.positive_high {
            1.0
        } else if new_value <= self.negative_high {
            -1.0
        } else {
            new_value
        };

        if let Some(old_value) = old_value {
            if (new_value - old_value).abs() <= self.threshold {
                return None;
            }
        }

        Some(new_value)
    }
}

/// A setting for a [`GamepadButton`].
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
/// The current value of a button is received through the [`GamepadEvent`](crate::gamepad::GamepadEvent)s
/// or [`GamepadEventRaw`](crate::gamepad::GamepadEventRaw)s.
#[derive(Debug, Clone)]
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
    /// Filters the `new_value` according to the specified settings.
    ///
    /// If the `new_value` is:
    /// - lower than or equal to `low` it will be rounded to 0.0.
    /// - higher than or equal to `high` it will be rounded to 1.0.
    /// - Otherwise it will not be rounded.
    ///
    /// If the difference between the calculated value and the `old_value` is lower or
    /// equal to the `threshold`, [`None`] will be returned.
    pub(crate) fn filter(&self, new_value: f32, old_value: Option<f32>) -> Option<f32> {
        let new_value = if new_value <= self.low {
            0.0
        } else if new_value >= self.high {
            1.0
        } else {
            new_value
        };

        if let Some(old_value) = old_value {
            if (new_value - old_value).abs() <= self.threshold {
                return None;
            }
        }

        Some(new_value)
    }
}

#[cfg(test)]
mod tests {
    use crate::gamepad::*;

    mod gamepad_settings {
        use super::*;

        #[test]
        fn test_get_button_settings() {
            let button = GamepadButton::new(Gamepad::new(1), GamepadButtonType::South);
            let mut gamepad_settings = GamepadSettings::default();

            // Default settings
            let settings = ButtonSettings::default();
            let retrieved_settings = gamepad_settings.get_button_settings(button);
            assert_eq!(retrieved_settings.press, settings.press);
            assert_eq!(retrieved_settings.release, settings.release);

            // User defined settings
            let settings = ButtonSettings {
                press: 1.0,
                release: 0.0,
            };

            gamepad_settings
                .button_settings
                .insert(button, settings.clone());

            let retrieved_settings = gamepad_settings.get_button_settings(button);
            assert_eq!(retrieved_settings.press, settings.press);
            assert_eq!(retrieved_settings.release, settings.release);
        }

        #[test]
        fn test_get_axis_settings() {
            let axis = GamepadAxis::new(Gamepad::new(1), GamepadAxisType::LeftStickX);
            let mut gamepad_settings = GamepadSettings::default();

            // Default settings
            let settings = AxisSettings::default();
            let retrieved_settings = gamepad_settings.get_axis_settings(axis);
            assert_eq!(retrieved_settings.positive_high, settings.positive_high);
            assert_eq!(retrieved_settings.positive_low, settings.positive_low,);
            assert_eq!(retrieved_settings.negative_high, settings.negative_high);
            assert_eq!(retrieved_settings.negative_low, settings.negative_low,);
            assert_eq!(retrieved_settings.threshold, settings.threshold);

            // User defined settings
            let settings = AxisSettings {
                positive_high: 1.0,
                positive_low: 0.5,
                negative_high: -1.0,
                negative_low: -0.5,
                threshold: 0.25,
            };

            gamepad_settings
                .axis_settings
                .insert(axis, settings.clone());

            let retrieved_settings = gamepad_settings.get_axis_settings(axis);
            assert_eq!(retrieved_settings.positive_high, settings.positive_high);
            assert_eq!(retrieved_settings.positive_low, settings.positive_low,);
            assert_eq!(retrieved_settings.negative_high, settings.negative_high);
            assert_eq!(retrieved_settings.negative_low, settings.negative_low,);
            assert_eq!(retrieved_settings.threshold, settings.threshold);
        }

        #[test]
        fn test_get_button_axis_settings() {
            let button = GamepadButton::new(Gamepad::new(1), GamepadButtonType::South);
            let mut gamepad_settings = GamepadSettings::default();

            // Default settings
            let settings = ButtonAxisSettings::default();
            let retrieved_settings = gamepad_settings.get_button_axis_settings(button);
            assert_eq!(retrieved_settings.high, settings.high);
            assert_eq!(retrieved_settings.low, settings.low,);
            assert_eq!(retrieved_settings.threshold, settings.threshold);

            // User defined settings
            let settings = ButtonAxisSettings {
                high: 1.0,
                low: 0.5,
                threshold: 0.25,
            };

            gamepad_settings
                .button_axis_settings
                .insert(button, settings.clone());

            let retrieved_settings = gamepad_settings.get_button_axis_settings(button);
            assert_eq!(retrieved_settings.high, settings.high);
            assert_eq!(retrieved_settings.low, settings.low,);
            assert_eq!(retrieved_settings.threshold, settings.threshold);
        }
    }

    mod button_settings {
        use super::*;

        #[test]
        fn test_is_pressed() {
            let settings = ButtonSettings {
                press: 0.9,
                release: 0.1,
            };
            assert!(settings.is_pressed(1.0));
            assert!(settings.is_pressed(0.9));
            assert!(!settings.is_pressed(0.8));
            assert!(!settings.is_pressed(0.7));
        }

        #[test]
        fn test_is_released() {
            let settings = ButtonSettings {
                press: 0.9,
                release: 0.1,
            };
            assert!(settings.is_released(0.0));
            assert!(settings.is_released(0.1));
            assert!(!settings.is_released(0.2));
            assert!(!settings.is_released(0.3));
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

                assert_eq!(expected, actual, "Testing is pressed for value: {}", value);
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

                assert_eq!(expected, actual, "Testing is released for value: {}", value);
            }
        }
    }

    mod axis_settings {
        use super::*;

        #[test]
        fn test_filter() {
            let settings = AxisSettings {
                positive_high: 1.0,
                positive_low: 0.5,
                negative_high: -1.0,
                negative_low: -0.5,
                threshold: 0.25,
            };

            // Between `positive_low` and `negative_low`.
            assert_eq!(settings.filter(0.0, None), Some(0.0));
            assert_eq!(settings.filter(0.5, None), Some(0.0));
            assert_eq!(settings.filter(-0.5, None), Some(0.0));

            // Higher than or equal to `positive_high`.
            assert_eq!(settings.filter(1.0, None), Some(1.0));
            assert_eq!(settings.filter(2.0, None), Some(1.0));
            assert_eq!(settings.filter(3.0, None), Some(1.0));

            // Lower than or equal to `negative_high`.
            assert_eq!(settings.filter(-1.0, None), Some(-1.0));
            assert_eq!(settings.filter(-2.0, None), Some(-1.0));
            assert_eq!(settings.filter(-3.0, None), Some(-1.0));

            // Between `positive_low` and `positive_high`.
            assert_eq!(settings.filter(0.6, None), Some(0.6));
            assert_eq!(settings.filter(0.7, None), Some(0.7));
            assert_eq!(settings.filter(0.8, None), Some(0.8));

            // Between `negative_low` and `negative_high`.
            assert_eq!(settings.filter(-0.6, None), Some(-0.6));
            assert_eq!(settings.filter(-0.7, None), Some(-0.7));
            assert_eq!(settings.filter(-0.8, None), Some(-0.8));

            // Not surpassing the `threshold`.
            assert_eq!(settings.filter(-0.8, Some(-0.6)), None);
            assert_eq!(settings.filter(0.7, Some(0.6)), None);
            assert_eq!(settings.filter(0.9, Some(0.7)), None);
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
                "Testing filtering for {:?} with new_value = {:?}, old_value = {:?}",
                settings, new_value, old_value
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
                let settings = AxisSettings::default();
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
                let settings = AxisSettings::default();
                test_axis_settings_filter(settings, new_value, old_value, expected);
            }
        }
    }

    mod button_axis_settings {
        use super::*;

        #[test]
        fn test_filter() {
            let settings = ButtonAxisSettings {
                high: 1.0,
                low: 0.5,
                threshold: 0.25,
            };

            // Higher than or equal to `high`.
            assert_eq!(settings.filter(1.0, None), Some(1.0));
            assert_eq!(settings.filter(2.0, None), Some(1.0));
            assert_eq!(settings.filter(3.0, None), Some(1.0));

            // Lower than or equal to `low`.
            assert_eq!(settings.filter(0.5, None), Some(0.0));
            assert_eq!(settings.filter(0.4, None), Some(0.0));
            assert_eq!(settings.filter(0.3, None), Some(0.0));

            // Between `low` and `high`.
            assert_eq!(settings.filter(0.6, None), Some(0.6));
            assert_eq!(settings.filter(0.7, None), Some(0.7));
            assert_eq!(settings.filter(0.8, None), Some(0.8));

            // Not surpassing the `threshold`.
            assert_eq!(settings.filter(0.7, Some(0.8)), None);
            assert_eq!(settings.filter(0.8, Some(0.9)), None);
            assert_eq!(settings.filter(0.9, Some(1.0)), None);
        }

        fn test_button_axis_settings_filter(
            settings: ButtonAxisSettings,
            new_value: f32,
            old_value: Option<f32>,
            expected: Option<f32>,
        ) {
            let actual = settings.filter(new_value, old_value);
            assert_eq!(
                expected, actual,
                "Testing filtering for {:?} with new_value = {:?}, old_value = {:?}",
                settings, new_value, old_value
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
    }
}
