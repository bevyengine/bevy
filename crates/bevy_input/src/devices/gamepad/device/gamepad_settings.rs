use bevy_input::gamepad::ButtonSettings;

#[derive(Default, Debug)]
pub struct GamepadSettings {
    pub default_button_settings: ButtonSettings,
    pub default_axis_settings: AxisSettings,
    pub default_button_axis_settings: ButtonAxisSettings,
    pub button_settings: HashMap<GamepadButton, ButtonSettings>,
    pub axis_settings: HashMap<GamepadAxis, AxisSettings>,
    pub button_axis_settings: HashMap<GamepadButton, ButtonAxisSettings>,
}

impl GamepadSettings {
    pub fn get_button_settings(&self, button: GamepadButton) -> &ButtonSettings {
        self.button_settings
            .get(&button)
            .unwrap_or(&self.default_button_settings)
    }

    pub fn get_axis_settings(&self, axis: GamepadAxis) -> &AxisSettings {
        self.axis_settings
            .get(&axis)
            .unwrap_or(&self.default_axis_settings)
    }

    pub fn get_button_axis_settings(&self, button: GamepadButton) -> &ButtonAxisSettings {
        self.button_axis_settings
            .get(&button)
            .unwrap_or(&self.default_button_axis_settings)
    }
}
