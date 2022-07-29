use bevy_ecs::{
    entity::Entity,
    event::EventWriter,
    prelude::{Component, With},
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_input::{mouse::MouseButton, touch::Touches, Input};
use bevy_math::Vec2;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::camera::{Camera, RenderTarget};
use bevy_render::view::ComputedVisibility;
use bevy_transform::components::GlobalTransform;
use bevy_ui_navigation::{
    events::NavRequest,
    prelude::{FocusState, Focusable},
};
use bevy_utils::FloatOrd;
use bevy_window::Windows;
use serde::{Deserialize, Serialize};

use crate::{entity::UiCameraConfig, CalculatedClip, Node};

/// Whether the mouse cursor is hovering over this entity.
///
/// If an entity has a `Hover` component, it will be set to [`Hover::Hovered`]
/// if the mouse cursor is hovering it.
///
/// Currently only works on UI.
///
/// There might be several entities under the cursor, so multiple entities might
/// have their `Hover` state set to [`Hover::Hovered`].
///
/// Note that the `Hover` state is completely **independent from** [`FocusPolicy`].

/// For UI interaction, prefer [`Focusable`], as it not only supports mouse interaction
/// and [`FocusPolicy`] but
/// but also gamepad navigation, out of the box.
#[derive(
    Component, Copy, Clone, Default, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize,
)]
#[reflect(Component, Serialize, Deserialize, PartialEq)]
pub enum Hover {
    /// This entity is currently not under the cursor.
    #[default]
    None,
    /// This entity is being hovered.
    Hovered,
}

/// Specify whether this entity should
/// let pointer focus pass through to nodes behind.
///
/// By default, mouse interaction sends focus
/// to the [`Focusable`] closest to the camera.
/// This component allows disabling this behavior
/// with the [`FocusPolicy::Pass`] variant.
///
/// This is useful if you expect overlapping UI elements
/// and want a way for your users to select elements behind others.
///
/// Note that `FocusPolicy` is an optional component,
/// when `FocusPolicy` is absent from the entity,
/// it acts the same as [`FocusPolicy::Capture`].
/// This is also only pertinent to pointer devices such as mouse and touch.
///
/// Also note that this does **not** affect the [`Hover`] component.
#[derive(
    Component, Copy, Clone, Default, Eq, PartialEq, Debug, Reflect, Serialize, Deserialize,
)]
#[reflect(Component, Serialize, Deserialize, PartialEq)]
pub enum FocusPolicy {
    /// Take focus on hover, the default.
    #[default]
    Capture,
    /// Do not focus on hover and let interaction pass through
    /// to [`Focusable`]s behind this one.
    Pass,
}

struct Positions {
    cursor: Vec2,
    node: Vec2,
    size: Vec2,
}

fn get_mouse_cursor(
    camera: &Query<(&Camera, Option<&UiCameraConfig>)>,
    windows: &Windows,
) -> Option<Vec2> {
    let is_ui_disabled =
        |camera_ui| matches!(camera_ui, Some(&UiCameraConfig { show_ui: false, .. }));

    camera
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
}

fn is_under_cursor(
    Positions { cursor, node, size }: Positions,
    node_clip: Option<&CalculatedClip>,
) -> bool {
    let extents = size / 2.0;
    let mut min = node - extents;
    let mut max = node + extents;
    if let Some(clip) = node_clip {
        min = Vec2::max(min, clip.clip.min);
        max = Vec2::min(max, clip.clip.max);
    }
    (min.x..max.x).contains(&cursor.x) && (min.y..max.y).contains(&cursor.y)
}

/// Sends [`NavRequest`] for UI elements based on the mouse cursor and touch activity.
pub fn ui_focus_system(
    camera: Query<(&Camera, Option<&UiCameraConfig>)>,
    windows: Res<Windows>,
    mouse_button_input: Res<Input<MouseButton>>,
    touches_input: Res<Touches>,
    node_query: Query<
        (
            Entity,
            Option<&FocusPolicy>,
            &GlobalTransform,
            &Node,
            Option<&CalculatedClip>,
            Option<&ComputedVisibility>,
        ),
        With<Focusable>,
    >,
    focusables_query: Query<&Focusable>,
    mut nav_requests: EventWriter<NavRequest>,
) {
    let mouse_released =
        mouse_button_input.just_released(MouseButton::Left) || touches_input.any_just_released();

    let cursor =
        get_mouse_cursor(&camera, &windows).or_else(|| touches_input.first_pressed_position());
    let cursor_position = match cursor {
        Some(pos) => pos,
        None => return,
    };
    // TODO: return early of no mouse release and cursor move

    // collect all (visible) entities currently under the cursor.
    let mut moused_over_z_sorted_nodes = node_query
        .iter()
        .filter(|(.., visibility)| visibility.map_or(true, |v| v.is_visible()))
        .filter(|(.., global_transform, node, clip, _)| {
            let positions = Positions {
                node: global_transform.translation().truncate(),
                cursor: cursor_position,
                size: node.size,
            };
            is_under_cursor(positions, *clip)
        })
        .map(|(entity, focus_policy, global_transform, ..)| {
            let z_position = global_transform.translation().z;
            (entity, focus_policy, FloatOrd(z_position))
        })
        .collect::<Vec<_>>();

    moused_over_z_sorted_nodes.sort_by_key(|(_, _, z)| -*z);

    for (entity, focus_policy, _) in moused_over_z_sorted_nodes.into_iter() {
        match focus_policy {
            Some(FocusPolicy::Pass) => {}
            None | Some(FocusPolicy::Capture) => {
                // unwrap: entity taked from a query with a `With<Focusable>` filter
                let focus_state = focusables_query.get(entity).unwrap().state();
                if focus_state != FocusState::Focused {
                    nav_requests.send(NavRequest::FocusOn(entity));
                } else if mouse_released {
                    nav_requests.send(NavRequest::Action);
                }
                break;
            }
        }
    }
}

/// System responsible to update the [`Hover`] component.
pub fn mouse_hover_system(
    camera: Query<(&Camera, Option<&UiCameraConfig>)>,
    windows: Res<Windows>,
    mut hover_query: Query<(
        &mut Hover,
        Option<&CalculatedClip>,
        Option<&ComputedVisibility>,
        &GlobalTransform,
        &Node,
    )>,
) {
    use Hover::Hovered;
    let cursor_position = get_mouse_cursor(&camera, &windows);

    for (mut old_hover, clip, visibility, global_transform, node) in &mut hover_query {
        if visibility.map_or(false, |v| !v.is_visible()) {
            continue;
        }
        let positions = |cursor| Positions {
            cursor,
            size: node.size,
            node: global_transform.translation().truncate(),
        };
        let is_hovered = cursor_position
            .map(positions)
            .map_or(false, |pos| is_under_cursor(pos, clip));

        let new_hover_state = if is_hovered { Hovered } else { Hover::None };
        if *old_hover != new_hover_state {
            *old_hover = new_hover_state;
        }
    }
}
