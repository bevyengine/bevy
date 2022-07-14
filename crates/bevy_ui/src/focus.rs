use crate::{entity::UiCameraConfig, CalculatedClip, Node};
use bevy_ecs::{
    entity::Entity,
    prelude::Component,
    reflect::ReflectComponent,
    system::{Local, Query, Res},
};
use bevy_input::{mouse::MouseButton, touch::Touches, Input};
use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::camera::{Camera, RenderTarget};
use bevy_transform::components::GlobalTransform;
use bevy_utils::FloatOrd;
use bevy_window::Windows;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Describes what type of input interaction has occurred for a UI node.
///
/// This is commonly queried with a `Changed<Interaction>` filter.
#[derive(
    Component, Copy, Clone, Default, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize,
)]
#[reflect_value(Component, Serialize, Deserialize, PartialEq)]
pub enum Interaction {
    /// The node has been clicked
    Clicked,
    /// The node has been hovered over
    Hovered,
    /// Nothing has happened
    #[default]
    None,
}

/// Describes whether the node should block interactions with lower nodes
#[derive(
    Component, Copy, Clone, Default, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize,
)]
#[reflect_value(Component, Serialize, Deserialize, PartialEq)]
pub enum FocusPolicy {
    /// Blocks interaction
    #[default]
    Block,
    /// Lets interaction pass through
    Pass,
}
/// Contains entities whose Interaction should be set to None
#[derive(Default)]
pub struct State {
    entities_to_reset: SmallVec<[Entity; 1]>,
}

/// The system that sets Interaction for all UI elements based on the mouse cursor activity
pub fn ui_focus_system(
    mut state: Local<State>,
    camera: Query<(&Camera, Option<&UiCameraConfig>)>,
    windows: Res<Windows>,
    mouse_button_input: Res<Input<MouseButton>>,
    touches_input: Res<Touches>,
    mut node_query: Query<(
        Entity,
        &Node,
        &GlobalTransform,
        Option<&mut Interaction>,
        Option<&FocusPolicy>,
        Option<&CalculatedClip>,
    )>,
) {
    // reset entities that were both clicked and released in the last frame
    for entity in state.entities_to_reset.drain(..) {
        if let Ok(mut interaction) = node_query.get_component_mut::<Interaction>(entity) {
            *interaction = Interaction::None;
        }
    }

    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.any_just_released();
    if mouse_released {
        for (_entity, _node, _global_transform, interaction, _focus_policy, _clip) in
            node_query.iter_mut()
        {
            if let Some(mut interaction) = interaction {
                if *interaction == Interaction::Clicked {
                    *interaction = Interaction::None;
                }
            }
        }
    }

    let mouse_clicked =
        mouse_button_input.just_pressed(MouseButton::Left) || touches_input.any_just_pressed();

    let is_ui_disabled =
        |camera_ui| matches!(camera_ui, Some(&UiCameraConfig { show_ui: false, .. }));

    let cursor_position = camera
        .iter()
        .filter(|(_, camera_ui)| !is_ui_disabled(*camera_ui))
        .filter_map(|(camera, _)| {
            if let RenderTarget::Window(window_id) = camera.target {
                Some(window_id)
            } else {
                None
            }
        })
        .filter_map(|window_id| windows.get(window_id))
        .filter(|window| window.is_focused())
        .find_map(|window| window.cursor_position())
        .or_else(|| touches_input.first_pressed_position());

    let mut moused_over_z_sorted_nodes = node_query
        .iter_mut()
        .filter_map(
            |(entity, node, global_transform, interaction, focus_policy, clip)| {
                let position = global_transform.translation();
                let ui_position = position.truncate();
                let extents = node.size / 2.0;
                let mut min = ui_position - extents;
                let mut max = ui_position + extents;
                if let Some(clip) = clip {
                    min = Vec2::max(min, clip.clip.min);
                    max = Vec2::min(max, clip.clip.max);
                }
                // if the current cursor position is within the bounds of the node, consider it for
                // clicking
                let contains_cursor = if let Some(cursor_position) = cursor_position {
                    (min.x..max.x).contains(&cursor_position.x)
                        && (min.y..max.y).contains(&cursor_position.y)
                } else {
                    false
                };

                if contains_cursor {
                    Some((entity, focus_policy, interaction, FloatOrd(position.z)))
                } else {
                    if let Some(mut interaction) = interaction {
                        if *interaction == Interaction::Hovered
                            || (cursor_position.is_none() && *interaction != Interaction::None)
                        {
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
            // don't reset clicked nodes because they're handled separately
            if *interaction != Interaction::Clicked && *interaction != Interaction::None {
                *interaction = Interaction::None;
            }
        }
    }
}
