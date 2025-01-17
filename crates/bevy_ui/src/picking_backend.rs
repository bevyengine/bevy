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

#![deny(missing_docs)]

use crate::{focus::pick_rounded_rect, prelude::*, UiStack};
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, query::QueryData, system::SystemParam};
use bevy_math::{Rect, Vec2, Vec3Swizzles};
use bevy_render::prelude::*;
use bevy_transform::prelude::*;
use bevy_utils::HashMap;
use bevy_window::PrimaryWindow;

use bevy_picking::{
    backend::prelude::*,
    pointer::{Location, PointerAction, PointerInput},
};
use thiserror::Error;

/// A plugin that adds picking support for UI nodes.
#[derive(Clone)]
pub struct UiPickingPlugin;
impl Plugin for UiPickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, ui_picking.in_set(PickSet::Backend));
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
    view_visibility: Option<&'static ViewVisibility>,
    target_camera: Option<&'static TargetCamera>,
}

/// Computes the UI node entities under each pointer.
///
/// Bevy's [`UiStack`] orders all nodes in the order they will be rendered, which is the same order
/// we need for determining picking.
///
/// Like all picking backends, this system reads the [`PointerId`] and [`PointerLocation`] components,
/// and produces [`PointerHits`] events.
pub fn ui_picking(
    pointers: Query<(&PointerId, &PointerLocation)>,
    camera_query: Query<(Entity, &Camera, Has<IsDefaultUiCamera>)>,
    default_ui_camera: DefaultUiCamera,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    ui_stack: Res<UiStack>,
    node_query: Query<NodeQuery>,
    mut output: EventWriter<PointerHits>,
) {
    // For each camera, the pointer and its position
    let mut pointer_pos_by_camera = HashMap::<Entity, HashMap<PointerId, Vec2>>::default();

    let default_camera_entity = default_ui_camera.get();

    for (pointer_id, pointer_location) in
        pointers.iter().filter_map(|(pointer, pointer_location)| {
            Some(*pointer).zip(pointer_location.location().cloned())
        })
    {
        // This pointer is associated with a render target, which could be used by multiple
        // cameras. We want to ensure we return all cameras with a matching target.
        for camera in camera_query
            .iter()
            .map(|(entity, camera, _)| {
                (
                    entity,
                    camera.target.normalize(primary_window.get_single().ok()),
                )
            })
            .filter_map(|(entity, target)| Some(entity).zip(target))
            .filter(|(_entity, target)| target == &pointer_location.target)
            .map(|(cam_entity, _target)| cam_entity)
        {
            let Ok((_, camera_data, _)) = camera_query.get(camera) else {
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
    let mut hit_nodes = HashMap::<(Entity, PointerId), Vec<Entity>>::default();

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

        // Nodes that are not rendered should not be interactable
        if node
            .view_visibility
            .map(|view_visibility| view_visibility.get())
            != Some(true)
        {
            continue;
        }
        let Some(camera_entity) = node
            .target_camera
            .map(TargetCamera::entity)
            .or(default_camera_entity)
        else {
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
                    .push(*node_entity);
            }
        }
    }

    for ((camera, pointer), hovered_nodes) in hit_nodes.iter() {
        // As soon as a node with a `Block` focus policy is detected, the iteration will stop on it
        // because it "captures" the interaction.
        let mut picks = Vec::new();
        let mut depth = 0.0;

        for node in node_query.iter_many(hovered_nodes) {
            let Some(camera_entity) = node
                .target_camera
                .map(TargetCamera::entity)
                .or(default_camera_entity)
            else {
                continue;
            };

            picks.push((node.entity, HitData::new(camera_entity, depth, None, None)));

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
            .map(|(_, cam, _)| cam.order)
            .unwrap_or_default() as f32
            + 0.5; // bevy ui can run on any camera, it's a special case

        output.send(PointerHits::new(*pointer, picks, order));
    }
}

/// A [`SystemParam`] for realistically simulating/mocking pointer events on UI nodes.
#[derive(SystemParam)]
pub struct EmulateNodePointerEvents<'w, 's> {
    /// Looks up information about the node that the pointer event should be simulated on.
    pub node_query: Query<'w, 's, (&'static GlobalTransform, Option<&'static TargetCamera>)>,
    /// Tries to find the default UI camera
    pub default_ui_camera: DefaultUiCamera<'w, 's>,
    /// Tries to find a primary window entity.
    pub primary_window_query: Query<'w, 's, Entity, With<PrimaryWindow>>,
    /// Looks up the required camera information.
    pub camera_query: Query<'w, 's, &'static Camera>,
    /// Writes the pointer events to the world.
    pub pointer_input_events: EventWriter<'w, PointerInput>,
}

impl<'w, 's> EmulateNodePointerEvents<'w, 's> {
    /// Simulate a [`Pointer`](bevy_picking::events::Pointer) event,
    /// at the origin of the provided UI node entity.
    ///
    /// The entity that represents the [`PointerId`] provided should already exist,
    /// as this method does not create it.
    ///
    /// Under the hood, this generates [`PointerInput`] events,
    /// which is read by the [`PointerInput::receive`] system to modify existing pointer entities,
    /// and ultimately then processed into UI events by the [`ui_picking`] system.
    ///
    /// When using [`UiPlugin`](crate::UiPlugin), that system runs in the [`PreUpdate`] schedule,
    /// under the [`PickSet::Backend`] set.
    /// To ensure that these events are seen at the right time,
    /// you should generally call this method in systems scheduled during [`First`],
    /// as part of the [`PickSet::Input`] system set.
    ///
    /// # Warning
    ///
    /// If the node is not pickable, or is blocked by a higher node,
    /// these events may not have any effect, even if sent correctly!
    pub fn emulate_pointer(
        &mut self,
        pointer_id: PointerId,
        pointer_action: PointerAction,
        entity: Entity,
    ) -> Result<(), SimulatedNodePointerError> {
        // Look up the node we're trying to send a pointer event to
        let Ok((global_transform, maybe_target_camera)) = self.node_query.get(entity) else {
            return Err(SimulatedNodePointerError::NodeNotFound(entity));
        };

        // Figure out which camera this node is associated with
        let camera_entity = match maybe_target_camera {
            Some(explicit_target_camera) => explicit_target_camera.entity(),
            // Fall back to the default UI camera
            None => match self.default_ui_camera.get() {
                Some(default_camera_entity) => default_camera_entity,
                None => return Err(SimulatedNodePointerError::NoCameraFound),
            },
        };

        // Find the primary window, needed to normalize the render target
        // If we find 0 or 2+ primary windows, treat it as if none were found
        let maybe_primary_window_entity = self.primary_window_query.get_single().ok();

        // Generate the correct render target for the pointer
        let Ok(camera) = self.camera_query.get(camera_entity) else {
            return Err(SimulatedNodePointerError::NoCameraFound);
        };

        let Some(target) = camera.target.normalize(maybe_primary_window_entity) else {
            return Err(SimulatedNodePointerError::CouldNotComputeRenderTarget);
        };

        // Calculate the pointer position in the render target
        // For UI nodes, their final position is stored on their global transform,
        // in pixels, with the origin at the top-left corner of the camera's viewport.
        let position = global_transform.translation().xy();

        let pointer_location = Location { target, position };

        self.pointer_input_events.send(PointerInput {
            pointer_id,
            location: pointer_location,
            action: pointer_action,
        });

        Ok(())
    }
}

/// An error returned by [`EmulateNodePointerEvents`].
#[derive(Debug, PartialEq, Clone, Error)]
pub enum SimulatedNodePointerError {
    /// The entity provided could not be found.
    ///
    /// It must have a [`GlobalTransform`] component,
    /// and should have a [`Node`] component.
    #[error("The entity {0:?} could not be found.")]
    NodeNotFound(Entity),
    /// The camera associated with the node could not be found.
    ///
    /// Did you forget to spawn a camera entity with the [`Camera`] component?
    ///
    /// The [`TargetCamera`] component can be used to associate a camera with a node,
    /// but if it is not present, the [`DefaultUiCamera`] will be used.
    #[error("No camera could be found for the node.")]
    NoCameraFound,
    /// The [`NormalizedRenderTarget`](bevy_render::camera::NormalizedRenderTarget) could not be computed.
    #[error("Could not compute the normalized render target for the camera.")]
    CouldNotComputeRenderTarget,
}
