//! Meta-module containing all feathers controls (widgets that are interactive).
use bevy_app::Plugin;

mod button;
mod checkbox;
mod radio;
mod slider;
mod toggle_switch;

pub use button::{button, tool_button, ButtonPlugin, ButtonProps, ButtonVariant};
pub use checkbox::{checkbox, CheckboxPlugin, CheckboxProps};
pub use radio::{radio, RadioPlugin};
pub use slider::{slider, SliderPlugin, SliderProps};
pub use toggle_switch::{toggle_switch, ToggleSwitchPlugin, ToggleSwitchProps};

/// Plugin which registers all `bevy_feathers` controls.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((
            ButtonPlugin,
            CheckboxPlugin,
            RadioPlugin,
            SliderPlugin,
            ToggleSwitchPlugin,
        ));
    }
}
