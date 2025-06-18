use bevy_app::Plugin;

mod button;
mod slider;

pub use button::{button, ButtonPlugin, ButtonProps, ButtonVariant};
pub use slider::{slider, SliderPlugin, SliderProps};

/// Plugin which registers all feathers controls.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins((ButtonPlugin, SliderPlugin));
    }
}
