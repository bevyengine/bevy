//! This crate provides a set of core widgets for Bevy UI, such as buttons, checkboxes, and sliders.
//! These widgets have no inherent styling, it's the responsibility of the user to add styling
//! appropriate for their game or application.
//!
//! # State Management

mod core_button;
mod events;

use bevy_app::{App, Plugin};
pub use events::{ButtonClicked, ValueChange};

pub use core_button::{CoreButton, CoreButtonPlugin};

/// A plugin that registers the observers for all of the core widgets. If you don't want to
/// use all of the widgets, you can import the individual widget plugins instead.
pub struct CoreWidgetsPlugin;

impl Plugin for CoreWidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            // CoreBarrierPlugin,
            CoreButtonPlugin,
            // CoreCheckboxPlugin,
            // CoreRadioPlugin,
            // CoreRadioGroupPlugin,
            // CoreScrollbarPlugin,
            // CoreSliderPlugin,
            // CursorIconPlugin,
        ));
    }
}
