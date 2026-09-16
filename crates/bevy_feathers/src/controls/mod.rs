//! Meta-module containing all feathers controls (widgets that are interactive).

mod button;
mod checkbox;
mod color_input;
mod color_plane;
mod color_slider;
mod color_swatch;
mod color_swatch_grid;
mod color_wheel;
mod dialog;
mod disclosure_toggle;
mod listview;
mod menu;
mod number_input;
mod radio;
mod scrollbar;
mod select;
mod slider;
mod text_input;
mod toggle_switch;
mod virtual_keyboard;

pub use button::*;
pub use checkbox::*;
pub use color_input::*;
pub use color_plane::*;
pub use color_slider::*;
pub use color_swatch::*;
pub use color_swatch_grid::*;
pub use color_wheel::*;
pub use dialog::*;
pub use disclosure_toggle::*;
pub use listview::*;
pub use menu::*;
pub use number_input::*;
pub use radio::*;
pub use scrollbar::*;
pub use select::*;
pub use slider::*;
pub use text_input::*;
pub use toggle_switch::*;
pub use virtual_keyboard::*;

use crate::alpha_pattern::AlphaPatternPlugin;
use bevy_app::{PluginGroup, PluginGroupBuilder};

/// Plugin group which registers all `bevy_feathers` controls.
pub struct ControlsPlugin;

impl PluginGroup for ControlsPlugin {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(AlphaPatternPlugin)
            .add(ButtonPlugin)
            .add(CheckboxPlugin)
            .add(ColorInputPlugin)
            .add(ColorPlanePlugin)
            .add(ColorSliderPlugin)
            .add(ColorSwatchPlugin)
            .add(ColorSwatchGridPlugin)
            .add(ColorWheelPlugin)
            .add(DisclosureTogglePlugin)
            .add(ListViewPlugin)
            .add(MenuPlugin)
            .add(NumberInputPlugin)
            .add(RadioPlugin)
            .add(ScrollbarPlugin)
            .add(SelectPlugin)
            .add(SliderPlugin)
            .add(TextInputPlugin)
            .add(ToggleSwitchPlugin)
    }
}
