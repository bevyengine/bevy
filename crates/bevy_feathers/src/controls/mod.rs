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

pub use button::*;
pub use checkbox::*;
pub use color_slider::*;
pub use color_swatch::*;
pub use radio::*;
pub use slider::*;
pub use toggle_switch::*;
pub use virtual_keyboard::*;

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
