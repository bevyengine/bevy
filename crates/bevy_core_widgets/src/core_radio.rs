use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::hierarchy::{ChildOf, Children};
use bevy_ecs::query::Has;
use bevy_ecs::system::In;
use bevy_ecs::{
    component::Component,
    observer::On,
    query::With,
    system::{Commands, Query},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::FocusedInput;
use bevy_picking::events::{Click, Pointer};
use bevy_ui::{Checkable, Checked, InteractionDisabled};

use crate::{Activate, Callback, Notify};

/// Headless widget implementation for a "radio button group". This component is used to group
/// multiple [`CoreRadio`] components together, allowing them to behave as a single unit. It
/// implements the tab navigation logic and keyboard shortcuts for radio buttons.
///
/// The [`CoreRadioGroup`] component does not have any state itself, and makes no assumptions about
/// what, if any, value is associated with each radio button, or what Rust type that value might be.
/// Instead, the output of the group is the entity id of the selected button. The app can then
/// derive the selected value from this using app-specific means, such as accessing a component on
/// the individual buttons.
///
/// The [`CoreRadioGroup`] doesn't actually set the [`Checked`] states directly, that is presumed to
/// happen by the app or via some external data-binding scheme. Typically, each button would be
/// associated with a particular constant value, and would be checked whenever that value is equal
/// to the group's value. This also means that as long as each button's associated value is unique
/// within the group, it should never be the case that more than one button is selected at a time.
#[derive(Component, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::RadioGroup)))]
pub struct CoreRadioGroup {
    /// Callback which is called when the selected radio button changes.
    pub on_change: Callback<In<Activate>>,
}

/// Headless widget implementation for radio buttons. These should be enclosed within a
/// [`CoreRadioGroup`] widget, which is responsible for the mutual exclusion logic.
///
/// According to the WAI-ARIA best practices document, radio buttons should not be focusable,
/// but rather the enclosing group should be focusable.
/// See <https://www.w3.org/WAI/ARIA/apg/patterns/radio>/
#[derive(Component, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::RadioButton)), Checkable)]
pub struct CoreRadio;

fn radio_group_on_key_input(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_group: Query<&CoreRadioGroup>,
    q_radio: Query<(Has<Checked>, Has<InteractionDisabled>), With<CoreRadio>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    if let Ok(CoreRadioGroup { on_change }) = q_group.get(ev.target()) {
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
                .iter_descendants(ev.target())
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

            // Trigger the on_change event for the newly checked radio button
            commands.notify_with(on_change, Activate(next_id));
        }
    }
}

fn radio_group_on_button_click(
    mut ev: On<Pointer<Click>>,
    q_group: Query<&CoreRadioGroup>,
    q_radio: Query<(Has<Checked>, Has<InteractionDisabled>), With<CoreRadio>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    if let Ok(CoreRadioGroup { on_change }) = q_group.get(ev.target()) {
        // Starting with the original target, search upward for a radio button.
        let radio_id = if q_radio.contains(ev.original_target()) {
            ev.original_target()
        } else {
            // Search ancestors for the first radio button
            let mut found_radio = None;
            for ancestor in q_parents.iter_ancestors(ev.original_target()) {
                if q_group.contains(ancestor) {
                    // We reached a radio group before finding a radio button, bail out
                    return;
                }
                if q_radio.contains(ancestor) {
                    found_radio = Some(ancestor);
                    break;
                }
            }

            match found_radio {
                Some(radio) => radio,
                None => return, // No radio button found in the ancestor chain
            }
        };

        // Radio button is disabled.
        if q_radio.get(radio_id).unwrap().1 {
            return;
        }

        // Gather all the enabled radio group descendants for exclusion.
        let radio_buttons = q_children
            .iter_descendants(ev.target())
            .filter_map(|child_id| match q_radio.get(child_id) {
                Ok((checked, false)) => Some((child_id, checked)),
                Ok((_, true)) | Err(_) => None,
            })
            .collect::<Vec<_>>();

        if radio_buttons.is_empty() {
            return; // No enabled radio buttons in the group
        }

        // Pick out the radio button that is currently checked.
        ev.propagate(false);
        let current_radio = radio_buttons
            .iter()
            .find(|(_, checked)| *checked)
            .map(|(id, _)| *id);

        if current_radio == Some(radio_id) {
            // If they clicked the currently checked radio button, do nothing
            return;
        }

        // Trigger the on_change event for the newly checked radio button
        commands.notify_with(on_change, Activate(radio_id));
    }
}

/// Plugin that adds the observers for the [`CoreRadioGroup`] widget.
pub struct CoreRadioGroupPlugin;

impl Plugin for CoreRadioGroupPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(radio_group_on_key_input)
            .add_observer(radio_group_on_button_click);
    }
}
