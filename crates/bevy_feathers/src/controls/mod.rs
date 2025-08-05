//! Meta-module containing all feathers controls (widgets that are interactive).
use bevy_app::Plugin;

mod button;
mod checkbox;
mod color_slider;
mod color_swatch;
mod radio;
mod slider;
mod toggle_switch;
mod virtual_keyboard;

pub use button::{button, ButtonPlugin, ButtonProps, ButtonVariant};
pub use checkbox::{checkbox, CheckboxPlugin, CheckboxProps};
pub use color_slider::{
    color_slider, ColorChannel, ColorSlider, ColorSliderPlugin, ColorSliderProps, SliderBaseColor,
};
pub use color_swatch::{color_swatch, ColorSwatch, ColorSwatchFg};
pub use radio::{radio, RadioPlugin};
pub use slider::{slider, SliderPlugin, SliderProps};
pub use toggle_switch::{toggle_switch, ToggleSwitchPlugin, ToggleSwitchProps};
pub use virtual_keyboard::virtual_keyboard;

use crate::alpha_pattern::AlphaPatternPlugin;

/// Plugin which registers all `bevy_feathers` controls.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((
            AlphaPatternPlugin,
            ButtonPlugin,
            CheckboxPlugin,
            ColorSliderPlugin,
            RadioPlugin,
            SliderPlugin,
            ToggleSwitchPlugin,
        ));
    }
}
