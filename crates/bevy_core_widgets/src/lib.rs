//! This crate provides a set of core widgets for Bevy UI, such as buttons, checkboxes, and sliders.
//! These widgets have no inherent styling, it's the responsibility of the user to add styling
//! appropriate for their game or application.
//!
//! # State Management
//!
//! Most of the widgets use external state management: this means that the widgets do not
//! automatically update their own internal state, but instead rely on the app to update the widget
//! state (as well as any other related game state) in response to a change event emitted by the
//! widget. The primary motivation for this is to avoid two-way data binding in scenarios where the
//! user interface is showing a live view of dynamic data coming from deeper within the game engine.

// Note on naming: the `Core` prefix is used on components that would normally be internal to the
// styled/opinionated widgets that use them. Components which are directly exposed to users above
// the widget level, like `SliderValue`, should not have the `Core` prefix.

mod callback;
mod core_button;
mod core_checkbox;
mod core_radio;
mod core_scrollbar;
mod core_slider;

use bevy_app::{PluginGroup, PluginGroupBuilder};

use bevy_ecs::entity::Entity;
pub use callback::{Callback, Notify};
pub use core_button::{CoreButton, CoreButtonPlugin};
pub use core_checkbox::{CoreCheckbox, CoreCheckboxPlugin, SetChecked, ToggleChecked};
pub use core_radio::{CoreRadio, CoreRadioGroup, CoreRadioGroupPlugin};
pub use core_scrollbar::{
    ControlOrientation, CoreScrollbar, CoreScrollbarDragState, CoreScrollbarPlugin,
    CoreScrollbarThumb,
};
pub use core_slider::{
    CoreSlider, CoreSliderDragState, CoreSliderPlugin, CoreSliderThumb, SetSliderValue,
    SliderPrecision, SliderRange, SliderStep, SliderValue, TrackClick,
};

/// A plugin group that registers the observers for all of the core widgets. If you don't want to
/// use all of the widgets, you can import the individual widget plugins instead.
pub struct CoreWidgetsPlugins;

impl PluginGroup for CoreWidgetsPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(CoreButtonPlugin)
            .add(CoreCheckboxPlugin)
            .add(CoreRadioGroupPlugin)
            .add(CoreScrollbarPlugin)
            .add(CoreSliderPlugin)
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
