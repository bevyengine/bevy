use crate::{FocusPolicy, Interaction, UiStack};
use bevy_ecs::prelude::{Component, Resource};
use bevy_ecs::system::{Local, Query, Res, ResMut};
use bevy_ecs::{change_detection::DetectChangesMut, entity::Entity, query::WorldQuery};
use bevy_input::{prelude::KeyCode, Input};
use bevy_reflect::{FromReflect, Reflect};
use bevy_render::view::ComputedVisibility;

/// A component that represents if a UI element is focused.
#[derive(Reflect, FromReflect, Component, Copy, Clone, Debug, Eq, PartialEq)]
pub enum FocusedState {
    /// Nothing has happened
    None,
    /// Entity is focused
    Focus {
        /// Focus has been reached through keyboard navigation and so a focus style should be displayed.
        /// This is similar to the `:focus-visible` pseudo-class in css.
        focus_visible: bool,
    },
}

/// A resource representing the currently focused entity and the focus visible status.
#[derive(PartialEq, Eq, Debug, Resource, Default)]
pub struct Focused {
    pub entity: Option<Entity>,
    pub focus_visible: bool,
}

/// Should the [`keyboard_navigation_system`] run?
pub(crate) fn tab_pressed(keyboard_input: Res<Input<KeyCode>>) -> bool {
    keyboard_input.just_pressed(KeyCode::Tab)
}

/// Main query for [`keyboard_system`]
#[derive(WorldQuery)]
#[world_query(mutable)]
pub(crate) struct KeyboardQuery {
    interaction: Option<&'static mut Interaction>,
    focus_policy: Option<&'static FocusPolicy>,
    computed_visibility: Option<&'static ComputedVisibility>,
}

/// The system updates the [`Focused`] resource when the user uses keyboard navigation with <kbd>Tab</kbd> or <kbd>Shift</kbd> + <kbd>Tab</kbd>.
///
/// Entities can be focused [`ComputedVisibility`] is visible and [`FocusPolicy`] is block.
pub(crate) fn keyboard_navigation_system(
    mut focused_entity: ResMut<Focused>,
    mut node_query: Query<KeyboardQuery>,
    keyboard_input: Res<Input<KeyCode>>,
    ui_stack: Res<UiStack>,
) {
    let reverse_order =
        keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight);

    let can_focus = |entity: &&Entity| {
        let Ok(node) = node_query.get_mut(**entity) else {
            return false;
        };

        // Nodes that are not rendered should not be interactable
        if let Some(computed_visibility) = node.computed_visibility {
            if !computed_visibility.is_visible() {
                return false;
            }
        }

        // Only allow keyboard navigation to nodes that block focus
        matches!(node.focus_policy, Some(&FocusPolicy::Block))
    };

    let ui_nodes = &ui_stack.uinodes;

    // Current index of the focused entity within the ui nodes list.
    let current_index = ui_nodes
        .iter()
        .position(|&ui_node| Some(ui_node) == focused_entity.entity);

    let new_focus = if reverse_order {
        // Start with the entity before the current focused or at the end of the list
        let first_index = current_index.map(|index| index - 1).unwrap_or_default();

        let wrapping_nodes_iterator = ui_nodes
            .iter()
            .take(first_index)
            .rev()
            .chain(ui_nodes.iter().skip(first_index).rev());

        wrapping_nodes_iterator.filter(can_focus).next().copied()
    } else {
        // Start with the entity after the current focused or at the start of the list
        let first_index = current_index.map(|index| index + 1).unwrap_or_default();

        let wrapping_nodes_iterator = ui_nodes
            .iter()
            .skip(first_index)
            .chain(ui_nodes.iter().take(first_index));

        wrapping_nodes_iterator.filter(can_focus).next().copied()
    };

    // Reset the clicked state
    if new_focus != focused_entity.entity {
        if let Some(node) = focused_entity
            .entity
            .and_then(|entity| node_query.get_mut(entity).ok())
        {
            if let Some(mut interaction) = node.interaction {
                if *interaction == Interaction::Clicked {
                    *interaction = Interaction::None;
                }
            }
        }
    }

    focused_entity.set_if_neq(Focused {
        entity: new_focus,
        focus_visible: true,
    });
}

/// Change the [`FocusedState`] for the specified entity
fn set_focus_state<'a>(
    entity: Option<Entity>,
    focus_state: &'a mut Query<&mut FocusedState>,
    new_state: FocusedState,
) {
    if let Some(mut focus_state) = entity.and_then(|entity| focus_state.get_mut(entity).ok()) {
        focus_state.set_if_neq(new_state);
    }
}

pub(crate) fn update_focused_state(
    mut focus_state: Query<&mut FocusedState>,
    focused_entity: Res<Focused>,
    mut old_focused_entity: Local<Option<Entity>>,
) {
    let new_focused_entity = focused_entity.entity;

    // Remove the interaction from the last focused entity
    if *old_focused_entity != new_focused_entity {
        set_focus_state(*old_focused_entity, &mut focus_state, FocusedState::None);
    }

    let focus_visible = focused_entity.focus_visible;
    let new_state = FocusedState::Focus { focus_visible };
    // Set the focused interaction on the newly focused entity
    set_focus_state(new_focused_entity, &mut focus_state, new_state);

    *old_focused_entity = new_focused_entity;
}

/// Should the [`keyboard_click`] system run?
pub(crate) fn trigger_click(keyboard_input: Res<Input<KeyCode>>) -> bool {
    keyboard_input.just_pressed(KeyCode::Space) || keyboard_input.just_pressed(KeyCode::Return)
}

/// Trigger the [`Focused`] entity to be clicked.
pub(crate) fn keyboard_click(mut interactions: Query<&mut Interaction>, focus: Res<Focused>) {
    if let Some(mut interaction) = focus
        .entity
        .and_then(|entity| interactions.get_mut(entity).ok())
    {
        interaction.set_if_neq(Interaction::Clicked);
    }
}

/// Should the [`end_keyboard_click`] system run?
pub(crate) fn trigger_click_end(keyboard_input: Res<Input<KeyCode>>) -> bool {
    keyboard_input.just_released(KeyCode::Space) || keyboard_input.just_released(KeyCode::Return)
}

/// Reset the clicked state.
pub(crate) fn end_keyboard_click(mut interactions: Query<&mut Interaction>) {
    interactions.for_each_mut(|mut interaction| {
        if *interaction == Interaction::Clicked {
            *interaction = Interaction::None
        }
    });
}
