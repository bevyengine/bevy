use crate::Node;
use bevy_core::FloatOrd;
use bevy_ecs::prelude::*;
use bevy_input::{mouse::MouseButton, touch::Touches, Input};
use bevy_transform::components::GlobalTransform;
use bevy_window::Windows;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Interaction {
    Clicked,
    Hovered,
    None,
}

impl Default for Interaction {
    fn default() -> Self {
        Interaction::None
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FocusPolicy {
    Block,
    Pass,
}

impl Default for FocusPolicy {
    fn default() -> Self {
        FocusPolicy::Block
    }
}

#[derive(Default)]
pub struct State {
    hovered_entity: Option<Entity>,
}

pub fn ui_focus_system(
    mut state: Local<State>,
    windows: Res<Windows>,
    mouse_button_input: Res<Input<MouseButton>>,
    touches_input: Res<Touches>,
    mut node_query: Query<(
        Entity,
        &Node,
        &GlobalTransform,
        Option<&mut Interaction>,
        Option<&FocusPolicy>,
    )>,
) {
    let cursor_position = if let Some(cursor_position) = windows
        .get_primary()
        .and_then(|window| window.cursor_position())
    {
        cursor_position
    } else {
        return;
    };

    if mouse_button_input.just_released(MouseButton::Left) || touches_input.just_released(0) {
        for (_entity, _node, _global_transform, interaction, _focus_policy) in node_query.iter_mut()
        {
            if let Some(mut interaction) = interaction {
                if *interaction == Interaction::Clicked {
                    *interaction = Interaction::None;
                }
            }
        }
    }

    let mouse_clicked =
        mouse_button_input.just_pressed(MouseButton::Left) || touches_input.just_released(0);
    let mut hovered_entity = None;

    {
        let mut moused_over_z_sorted_nodes = node_query
            .iter_mut()
            .filter_map(
                |(entity, node, global_transform, interaction, focus_policy)| {
                    let position = global_transform.translation;
                    let ui_position = position.truncate();
                    let extents = node.size / 2.0;
                    let min = ui_position - extents;
                    let max = ui_position + extents;
                    // if the current cursor position is within the bounds of the node, consider it for clicking
                    if (min.x..max.x).contains(&cursor_position.x)
                        && (min.y..max.y).contains(&cursor_position.y)
                    {
                        Some((entity, focus_policy, interaction, FloatOrd(position.z)))
                    } else {
                        if let Some(mut interaction) = interaction {
                            if *interaction == Interaction::Hovered {
                                *interaction = Interaction::None;
                            }
                        }
                        None
                    }
                },
            )
            .collect::<Vec<_>>();

        moused_over_z_sorted_nodes.sort_by_key(|(_, _, _, z)| -*z);
        for (entity, focus_policy, interaction, _) in moused_over_z_sorted_nodes {
            if let Some(mut interaction) = interaction {
                if mouse_clicked {
                    // only consider nodes with ClickState "clickable"
                    if *interaction != Interaction::Clicked {
                        *interaction = Interaction::Clicked;
                    }
                } else if *interaction == Interaction::None {
                    *interaction = Interaction::Hovered;
                }
            }

            hovered_entity = Some(entity);

            match focus_policy.cloned().unwrap_or(FocusPolicy::Block) {
                FocusPolicy::Block => {
                    break;
                }
                FocusPolicy::Pass => { /* allow the next node to be hovered/clicked */ }
            }
        }
    }

    // if there is a new hovered entity, but an entity is currently hovered, unhover the old entity
    if let Some(new_hovered_entity) = hovered_entity {
        if let Some(old_hovered_entity) = state.hovered_entity {
            if new_hovered_entity != old_hovered_entity {
                if let Ok(mut interaction) =
                    node_query.get_component_mut::<Interaction>(old_hovered_entity)
                {
                    if *interaction == Interaction::Hovered {
                        *interaction = Interaction::None;
                    }
                }
                state.hovered_entity = None;
            }
        }
        state.hovered_entity = hovered_entity;
    }
}
