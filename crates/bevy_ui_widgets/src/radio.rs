use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::{Has, With},
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::FocusedInput;
use bevy_picking::events::{Click, Pointer};
use bevy_reflect::Reflect;
use bevy_ui::{Checkable, Checked, InteractionDisabled};

use crate::ValueChange;

/// Headless widget implementation for a "radio button group". This component is used to group
/// multiple [`RadioButton`] components together, allowing them to behave as a single unit. It
/// implements the tab navigation logic and keyboard shortcuts for radio buttons.
///
/// The [`RadioGroup`] component does not have any state itself, and makes no assumptions about
/// what, if any, value is associated with each radio button, or what Rust type that value might be.
/// Instead, the output of the group is a [`ValueChange`] event whose payload is the entity id of
/// the selected button. This event is emitted whenever a radio button is clicked, or when using
/// the arrow keys while the radio group is focused. The app can then derive the selected value from
/// this using app-specific means, such as accessing a component on the individual buttons.
///
/// The [`RadioGroup`] doesn't actually set the [`Checked`] states directly, that is presumed to
/// happen by the app or via some external data-binding scheme. Typically, each button would be
/// associated with a particular constant value, and would be checked whenever that value is equal
/// to the group's value. This also means that as long as each button's associated value is unique
/// within the group, it should never be the case that more than one button is selected at a time.
#[derive(Component, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::RadioGroup)))]
pub struct RadioGroup;

/// Headless widget implementation for radio buttons. They can be used independently,
/// but enclosing them in a [`RadioGroup`] widget allows them to behave as a single,
/// mutually exclusive unit.
///
/// According to the WAI-ARIA best practices document, radio buttons should not be focusable,
/// but rather the enclosing group should be focusable.
/// See <https://www.w3.org/WAI/ARIA/apg/patterns/radio>/
///
/// The widget emits a [`ValueChange<bool>`] event with the value `true` whenever it becomes checked,
/// either through a mouse click or when a [`RadioGroup`] checks the widget.
/// If the [`RadioButton`] is focusable, it can also be checked using the `Enter` or `Space` keys,
/// in which case the event will likewise be emitted.
#[derive(Component, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::RadioButton)), Checkable)]
#[derive(Reflect)]
#[reflect(Component)]
pub struct RadioButton;

fn radio_group_on_key_input(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_group: Query<(), With<RadioGroup>>,
    q_radio: Query<(Has<Checked>, Has<InteractionDisabled>), With<RadioButton>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    if q_group.contains(ev.focused_entity) {
        let event = &ev.event().input;
        if event.state == ButtonState::Pressed
            && !event.repeat
            && matches!(
                event.key_code,
                KeyCode::ArrowUp
                    | KeyCode::ArrowDown
                    | KeyCode::ArrowLeft
                    | KeyCode::ArrowRight
                    | KeyCode::Home
                    | KeyCode::End
            )
        {
            let key_code = event.key_code;
            ev.propagate(false);

            // Find all radio descendants that are not disabled
            let radio_buttons = q_children
                .iter_descendants(ev.focused_entity)
                .filter_map(|child_id| match q_radio.get(child_id) {
                    Ok((checked, false)) => Some((child_id, checked)),
                    Ok((_, true)) | Err(_) => None,
                })
                .collect::<Vec<_>>();
            if radio_buttons.is_empty() {
                return; // No enabled radio buttons in the group
            }
            let current_index = radio_buttons
                .iter()
                .position(|(_, checked)| *checked)
                .unwrap_or(usize::MAX); // Default to invalid index if none are checked

            let next_index = match key_code {
                KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                    // Navigate to the previous radio button in the group
                    if current_index == 0 || current_index >= radio_buttons.len() {
                        // If we're at the first one, wrap around to the last
                        radio_buttons.len() - 1
                    } else {
                        // Move to the previous one
                        current_index - 1
                    }
                }
                KeyCode::ArrowDown | KeyCode::ArrowRight => {
                    // Navigate to the next radio button in the group
                    if current_index >= radio_buttons.len() - 1 {
                        // If we're at the last one, wrap around to the first
                        0
                    } else {
                        // Move to the next one
                        current_index + 1
                    }
                }
                KeyCode::Home => {
                    // Navigate to the first radio button in the group
                    0
                }
                KeyCode::End => {
                    // Navigate to the last radio button in the group
                    radio_buttons.len() - 1
                }
                _ => {
                    return;
                }
            };

            if current_index == next_index {
                // If the next index is the same as the current, do nothing
                return;
            }

            let (next_id, _) = radio_buttons[next_index];

            // Trigger the value change event on the radio button
            commands.trigger(ValueChange::<bool> {
                source: next_id,
                value: true,
            });
            // Trigger the on_change event for the newly checked radio button on radio group
            commands.trigger(ValueChange::<Entity> {
                source: ev.focused_entity,
                value: next_id,
            });
        }
    }
}

// Provides functionality for standalone focusable [`RadioButton`] to react
// on `Space` or `Enter` key press.
fn radio_button_on_key_input(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_radio_button: Query<(Has<InteractionDisabled>, Has<Checked>), With<RadioButton>>,
    q_group: Query<(), With<RadioGroup>>,
    q_parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let Ok((disabled, checked)) = q_radio_button.get(ev.focused_entity) else {
        // Not a radio button
        return;
    };

    let event = &ev.event().input;
    if event.state == ButtonState::Pressed
        && !event.repeat
        && (event.key_code == KeyCode::Enter || event.key_code == KeyCode::Space)
    {
        ev.propagate(false);

        // Radio button is disabled or already checked
        if disabled || checked {
            return;
        }

        trigger_radio_button_and_radio_group_value_change(
            ev.focused_entity,
            &q_group,
            &q_parents,
            &mut commands,
        );
    }
}

fn radio_button_on_click(
    mut ev: On<Pointer<Click>>,
    q_group: Query<(), With<RadioGroup>>,
    q_radio: Query<(Has<InteractionDisabled>, Has<Checked>), With<RadioButton>>,
    q_parents: Query<&ChildOf>,
    mut commands: Commands,
) {
    let Ok((disabled, checked)) = q_radio.get(ev.entity) else {
        // Not a radio button
        return;
    };

    ev.propagate(false);

    // Radio button is disabled or already checked
    if disabled || checked {
        return;
    }

    trigger_radio_button_and_radio_group_value_change(
        ev.entity,
        &q_group,
        &q_parents,
        &mut commands,
    );
}

fn trigger_radio_button_and_radio_group_value_change(
    radio_button: Entity,
    q_group: &Query<(), With<RadioGroup>>,
    q_parents: &Query<&ChildOf>,
    commands: &mut Commands,
) {
    commands.trigger(ValueChange::<bool> {
        source: radio_button,
        value: true,
    });

    // Find if radio button is inside radio group
    let radio_group = q_parents
        .iter_ancestors(radio_button)
        .find(|ancestor| q_group.contains(*ancestor));

    // If is inside radio group
    if let Some(radio_group) = radio_group {
        // Trigger event for radio group
        commands.trigger(ValueChange::<Entity> {
            source: radio_group,
            value: radio_button,
        });
    }
}

/// Plugin that adds the observers for [`RadioButton`] and [`RadioGroup`] widget.
pub struct RadioGroupPlugin;

impl Plugin for RadioGroupPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(radio_group_on_key_input)
            .add_observer(radio_button_on_click)
            .add_observer(radio_button_on_key_input);
    }
}
