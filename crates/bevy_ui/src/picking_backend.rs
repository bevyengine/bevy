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
//! - `bevy_ui` can render on any camera with a flag, it is special, and is not tied to a particular
//!   camera.
//! - To correctly sort picks, the order of `bevy_ui` is set to be the camera order plus 0.5.
//! - The `position` reported in `HitData` is normalized relative to the node, with
//!   `(-0.5, -0.5, 0.)` at the top left and `(0.5, 0.5, 0.)` in the bottom right. Coordinates are
//!   relative to the entire node, not just the visible region. This backend does not provide a `normal`.

#![deny(missing_docs)]

use crate::{clip_check_recursive, prelude::*, ui_transform::UiGlobalTransform, UiStack};
use bevy_app::prelude::*;
use bevy_camera::{visibility::InheritedVisibility, Camera, RenderTarget};
use bevy_ecs::{prelude::*, query::QueryData};
use bevy_math::Vec2;
use bevy_platform::collections::HashMap;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_text::{ComputedTextBlock, TextLayoutInfo};
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
            .add_systems(PreUpdate, ui_picking.in_set(PickingSystems::Backend));
    }
}

/// Main query from bevy's `ui_focus_system`
#[derive(QueryData)]
#[query_data(mutable)]
pub struct NodeQuery {
    entity: Entity,
    node: &'static ComputedNode,
    transform: &'static UiGlobalTransform,
    pickable: Option<&'static Pickable>,
    inherited_visibility: Option<&'static InheritedVisibility>,
    target_camera: &'static ComputedUiTargetCamera,
    text_node: Option<(&'static TextLayoutInfo, &'static ComputedTextBlock)>,
}

/// Computes the UI node entities under each pointer.
///
/// Bevy's [`UiStack`] orders all nodes in the order they will be rendered, which is the same order
/// we need for determining picking.
pub fn ui_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    camera_query: Query<(Entity, &Camera, &RenderTarget, Has<UiPickingCamera>)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    settings: Res<UiPickingSettings>,
    ui_stack: Res<UiStack>,
    node_query: Query<NodeQuery>,
    mut output: MessageWriter<PointerHits>,
    clipping_query: Query<(&ComputedNode, &UiGlobalTransform, &Node)>,
    child_of_query: Query<&ChildOf, Without<OverrideClip>>,
    pickable_query: Query<&Pickable>,
) {
    // Map from each camera to its active pointers and their positions in viewport space
    let mut pointer_pos_by_camera = HashMap::<Entity, HashMap<PointerId, Vec2>>::default();

    for (pointer_id, pointer_location) in
        pointers.iter().filter_map(|(pointer, pointer_location)| {
            Some(*pointer).zip(pointer_location.location().cloned())
        })
    {
        // This pointer is associated with a render target, which could be used by multiple
        // cameras. We want to ensure we return all cameras with a matching target.
        for (entity, camera, _, _) in
            camera_query
                .iter()
                .filter(|(_, _, render_target, cam_can_pick)| {
                    (!settings.require_markers || *cam_can_pick)
                        && render_target
                            .normalize(primary_window.single().ok(), None)
                            .is_some_and(|target| target == pointer_location.target)
                })
        {
            let mut pointer_pos =
                pointer_location.position * camera.target_scaling_factor().unwrap_or(1.);
            if let Some(viewport) = camera.physical_viewport_rect() {
                if !viewport.as_rect().contains(pointer_pos) {
                    // The pointer is outside the viewport, skip it
                    continue;
                }
                pointer_pos -= viewport.min.as_vec2();
            }
            pointer_pos_by_camera
                .entry(entity)
                .or_default()
                .insert(pointer_id, pointer_pos);
        }
    }

    // The list of node entities hovered for each (camera, pointer) combo
    let mut hit_nodes =
        HashMap::<(Entity, PointerId), Vec<(Entity, Entity, Option<Pickable>, Vec2)>>::default();

    // prepare an iterator that contains all the nodes that have the cursor in their rect,
    // from the top node to the bottom one. this will also reset the interaction to `None`
    // for all nodes encountered that are no longer hovered.
    // Reverse the iterator to traverse the tree from closest slice to furthest
    for uinodes in ui_stack
        .partition
        .iter()
        .rev()
        .map(|range| &ui_stack.uinodes[range.clone()])
    {
        // Retrieve the first node and resolve its camera target.
        // Only need to do this once per slice, as all the nodes in the same slice share the same camera.
        let Ok(uinode) = node_query.get(uinodes[0]) else {
            continue;
        };

        let Some(camera_entity) = uinode.target_camera.get() else {
            continue;
        };

        let Some(pointers_on_this_cam) = pointer_pos_by_camera.get(&camera_entity) else {
            continue;
        };

        // Reverse the iterator to traverse the tree from closest nodes to furthest
        for node_entity in uinodes.iter().rev().cloned() {
            let Ok(node) = node_query.get(node_entity) else {
                continue;
            };

            // Nodes with Display::None have a (0., 0.) logical rect and can be ignored
            if node.node.size() == Vec2::ZERO {
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

            // If this is a text node, need to do this check per section.
            if node.text_node.is_none() && settings.require_markers && node.pickable.is_none() {
                continue;
            }

            // Find the normalized cursor position relative to the node.
            // (±0., 0.) is the center with the corners at points (±0.5, ±0.5).
            // Coordinates are relative to the entire node, not just the visible region.
            for (pointer_id, cursor_position) in pointers_on_this_cam.iter() {
                if let Some((text_layout_info, text_block)) = node.text_node {
                    if let Some(text_entity) = pick_ui_text_section(
                        node.node,
                        node.transform,
                        *cursor_position,
                        text_layout_info,
                        text_block,
                    ) && clip_check_recursive(
                        *cursor_position,
                        node_entity,
                        &clipping_query,
                        &child_of_query,
                    ) {
                        if settings.require_markers && !pickable_query.contains(text_entity) {
                            continue;
                        }

                        hit_nodes
                            .entry((camera_entity, *pointer_id))
                            .or_default()
                            .push((
                                text_entity,
                                camera_entity,
                                node.pickable.cloned(),
                                node.transform.inverse().transform_point2(*cursor_position)
                                    / node.node.size(),
                            ));
                    }
                } else if node.node.contains_point(*node.transform, *cursor_position)
                    && clip_check_recursive(
                        *cursor_position,
                        node_entity,
                        &clipping_query,
                        &child_of_query,
                    )
                {
                    hit_nodes
                        .entry((camera_entity, *pointer_id))
                        .or_default()
                        .push((
                            node_entity,
                            camera_entity,
                            node.pickable.cloned(),
                            node.transform.inverse().transform_point2(*cursor_position)
                                / node.node.size(),
                        ));
                }
            }
        }
    }

    for ((camera, pointer), hovered) in hit_nodes.iter() {
        // As soon as a node with a `Block` focus policy is detected, the iteration will stop on it
        // because it "captures" the interaction.
        let mut picks = Vec::new();
        let mut depth = 0.0;

        for (hovered_node, camera_entity, pickable, position) in hovered {
            picks.push((
                *hovered_node,
                HitData::new(*camera_entity, depth, Some(position.extend(0.0)), None),
            ));

            if let Some(pickable) = pickable {
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

fn pick_ui_text_section(
    uinode: &ComputedNode,
    global_transform: &UiGlobalTransform,
    point: Vec2,
    text_layout_info: &TextLayoutInfo,
    text_block: &ComputedTextBlock,
) -> Option<Entity> {
    let local_point = global_transform
        .try_inverse()
        .map(|transform| transform.transform_point2(point) + 0.5 * uinode.size())?;

    for run in text_layout_info.run_geometry.iter() {
        if run.bounds.contains(local_point) {
            return text_block.entities().get(run.span_index).map(|e| e.entity);
        }
    }
    None
}
