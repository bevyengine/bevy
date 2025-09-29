//! This crate provides a set of standard widgets for Bevy UI, such as buttons, checkboxes, and sliders.
//! These widgets have no inherent styling, it's the responsibility of the user to add styling
//! appropriate for their game or application.
//!
//! ## Warning: Experimental
//!
//! This crate is currently experimental and under active development.
//! The API is likely to change substantially: be prepared to migrate your code.
//!
//! We are actively seeking feedback on the design and implementation of this crate, so please
//! file issues or create PRs if you have any comments or suggestions.
//!
//! ## State Management
//!
//! Most of the widgets use external state management: this means that the widgets do not
//! automatically update their own internal state, but instead rely on the app to update the widget
//! state (as well as any other related game state) in response to a change event emitted by the
//! widget. The primary motivation for this is to avoid two-way data binding in scenarios where the
//! user interface is showing a live view of dynamic data coming from deeper within the game engine.

mod button;
mod callback;
mod checkbox;
mod radio;
mod scrollbar;
mod slider;

pub use button::*;
pub use callback::*;
pub use checkbox::*;
pub use radio::*;
pub use scrollbar::*;
pub use slider::*;

use bevy_app::{PluginGroup, PluginGroupBuilder};
use bevy_ecs::entity::Entity;

/// A plugin group that registers the observers for all of the widgets in this crate. If you don't want to
/// use all of the widgets, you can import the individual widget plugins instead.
pub struct UiWidgetsPlugins;

impl PluginGroup for UiWidgetsPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(ButtonPlugin)
            .add(CheckboxPlugin)
            .add(RadioGroupPlugin)
            .add(ScrollbarPlugin)
            .add(SliderPlugin)
    }
}

/// Notification sent by a button or menu item.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Activate(pub Entity);

/// Notification sent by a widget that edits a scalar value.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ValueChange<T> {
    /// The id of the widget that produced this value.
    pub source: Entity,
    /// The new value.
    pub value: T,
}
