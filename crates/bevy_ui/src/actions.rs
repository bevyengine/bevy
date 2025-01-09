//! Semantically meaningful actions that can be performed on UI elements.
//!
//! Rather than listening for raw keyboard or picking events, UI elements should listen to these actions
//! for all functional behavior. This allows for more consistent behavior across different input devices
//! and makes it easier to customize input mappings.
//!
//! By contrast, cosmetic behavior like hover effects should generally be implemented by reading the [`Interaction`](crate::focus::Interaction) component,
//! the [`InputFocus`](bevy_input_focus::InputFocus) resource or in response to various [`Pointer`](bevy_picking::events::Pointer) events.

use bevy_ecs::event::Event;

/// Activate a UI element.
///
/// This is typically triggered by a mouse click (or press),
/// the enter key press on the focused element,
/// or the "A" button on a gamepad.
///
/// Buttons should respond to this action via an observer to perform their primary action.
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
/// }
///
/// # assert_is_system!(send_activate_event_to_input_focus);
/// # assert_is_system!(spawn_my_button);
/// ```
#[derive(Debug, Event, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Activate;
