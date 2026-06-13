//! Meta-module containing all feathers controls (widgets that are interactive).
#![expect(deprecated, reason = "deprecated control bundles are exported here")]

mod button;
mod checkbox;
mod color_plane;
mod color_slider;
mod color_swatch;
mod dialog;
mod disclosure_toggle;
mod listview;
mod menu;
mod number_input;
mod radio;
mod scrollbar;
mod slider;
mod text_input;
mod toggle_switch;
mod virtual_keyboard;

pub use button::*;
pub use checkbox::*;
pub use color_plane::*;
pub use color_slider::*;
pub use color_swatch::*;
pub use dialog::*;
pub use disclosure_toggle::*;
pub use listview::*;
pub use menu::*;
pub use number_input::*;
pub use radio::*;
pub use scrollbar::*;
pub use slider::*;
pub use text_input::*;
pub use toggle_switch::*;
pub use virtual_keyboard::*;

use crate::alpha_pattern::AlphaPatternPlugin;
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
            DisclosureTogglePlugin,
            ListViewPlugin,
            MenuPlugin,
            RadioPlugin,
            ScrollbarPlugin,
            SliderPlugin,
            TextInputPlugin,
            ToggleSwitchPlugin,
        ));
    }
}
