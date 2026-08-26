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
    system::{Commands, Query, ResMut},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::{FocusGained, FocusLost, FocusedInput, InputFocusVisible};
use bevy_picking::events::{Click, Pointer};
use bevy_reflect::Reflect;
use bevy_ui::{InteractionDisabled, Selectable, Selected};

use crate::{ScrollIntoView, ValueChange};

/// Headless widget implementation for a list box. This component contains multiple [`ListItem`]
/// entities. It implements the tab navigation logic and keyboard shortcuts for list items.
#[derive(Component, Debug, Clone, Default)]
#[require(
    AccessibilityNode(accesskit::Node::new(Role::ListBox)),
    ActiveDescendant
)]
pub struct ListBox;

/// Marker component that indicates we want to support multiple selection of list items.
#[derive(Component, Debug, Clone, Default)]
pub struct ListBoxMultiSelect;

/// Headless widget implementation for listbox items. These should be enclosed within a
/// [`ListBox`] widget, which is responsible for the mutual exclusion logic.
#[derive(Component, Debug, Clone, Default)]
#[require(AccessibilityNode(accesskit::Node::new(Role::ListItem)), Selectable)]
#[derive(Reflect)]
#[reflect(Component)]
pub struct ListItem;

/// Component used for keyboard navigation. Individual rows should not be focusable in
/// the normal way, as this would make tabbing through a long list tedious. Instead, we track
/// the current "active" row separately using a component on the list box. The active row
/// will be displayed with an outline.
///
/// Based on the ARIA `active-descendant` attribute.
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
#[component(immutable)]
pub struct ActiveDescendant(pub Option<Entity>);

fn listbox_on_key_input(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_listbox: Query<&ActiveDescendant, With<ListBox>>,
    q_listitems: Query<(Has<Selected>, Has<InteractionDisabled>), With<ListItem>>,
    q_children: Query<&Children>,
    mut commands: Commands,
    mut focus_visible: ResMut<InputFocusVisible>,
) {
    if q_listbox.contains(ev.focused_entity) {
        let listbox = ev.focused_entity;
        let Ok(active_descendant) = q_listbox.get(listbox) else {
            return;
        };
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
                    | KeyCode::Space
                    | KeyCode::Enter
            )
        {
            let key_code = event.key_code;
            ev.propagate(false);

            // Find all listbox descendants that are not disabled
            let list_items = q_children
                .iter_descendants(listbox)
                .filter_map(|child_id| match q_listitems.get(child_id) {
                    Ok((selected, disabled)) => Some((child_id, selected, disabled)),
                    Err(_) => None,
                })
                .collect::<Vec<_>>();
            if list_items.is_empty() {
                return; // No enabled rows in the group
            }

            // Prefer the current active descendant if it exists
            let prev_active = list_items
                .iter()
                .position(|(id, _, _)| Some(*id) == active_descendant.0)
                .or_else(|| {
                    // Fallback to the first selected row if the active descendant isn't in list_items
                    list_items.iter().position(|(_, selected, _)| *selected)
                })
                .unwrap_or(usize::MAX);

            let next_active = match key_code {
                KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                    // Navigate to the previous list row in the group
                    if prev_active == 0 || prev_active >= list_items.len() {
                        // If we're at the first one, wrap around to the last
                        list_items.len() - 1
                    } else {
                        // Move to the previous one
                        prev_active - 1
                    }
                }
                KeyCode::ArrowDown | KeyCode::ArrowRight => {
                    // Navigate to the next list row in the group
                    if prev_active >= list_items.len() - 1 {
                        // If we're at the last one, wrap around to the first
                        0
                    } else {
                        // Move to the next one
                        prev_active + 1
                    }
                }
                KeyCode::Home => {
                    // Navigate to the first list row in the group
                    0
                }
                KeyCode::End => {
                    // Navigate to the last list row in the group
                    list_items.len() - 1
                }

                KeyCode::Space | KeyCode::Enter => {
                    // Toggle selected state of active row
                    if prev_active < list_items.len() {
                        let (active_id, selected, disabled) = list_items[prev_active];
                        if !selected && !disabled {
                            commands.trigger(ValueChange::<Entity> {
                                source: listbox,
                                value: active_id,
                                is_final: true,
                            });
                        }
                    }
                    return;
                }

                _ => {
                    return;
                }
            };

            // Change active descendant
            let (next_id, _, _) = list_items[next_active];

            // If the next index is the same as the current, do nothing
            if prev_active != next_active {
                focus_visible.0 = true;
                commands
                    .entity(listbox)
                    .insert(ActiveDescendant(Some(next_id)));
            }

            // Scroll active descendant into view
            commands.trigger(ScrollIntoView { entity: next_id });
        }
    }
}

fn listbox_on_row_click(
    mut ev: On<Pointer<Click>>,
    q_listbox: Query<(), With<ListBox>>,
    q_listitems: Query<(Has<Selected>, Has<InteractionDisabled>), With<ListItem>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    if q_listbox.contains(ev.entity) {
        // Processing clicks at the listbox level, not the list item level, so that we can
        // do exclusion. Starting with the original target, search upward for a list row.
        let row_id = if q_listitems.contains(ev.original_event_target()) {
            ev.original_event_target()
        } else {
            // Search ancestors for the first list row
            let mut found_row = None;
            for ancestor in q_parents.iter_ancestors(ev.original_event_target()) {
                if q_listbox.contains(ancestor) {
                    // We reached a list box before finding a list row, bail out
                    return;
                }
                if q_listitems.contains(ancestor) {
                    found_row = Some(ancestor);
                    break;
                }
            }

            match found_row {
                Some(row) => row,
                None => return, // No list row found in the ancestor chain
            }
        };

        // Clicking sets the active descendant, even if disabled
        commands
            .entity(ev.entity)
            .insert(ActiveDescendant(Some(row_id)));

        // List row is disabled.
        if let (_, disabled) = q_listitems.get(row_id).unwrap()
            && disabled
        {
            return;
        }

        // Gather all the enabled list box descendants for exclusion.
        let all_rows = q_children
            .iter_descendants(ev.entity)
            .filter_map(|child_id| match q_listitems.get(child_id) {
                Ok((selected, false)) => Some((child_id, selected)),
                Ok((_, true)) | Err(_) => None,
            })
            .collect::<Vec<_>>();

        if all_rows.is_empty() {
            return; // No enabled list rows in the group
        }

        // Pick out the list row that is currently checked.
        ev.propagate(false);
        let current_row = all_rows
            .iter()
            .find(|(_, checked)| *checked)
            .map(|(id, _)| *id);

        if current_row == Some(row_id) {
            // If they clicked the currently checked list row, do nothing
            return;
        }

        // Trigger the on_change event for the newly checked list row
        commands.trigger(ValueChange::<Entity> {
            source: ev.entity,
            value: row_id,
            is_final: true,
        });
    }
}

/// Update the active descendant on focus changes. Whenever a listbox has focus, it should have
/// an active descendant, which represents the focus row; when a widget loses focus, the active
/// descendant should be cleared.
fn listbox_focus_gained(
    focus: On<FocusGained>,
    q_listbox: Query<(Entity, &ActiveDescendant), With<ListBox>>,
    q_listitems: Query<(Has<Selected>, Has<InteractionDisabled>), With<ListItem>>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    if let Ok((listbox, active_descendant)) = q_listbox.get(focus.entity) {
        // If the listbox is focused, make sure we have an active descendant
        if active_descendant.0.is_none() {
            // Find all listbox descendants that are not disabled
            let list_items = q_children
                .iter_descendants(listbox)
                .filter_map(|child_id| match q_listitems.get(child_id) {
                    Ok((selected, false)) => Some((child_id, selected)),
                    Ok((_, true)) | Err(_) => None,
                })
                .collect::<Vec<_>>();
            if list_items.is_empty() {
                return; // No enabled rows in the group
            }

            // Prefer the current active descendant if it exists, otherwise first element
            let first_selected = list_items
                .iter()
                .position(|(_, selected)| *selected)
                .unwrap_or(0);

            commands
                .entity(listbox)
                .insert(ActiveDescendant(Some(list_items[first_selected].0)));
        }
    }
}

fn listbox_focus_lost(
    focus: On<FocusLost>,
    q_listbox: Query<Entity, With<ListBox>>,
    mut commands: Commands,
) {
    if let Ok(listbox) = q_listbox.get(focus.entity) {
        // Listbox is not focused, clear active descendant
        commands.entity(listbox).insert(ActiveDescendant::default());
    }
}

/// Plugin that adds the observers for the [`ListBox`] widget.
pub struct ListBoxPlugin;

impl Plugin for ListBoxPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(listbox_on_key_input)
            .add_observer(listbox_on_row_click)
            .add_observer(listbox_focus_gained)
            .add_observer(listbox_focus_lost);
    }
}

/// Observer function for updating list row selection state.
pub fn listbox_update_selection(
    value_change: On<ValueChange<Entity>>,
    q_listbox: Query<(), With<ListBox>>,
    q_listitems: Query<(Has<Selected>, Has<InteractionDisabled>), With<ListItem>>,
    q_parents: Query<&ChildOf>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    let change = value_change.event();
    let row = change.value;

    // Find the ListBox that this change applies to. Prefer the event source if it's a ListBox,
    // otherwise walk the ancestors of the row to find the containing ListBox.
    let listbox = if q_listbox.contains(change.source) {
        change.source
    } else {
        // requires: q_parents: Query<&ChildOf>
        let mut found = None;
        for ancestor in q_parents.iter_ancestors(row) {
            if q_listbox.contains(ancestor) {
                found = Some(ancestor);
                break;
            }
        }
        match found {
            Some(lb) => lb,
            None => return, // no containing ListBox found
        }
    };

    // Update selection
    for child in q_children.iter_descendants(listbox) {
        let Ok((selected, interaction_disabled)) = q_listitems.get(child) else {
            continue;
        };
        if interaction_disabled {
            continue;
        }
        if child == row {
            if !selected {
                commands.entity(child).insert(Selected);
            }
        } else {
            if selected {
                commands.entity(child).remove::<Selected>();
            }
        }
    }
}
