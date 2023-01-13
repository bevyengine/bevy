//! This module contains the basic building blocks of Bevy's UI

mod button;
mod image;
mod slider;
mod text;

use bevy_app::{Plugin, App};
pub use button::*;
pub use image::*;
pub use slider::*;
pub use text::*;

/// The plugin for UI widgets
#[derive(Default)]
pub struct WidgetPlugin;

impl Plugin for WidgetPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugin(ButtonPlugin)
			.add_plugin(TextPlugin)
			.add_plugin(ImagePlugin)
			.add_plugin(SliderPlugin);
	}
}
