//! A picking backend for UI nodes.
//!
//! # Usage
//!
//! This backend does not require markers on cameras or entities to function. It will look for any
//! pointers using the same render target as the UI camera, and run hit tests on the UI node tree.
//!
//! ## Important Note
//!
//! This backend completely ignores [`FocusPolicy`](crate::FocusPolicy). The design of `bevy_ui`'s
//! focus systems and the picking plugin are not compatible. Instead, use the optional [`Pickable`] component
//! to override how an entity responds to picking focus. Nodes without the [`Pickable`] component
//! will still trigger events and block items below it from being hovered.
//!
//! ## Implementation Notes
//!
//! - `bevy_ui` can only render to the primary window
//! - `bevy_ui` can render on any camera with a flag, it is special, and is not tied to a particular
//!   camera.
//! - To correctly sort picks, the order of `bevy_ui` is set to be the camera order plus 0.5.
//! - The `position` reported in `HitData` is normalized relative to the node, with `(0.,0.,0.)` at
//!   the top left and `(1., 1., 0.)` in the bottom right. Coordinates are relative to the entire
//!   node, not just the visible region. This backend does not provide a `normal`.

#![deny(missing_docs)]

use crate::{focus::pick_rounded_rect, prelude::*, UiStack};
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, query::QueryData};
use bevy_math::{Rect, Vec2};
use bevy_platform::collections::HashMap;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::prelude::*;
use bevy_transform::prelude::*;
use bevy_window::PrimaryWindow;

use bevy_picking::backend::prelude::*;

/// An optional component that marks cameras that should be used in the [`UiPickingPlugin`].
///
/// Only needed if [`UiPickingSettings::require_markers`] is set to `true`, and ignored
/// otherwise.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Debug, Default, Component)]
pub struct UiPickingCamera;

/// Runtime settings for the [`UiPickingPlugin`].
#[derive(Resource, Reflect)]
#[reflect(Resource, Default)]
pub struct UiPickingSettings {
    /// When set to `true` UI picking will only consider cameras marked with
    /// [`UiPickingCamera`] and entities marked with [`Pickable`]. `false` by default.
    ///
    /// This setting is provided to give you fine-grained control over which cameras and entities
    /// should be used by the UI picking backend at runtime.
    pub require_markers: bool,
}

#[expect(
    clippy::allow_attributes,
    reason = "clippy::derivable_impls is not always linted"
)]
#[allow(
    clippy::derivable_impls,
    reason = "Known false positive with clippy: <https://github.com/rust-lang/rust-clippy/issues/13160>"
)]
impl Default for UiPickingSettings {
    fn default() -> Self {
        Self {
            require_markers: false,
        }
    }
}

/// A plugin that adds picking support for UI nodes.
///
/// This is included by default in [`UiPlugin`](crate::UiPlugin).
#[derive(Clone)]
pub struct UiPickingPlugin;
impl Plugin for UiPickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiPickingSettings>()
            .register_type::<(UiPickingCamera, UiPickingSettings)>()
            .add_systems(PreUpdate, ui_picking.in_set(PickSet::Backend));
    }
}

/// Main query from bevy's `ui_focus_system`
#[derive(QueryData)]
#[query_data(mutable)]
pub struct NodeQuery {
    entity: Entity,
    node: &'static ComputedNode,
    global_transform: &'static GlobalTransform,
    pickable: Option<&'static Pickable>,
    calculated_clip: Option<&'static CalculatedClip>,
    inherited_visibility: Option<&'static InheritedVisibility>,
    target_camera: &'static ComputedNodeTarget,
}

/// Computes the UI node entities under each pointer.
///
/// Bevy's [`UiStack`] orders all nodes in the order they will be rendered, which is the same order
/// we need for determining picking.
pub fn ui_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    camera_query: Query<(
        Entity,
        &Camera,
        Has<IsDefaultUiCamera>,
        Has<UiPickingCamera>,
    )>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    settings: Res<UiPickingSettings>,
    ui_stack: Res<UiStack>,
    node_query: Query<NodeQuery>,
    mut output: EventWriter<PointerHits>,
) {
    // For each camera, the pointer and its position
    let mut pointer_pos_by_camera = HashMap::<Entity, HashMap<PointerId, Vec2>>::default();

    for (pointer_id, pointer_location) in
        pointers.iter().filter_map(|(pointer, pointer_location)| {
            Some(*pointer).zip(pointer_location.location().cloned())
        })
    {
        // This pointer is associated with a render target, which could be used by multiple
        // cameras. We want to ensure we return all cameras with a matching target.
        for camera in camera_query
            .iter()
            .filter(|(_, _, _, cam_can_pick)| !settings.require_markers || *cam_can_pick)
            .map(|(entity, camera, _, _)| {
                (
                    entity,
                    camera.target.normalize(primary_window.single().ok()),
                )
            })
            .filter_map(|(entity, target)| Some(entity).zip(target))
            .filter(|(_entity, target)| target == &pointer_location.target)
            .map(|(cam_entity, _target)| cam_entity)
        {
            let Ok((_, camera_data, _, _)) = camera_query.get(camera) else {
                continue;
            };
            let mut pointer_pos =
                pointer_location.position * camera_data.target_scaling_factor().unwrap_or(1.);
            if let Some(viewport) = camera_data.physical_viewport_rect() {
                pointer_pos -= viewport.min.as_vec2();
            }
            pointer_pos_by_camera
                .entry(camera)
                .or_default()
                .insert(pointer_id, pointer_pos);
        }
    }

    // The list of node entities hovered for each (camera, pointer) combo
    let mut hit_nodes = HashMap::<(Entity, PointerId), Vec<(Entity, Vec2)>>::default();

    // prepare an iterator that contains all the nodes that have the cursor in their rect,
    // from the top node to the bottom one. this will also reset the interaction to `None`
    // for all nodes encountered that are no longer hovered.
    for node_entity in ui_stack
        .uinodes
        .iter()
        // reverse the iterator to traverse the tree from closest nodes to furthest
        .rev()
    {
        let Ok(node) = node_query.get(*node_entity) else {
            continue;
        };

        if settings.require_markers && node.pickable.is_none() {
            continue;
        }

        // Nodes that are not rendered should not be interactable
        if node
            .inherited_visibility
            .map(|inherited_visibility| inherited_visibility.get())
            != Some(true)
        {
            continue;
        }
        let Some(camera_entity) = node.target_camera.camera() else {
            continue;
        };

        let node_rect = Rect::from_center_size(
            node.global_transform.translation().truncate(),
            node.node.size(),
        );

        // Nodes with Display::None have a (0., 0.) logical rect and can be ignored
        if node_rect.size() == Vec2::ZERO {
            continue;
        }

        // Intersect with the calculated clip rect to find the bounds of the visible region of the node
        let visible_rect = node
            .calculated_clip
            .map(|clip| node_rect.intersect(clip.clip))
            .unwrap_or(node_rect);

        let pointers_on_this_cam = pointer_pos_by_camera.get(&camera_entity);

        // The mouse position relative to the node
        // (0., 0.) is the top-left corner, (1., 1.) is the bottom-right corner
        // Coordinates are relative to the entire node, not just the visible region.
        for (pointer_id, cursor_position) in pointers_on_this_cam.iter().flat_map(|h| h.iter()) {
            let relative_cursor_position = (*cursor_position - node_rect.min) / node_rect.size();

            if visible_rect
                .normalize(node_rect)
                .contains(relative_cursor_position)
                && pick_rounded_rect(
                    *cursor_position - node_rect.center(),
                    node_rect.size(),
                    node.node.border_radius,
                )
            {
                hit_nodes
                    .entry((camera_entity, *pointer_id))
                    .or_default()
                    .push((*node_entity, relative_cursor_position));
            }
        }
    }

    for ((camera, pointer), hovered) in hit_nodes.iter() {
        // As soon as a node with a `Block` focus policy is detected, the iteration will stop on it
        // because it "captures" the interaction.
        let mut picks = Vec::new();
        let mut depth = 0.0;

        for (hovered_node, position) in hovered {
            let node = node_query.get(*hovered_node).unwrap();

            let Some(camera_entity) = node.target_camera.camera() else {
                continue;
            };

            picks.push((
                node.entity,
                HitData::new(camera_entity, depth, Some(position.extend(0.0)), None),
            ));

            if let Some(pickable) = node.pickable {
                // If an entity has a `Pickable` component, we will use that as the source of truth.
                if pickable.should_block_lower {
                    break;
                }
            } else {
                // If the `Pickable` component doesn't exist, default behavior is to block.
                break;
            }

            depth += 0.00001; // keep depth near 0 for precision
        }

        let order = camera_query
            .get(*camera)
            .map(|(_, cam, _, _)| cam.order)
            .unwrap_or_default() as f32
            + 0.5; // bevy ui can run on any camera, it's a special case

        output.write(PointerHits::new(*pointer, picks, order));
    }
}
