//! Semantically meaningful actions that can be performed on UI elements.
//!
//! Rather than listening for raw keyboard or picking events, UI elements should listen to these actions
//! for all functional behavior. This allows for more consistent behavior across different input devices
//! and makes it easier to customize input mappings.
//!
//! By contrast, cosmetic behavior like hover effects should generally be implemented by reading the [`Interaction`](crate::focus::Interaction) component,
//! the [`InputFocus`](bevy_input_focus::InputFocus) resource or in response to various [`Pointer`](bevy_picking::events::Pointer) events.
//!
//! # Event bubbling
//!
//! All of the events in this module are will automatically bubble up the entity hierarchy.
//! This allows for more responsiveness to the users' input, as the event will be
//! consumed by the first entity that cares about it.
//!
//! When responding to these events, make sure to call [`Trigger::propagate`] with `false`
//! to prevent the event from being consumed by other later entities.

use bevy_ecs::prelude::*;
use bevy_hierarchy::Parent;
use bevy_reflect::prelude::*;

/// Activate a UI element.
///
/// This is typically triggered by a mouse click (or press),
/// the enter key press on the focused element,
/// or the "A" button on a gamepad.
///
/// [`Button`](crate::widget::Button)s should respond to this action via an observer to perform their primary action.
///
/// # Bubbling
///
/// This event will bubble up the entity hierarchy.
/// Make sure to call [`Trigger::propagate`] with `false` to prevent the event from being consumed by other later entities.
///
/// # Example
///
/// ```rust
/// use bevy_input_focus::InputFocus;
///
/// fn send_activate_event_to_input_focus(keyboard_input: Res<ButtonInput<KeyCode>>, input_focus: Res<InputFocus>, mut commands: Commands) {
///     if keyboard_input.just_pressed(KeyCode::Enter) {
/// 	   if let Some(focused_entity) = input_focus.get() {
/// 		   commands.trigger_targets(Activate, focused_entity);
/// 	   }
///    }
/// }
///
/// fn spawn_my_button(mut commands: Commands) {
///     // This observer will only watch this entity;
///     // use a global observer to respond to *any* Activate event.
///     commands.spawn(Button).observe(activate_my_button);
/// }
///
/// fn activate_my_button(trigger: Trigger<Activate>) {
///    let button_entity = trigger.target();
///    println!("The button with the entity ID {button_entity} was activated!");
///    // We've handled the event, so don't let it bubble up.
///    trigger.propagate(false);
/// }
///
/// # assert_is_system!(send_activate_event_to_input_focus);
/// # assert_is_system!(spawn_my_button);
/// ```
#[derive(Component, Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Default, PartialEq, Hash)]
pub struct Activate;

impl Event for Activate {
    type Traversal = &'static Parent;
    const AUTO_PROPAGATE: bool = true;
}
