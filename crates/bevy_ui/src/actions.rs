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
//!
//! # Systems
//!
//! Various public systems are provided to trigger these actions in response to raw input events.
//! These systems run in [`PreUpdate`](bevy_app::main_schedule::PreUpdate) as part of [`UiSystem::Actions`](crate::UiSystem::Actions).
//! They are all enabled by default in the [`UiPlugin`](crate::UiPlugin),
//! but are split apart for more control over when / if they are run via run conditions.
//!
//! To disable them entirely, set [`UiPlugin::actions`](crate::UiPlugin::actions) to `false`.

use bevy_ecs::prelude::*;
use bevy_hierarchy::Parent;
use bevy_input::{
    gamepad::{Gamepad, GamepadButton},
    keyboard::KeyCode,
    ButtonInput,
};
use bevy_input_focus::InputFocus;
use bevy_picking::events::{Click, Pointer};
use bevy_reflect::prelude::*;

use crate::Node;

/// A system which triggers the [`Activate`](crate::Activate) action
/// when an entity with the [`Node`] component is clicked.
pub fn activate_ui_elements_on_click(
    mut click_events: EventReader<Pointer<Click>>,
    node_query: Query<(), With<Node>>,
    mut commands: Commands,
) {
    for click in click_events.read() {
        if node_query.contains(click.target) {
            commands.trigger_targets(Activate, click.target);
        }
    }
}

/// A system which activates the [`Activate`](crate::Activate) action
/// when [`KeyCode::Enter`] is first pressed.
pub fn activate_focus_on_enter(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    input_focus: Res<InputFocus>,
    mut commands: Commands,
) {
    if keyboard_input.just_pressed(KeyCode::Enter) {
        if let Some(focused_entity) = input_focus.get() {
            commands.trigger_targets(Activate, focused_entity);
        }
    }
}

/// A system which activates the [`Activate`](crate::Activate) action
/// when [`GamepadButton::South`] is first pressed on any controller.
///
/// This system is generally not suitable for local co-op games,
/// as *any* gamepad can activate the focused element.
///
/// # Warning
///
/// Note that for Nintendo Switch controllers, the "A" button (commonly used as "activate"),
///  is *not* the South button. It's instead the [`GamepadButton::East`].
pub fn activate_focus_on_gamepad_south(
    input_focus: Res<InputFocus>,
    gamepads: Query<&Gamepad>,
    mut commands: Commands,
) {
    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::South) {
            if let Some(focused_entity) = input_focus.get() {
                commands.trigger_targets(Activate, focused_entity);
                // Only send one activate event per frame,
                // even if multiple gamepads pressed the button.
                return;
            }
        }
    }
}

/// A system which activates the [`Activate`](crate::Activate) action
/// when the [`GamepadButtonType::South`] is pressed.

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
/// use bevy_ecs::prelude::*;
/// use bevy_input_focus::InputFocus;
/// use bevy_ui::Activate;
/// use bevy_input::keyboard::KeyCode;
///
/// // This system is already added to the `UiPlugin` by default.
/// fn activate_focus_on_enter(keyboard_input: Res<ButtonInput<KeyCode>>, input_focus: Res<InputFocus>, mut commands: Commands) {
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
/// # assert_is_system!(activate_focus_on_enter);
/// # assert_is_system!(spawn_my_button);
/// ```
#[derive(Component, Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Reflect)]
#[reflect(Component, Default, PartialEq, Hash)]
pub struct Activate;

impl Event for Activate {
    type Traversal = &'static Parent;
    const AUTO_PROPAGATE: bool = true;
}
