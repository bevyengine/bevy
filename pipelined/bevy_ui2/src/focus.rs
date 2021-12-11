use crate::Node;
use bevy_core::FloatOrd;
use bevy_ecs::{
    entity::Entity,
    prelude::Component,
    reflect::ReflectComponent,
    system::{Local, Query, Res},
};
use bevy_input::{mouse::MouseButton, touch::Touches, Input};
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_transform::components::GlobalTransform;
use bevy_window::Windows;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize)]
#[reflect_value(Component, Serialize, Deserialize, PartialEq)]
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

#[derive(Component, Copy, Clone, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize)]
#[reflect_value(Component, Serialize, Deserialize, PartialEq)]
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
    entities_to_reset: SmallVec<[Entity; 1]>,
}

#[allow(clippy::type_complexity)]
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

    // reset entities that were both clicked and released in the last frame
    for entity in state.entities_to_reset.drain(..) {
        if let Ok(mut interaction) = node_query.get_component_mut::<Interaction>(entity) {
            *interaction = Interaction::None;
        }
    }

    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.just_released(0);
    if mouse_released {
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

    let mut moused_over_z_sorted_nodes = node_query
        .iter_mut()
        .filter_map(
            |(entity, node, global_transform, interaction, focus_policy)| {
                let position = global_transform.translation;
                let ui_position = position.truncate();
                let extents = node.size / 2.0;
                let min = ui_position - extents;
                let max = ui_position + extents;
                // if the current cursor position is within the bounds of the node, consider it for
                // clicking
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

    let mut moused_over_z_sorted_nodes = moused_over_z_sorted_nodes.into_iter();
    // set Clicked or Hovered on top nodes
    for (entity, focus_policy, interaction, _) in moused_over_z_sorted_nodes.by_ref() {
        if let Some(mut interaction) = interaction {
            if mouse_clicked {
                // only consider nodes with Interaction "clickable"
                if *interaction != Interaction::Clicked {
                    *interaction = Interaction::Clicked;
                    // if the mouse was simultaneously released, reset this Interaction in the next
                    // frame
                    if mouse_released {
                        state.entities_to_reset.push(entity);
                    }
                }
            } else if *interaction == Interaction::None {
                *interaction = Interaction::Hovered;
            }
        }

        match focus_policy.cloned().unwrap_or(FocusPolicy::Block) {
            FocusPolicy::Block => {
                break;
            }
            FocusPolicy::Pass => { /* allow the next node to be hovered/clicked */ }
        }
    }
    // reset lower nodes to None
    for (_entity, _focus_policy, interaction, _) in moused_over_z_sorted_nodes {
        if let Some(mut interaction) = interaction {
            if *interaction != Interaction::None {
                *interaction = Interaction::None;
            }
        }
    }
}
