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

use crate::{clip_check_recursive, prelude::*, ui_transform::UiGlobalTransform, UiStack};
use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_camera::{visibility::InheritedVisibility, Camera, RenderTarget};
use bevy_color::prelude::*;
use bevy_ecs::{prelude::*, query::QueryData};
use bevy_image::prelude::*;
use bevy_math::{Rect, Vec2};
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

/// An optional component that specifies how [`UiPickingPlugin`] should handle
/// picking for a UI node, in particular how it treats the transparent pixels of
/// an [`ImageNode`].
///
/// The picking mode can be set globally via
/// [`UiPickingSettings::picking_mode`], and overridden on a per-node basis by
/// adding this component to a UI node entity.
#[derive(Debug, Clone, Copy, Component, Reflect)]
#[reflect(Debug, Clone, Component)]
pub enum UiPickingMode {
    /// Even if a node is picked over a transparent pixel, it should still count
    /// as a hit. Only the bounding box of the node is considered.
    ///
    /// This is also the effective behavior for nodes without an [`ImageNode`],
    /// since they have no texture to sample. Such nodes are always treated as
    /// hits regardless of the mode.
    BoundingBox,
    /// Ignore any part of an [`ImageNode`] which has a lower alpha value than
    /// the threshold (inclusive). The threshold is given as an `f32`
    /// representing the alpha value in a Bevy [`Color`].
    AlphaThreshold(f32),
}

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
    /// Whether the backend should count transparent pixels of image nodes as
    /// part of the node for picking purposes, or whether it should use the
    /// bounding box of the node alone.
    ///
    /// This only affects nodes with an [`ImageNode`]. Nodes without a texture
    /// to sample are always treated as hits. It is the global default and can
    /// be overridden per-node with the [`UiPickingMode`] component.
    ///
    /// Defaults to an inclusive alpha threshold of 0.1.
    pub picking_mode: UiPickingMode,
}

impl Default for UiPickingSettings {
    fn default() -> Self {
        Self {
            require_markers: false,
            picking_mode: UiPickingMode::AlphaThreshold(0.1),
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
    image: Option<&'static ImageNode>,
    picking_mode: Option<&'static UiPickingMode>,
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
    images: Res<Assets<Image>>,
    texture_atlases: Res<Assets<TextureAtlasLayout>>,
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
                            .normalize(primary_window.single().ok())
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
                if node.node.contains_point(*node.transform, *cursor_position)
                    && clip_check_recursive(
                        *cursor_position,
                        node_entity,
                        &clipping_query,
                        &child_of_query,
                    )
                    && let Some(target) = node
                        .text_node
                        .and_then(|(text_layout_info, text_block)| {
                            pick_ui_text_section(
                                node.node,
                                node.transform,
                                *cursor_position,
                                text_layout_info,
                                text_block,
                            )
                            .filter(|&text_entity| {
                                !settings.require_markers || pickable_query.contains(text_entity)
                            })
                        })
                        .or_else(|| {
                            (!settings.require_markers || node.pickable.is_some())
                                .then_some(node_entity)
                        })
                {
                    // Normalized cursor position relative to the node, with
                    // `(-0.5, -0.5)` at the top left and `(0.5, 0.5)` at the
                    // bottom right.
                    let relative_cursor_position =
                        node.transform.inverse().transform_point2(*cursor_position)
                            / node.node.size();

                    let picking_mode = node.picking_mode.copied().unwrap_or(settings.picking_mode);

                    let hit = match picking_mode {
                        UiPickingMode::BoundingBox => true,
                        UiPickingMode::AlphaThreshold(cutoff) => match node.image {
                            Some(image_node) => image_node_contains_opaque_pixel(
                                image_node,
                                relative_cursor_position + 0.5, // convert to a `0..1` UV with `(0, 0)` at the top left
                                cutoff,
                                &images,
                                &texture_atlases,
                            ),
                            // Nodes without an image have no texture to sample,
                            // so they are always treated as hits.
                            None => true,
                        },
                    };

                    if hit {
                        hit_nodes
                            .entry((camera_entity, *pointer_id))
                            .or_default()
                            .push((
                                target,
                                camera_entity,
                                node.pickable.cloned(),
                                relative_cursor_position,
                            ));
                    }
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

/// Returns `true` if the pixel of `image_node`'s texture under `uv` (normalized
/// `0..1` with `(0, 0)` at the top left) has an alpha value greater than
/// `cutoff`.
///
/// Nodes whose image can't be sampled are treated as hits. This includes nodes
/// whose image asset isn't available (e.g., it is still loading or failed to
/// load) and nodes using a non-linear image mode such as
/// [`NodeImageMode::Sliced`] or [`NodeImageMode::Tiled`], which don't map
/// linearly from the node to the texture.
fn image_node_contains_opaque_pixel(
    image_node: &ImageNode,
    uv: Vec2,
    cutoff: f32,
    images: &Assets<Image>,
    texture_atlases: &Assets<TextureAtlasLayout>,
) -> bool {
    // Sliced and tiled image modes don't map linearly from the node to the
    // texture, so we can't easily sample the pixel under the cursor. Treat
    // these as hits.
    if image_node.image_mode.uses_slices() {
        return true;
    }

    let Some(image) = images.get(&image_node.image) else {
        // The image asset isn't available (e.g., it is still loading or failed
        // to load), so we can't inspect its pixels. Fall back to a bounding-box
        // hit rather than making the node unpickable.
        return true;
    };

    let image_size = image.size();

    let atlas_rect = image_node
        .texture_atlas
        .as_ref()
        .and_then(|atlas| atlas.texture_rect(texture_atlases))
        .map(|rect| rect.as_rect());
    let texture_rect = match (atlas_rect, image_node.rect) {
        (None, None) => Rect::new(0.0, 0.0, image_size.x as f32, image_size.y as f32),
        (None, Some(rect)) => rect,
        (Some(atlas_rect), None) => atlas_rect,
        (Some(atlas_rect), Some(mut rect)) => {
            // Make the node's rect relative to the atlas rect.
            rect.min += atlas_rect.min;
            rect.max += atlas_rect.min;
            rect
        }
    };

    let mut uv = uv;
    if image_node.flip_x {
        uv.x = 1.0 - uv.x;
    }
    if image_node.flip_y {
        uv.y = 1.0 - uv.y;
    }

    let texture_position = texture_rect.min + uv * texture_rect.size();

    let Ok(color) = image.get_color_at(texture_position.x as u32, texture_position.y as u32) else {
        // We don't know how to interpret the pixel.
        return false;
    };

    color.alpha() > cutoff
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
        .map(|transform| transform.transform_point2(point) - uinode.content_box().min)?;
    let section_index = text_layout_info
        .run_geometry
        .iter()
        .find(|run| run.bounds.contains(local_point))
        .map(|run| run.section_index)?;
    text_block
        .entities()
        .get(section_index as usize)
        .map(|e| e.entity)
}
