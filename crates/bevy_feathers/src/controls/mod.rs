//! Meta-module containing all feathers controls (widgets that are interactive).
#![expect(deprecated, reason = "deprecated control bundles are exported here")]

mod button;
mod checkbox;
mod color_plane;
mod color_slider;
mod color_swatch;
mod radio;
mod slider;
mod toggle_switch;
mod virtual_keyboard;

pub use button::{button, button_bundle, ButtonPlugin, ButtonProps, ButtonVariant};
pub use checkbox::{checkbox, checkbox_bundle, CheckboxPlugin};
pub use color_plane::{color_plane, color_plane_bundle, ColorPlane, ColorPlaneValue};
pub use color_slider::{
    color_slider, color_slider_bundle, ColorChannel, ColorSlider, ColorSliderPlugin,
    ColorSliderProps, SliderBaseColor,
};
pub use color_swatch::{
    color_swatch, color_swatch_bundle, ColorSwatch, ColorSwatchFg, ColorSwatchValue,
};
pub use radio::{radio, radio_bundle, RadioPlugin};
pub use slider::{slider, slider_bundle, SliderPlugin, SliderProps};
pub use toggle_switch::{toggle_switch, toggle_switch_bundle, ToggleSwitchPlugin};
pub use virtual_keyboard::{virtual_keyboard, virtual_keyboard_bundle, VirtualKeyPressed};

use crate::{
    alpha_pattern::AlphaPatternPlugin,
    controls::{color_plane::ColorPlanePlugin, color_swatch::ColorSwatchPlugin},
};
use bevy_app::Plugin;

/// Plugin which registers all `bevy_feathers` controls.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((
            AlphaPatternPlugin,
            ButtonPlugin,
            CheckboxPlugin,
            ColorPlanePlugin,
            ColorSliderPlugin,
            ColorSwatchPlugin,
            RadioPlugin,
            SliderPlugin,
            ToggleSwitchPlugin,
        ));
    }
}
