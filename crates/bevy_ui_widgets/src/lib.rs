//! This crate provides a set of standard widgets for Bevy UI, such as buttons, checkboxes, and sliders.
//! These widgets have no inherent styling, it's the responsibility of the user to add styling
//! appropriate for their game or application.
//!
//! ## State Management
//!
//! The typical way that UI widgets are used in games is that they often display a view of live
//! data in the engine - they aren't just passive form fields to fill in and then submit.
//! For example, a "color picker" might have multiple ways to edit an RGB value, including
//! sliders, a color plane, a color wheel, hex input, or swatches of recent colors. Interacting with
//! any of these widgets not only updates the RGB model, but also updates all of the other widgets.
//!
//! For this reason, almost all of the widgets use **external** state management: this means that
//! the widgets do not automatically update their own internal state, but instead rely on the app
//! to update the widget state (as well as any other related game state) in response to a change
//! event emitted by the widget.
//!
//! These widgets emit a [`ValueChange`] event which carries the proposed new state; the app should respond
//! to this event by updating its own internal model, and by updating the widget state. The general
//! convention is that the widget state is contained in a specific state-bearing component
//! (like [`SliderValue`]), and the widget detects when this component is inserted or replaced.
//!
//! This design pattern draws from the classic "MVC" (Model / View / Controller) style of
//! widget design, which is also used by React.js for
//! [controlled](https://medium.com/@rupalsinghal/controlled-vs-uncontrolled-components-in-react-the-complete-guide-you-cant-afford-to-miss-fbf6ea28b0fd)
//! widgets.
//!
//! There are a few exceptions to this rule:
//! * Buttons and menu items don't have any state, so they emit an [`Activate`] event instead.
//!   (The hover and pressed states don't count, because they are purely visual / stylistic).
//! * Text input widgets have a large and expensive state, so they handle their own state updates.
//! * Scrollbars don't emit events, they modify the scroll position of the target entity
//!   directly.
//!
//! For users who don't want to bother with writing a state update handler, these widgets provide
//! a "self update" observer function: for example, the [`checkbox_self_update`] observer will
//! listen for the [`ValueChange`] event and update the checkbox state. This effectively converts
//! it into an "uncontrolled" widget, in React.js terms.
//!
//! ## Best practices for event propagation
//!
//! Generally, when a widget handles an event,
//! propagation of that event to parent entities should be stopped. Events which are not of
//! interest to the widget should be allowed to propagate.
//! This "consume what you use" principle is important when writing your custom widgets, and
//! understanding the behavior of existing widgets.
//!
//! For more guidance on this, see the documentation for [`EntityEvent`].

mod button;
mod checkbox;
mod dialog;
mod list;
mod menu;
mod modal;
mod observe;
pub mod popover;
mod radio;
mod scrollarea;
mod scrollbar;
mod slider;
mod text_input;

use bevy_input_focus::pointer_focus::PointerFocusPlugin;
pub use button::*;
pub use checkbox::*;
pub use dialog::*;
pub use list::*;
pub use menu::*;
pub use modal::*;
pub use observe::*;
pub use radio::*;
pub use scrollarea::*;
pub use scrollbar::*;
pub use slider::*;
pub use text_input::*;

use bevy_app::{PluginGroup, PluginGroupBuilder};
use bevy_ecs::{entity::Entity, event::EntityEvent, reflect::ReflectEvent};
use bevy_reflect::Reflect;

use crate::popover::PopoverPlugin;

/// A plugin group that registers the observers for all of the widgets in this crate. If you don't want to
/// use all of the widgets, you can import the individual widget plugins instead.
#[derive(Default)]
pub struct UiWidgetsPlugins;

impl PluginGroup for UiWidgetsPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(ButtonPlugin)
            .add(CheckboxPlugin)
            .add(EditableTextInputPlugin)
            .add(ListBoxPlugin)
            .add(MenuPlugin)
            .add(DialogPlugin)
            .add(ModalDialogPlugin)
            .add(PopoverPlugin)
            .add(RadioGroupPlugin)
            .add(ScrollAreaPlugin)
            .add(ScrollbarPlugin)
            .add(SliderPlugin)
            .add(PointerFocusPlugin)
    }
}

/// Notification sent by a button or menu item.
#[derive(Copy, Clone, Debug, PartialEq, EntityEvent, Reflect)]
#[reflect(Event)]
pub struct Activate {
    /// The activated entity.
    pub entity: Entity,
}

/// Notification sent by a widget that edits a scalar value.
#[derive(Copy, Clone, Debug, PartialEq, EntityEvent, Reflect)]
#[reflect(Event)]
pub struct ValueChange<T> {
    /// The id of the widget that produced this value.
    #[event_target]
    pub source: Entity,
    /// The new value.
    pub value: T,
    /// If false, it means that we are in the middle of an interaction (slider being dragged,
    /// user typing), while if true it means that the user's interaction is finished (mouse button
    /// released, drag ended, input lost focus).
    pub is_final: bool,
}
