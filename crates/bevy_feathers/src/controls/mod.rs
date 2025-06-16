use bevy_app::Plugin;

use crate::controls::button::ButtonPlugin;

mod button;
mod slider;

/// Plugin which registers all feathers controls.
pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(ButtonPlugin);
    }
}
