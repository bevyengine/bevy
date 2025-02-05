//! This module contains systems that update the UI when something changes

use crate::{
    experimental::{UiChildren, UiRootNodes},
    CalculatedClip, ComputedNodeScaleFactor, ComputedNodeTargetCamera, ComputedNodeTargetSize,
    DefaultUiCamera, Display, Node, OverflowAxis, UiScale, UiTargetCamera,
};

use super::ComputedNode;
use bevy_asset::Assets;
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::{hash_set::EntityHashSet, Entity},
    hierarchy::ChildOf,
    query::{Changed, With},
    system::{Commands, Local, Query, Res},
};
use bevy_image::Image;
use bevy_math::{Rect, UVec2};
use bevy_render::camera::{Camera, ManualTextureViews};
use bevy_sprite::BorderRect;
use bevy_transform::components::GlobalTransform;
use bevy_window::{PrimaryWindow, Window};

/// Updates clipping for all nodes
pub fn update_clipping_system(
    mut commands: Commands,
    root_nodes: UiRootNodes,
    mut node_query: Query<(
        &Node,
        &ComputedNode,
        &GlobalTransform,
        Option<&mut CalculatedClip>,
    )>,
    ui_children: UiChildren,
) {
    for root_node in root_nodes.iter() {
        update_clipping(
            &mut commands,
            &ui_children,
            &mut node_query,
            root_node,
            None,
        );
    }
}

fn update_clipping(
    commands: &mut Commands,
    ui_children: &UiChildren,
    node_query: &mut Query<(
        &Node,
        &ComputedNode,
        &GlobalTransform,
        Option<&mut CalculatedClip>,
    )>,
    entity: Entity,
    mut maybe_inherited_clip: Option<Rect>,
) {
    let Ok((node, computed_node, global_transform, maybe_calculated_clip)) =
        node_query.get_mut(entity)
    else {
        return;
    };

    // If `display` is None, clip the entire node and all its descendants by replacing the inherited clip with a default rect (which is empty)
    if node.display == Display::None {
        maybe_inherited_clip = Some(Rect::default());
    }

    // Update this node's CalculatedClip component
    if let Some(mut calculated_clip) = maybe_calculated_clip {
        if let Some(inherited_clip) = maybe_inherited_clip {
            // Replace the previous calculated clip with the inherited clipping rect
            if calculated_clip.clip != inherited_clip {
                *calculated_clip = CalculatedClip {
                    clip: inherited_clip,
                };
            }
        } else {
            // No inherited clipping rect, remove the component
            commands.entity(entity).remove::<CalculatedClip>();
        }
    } else if let Some(inherited_clip) = maybe_inherited_clip {
        // No previous calculated clip, add a new CalculatedClip component with the inherited clipping rect
        commands.entity(entity).try_insert(CalculatedClip {
            clip: inherited_clip,
        });
    }

    // Calculate new clip rectangle for children nodes
    let children_clip = if node.overflow.is_visible() {
        // When `Visible`, children might be visible even when they are outside
        // the current node's boundaries. In this case they inherit the current
        // node's parent clip. If an ancestor is set as `Hidden`, that clip will
        // be used; otherwise this will be `None`.
        maybe_inherited_clip
    } else {
        // If `maybe_inherited_clip` is `Some`, use the intersection between
        // current node's clip and the inherited clip. This handles the case
        // of nested `Overflow::Hidden` nodes. If parent `clip` is not
        // defined, use the current node's clip.

        let mut clip_rect = Rect::from_center_size(
            global_transform.translation().truncate(),
            computed_node.size(),
        );

        // Content isn't clipped at the edges of the node but at the edges of the region specified by [`Node::overflow_clip_margin`].
        //
        // `clip_inset` should always fit inside `node_rect`.
        // Even if `clip_inset` were to overflow, we won't return a degenerate result as `Rect::intersect` will clamp the intersection, leaving it empty.
        let clip_inset = match node.overflow_clip_margin.visual_box {
            crate::OverflowClipBox::BorderBox => BorderRect::ZERO,
            crate::OverflowClipBox::ContentBox => computed_node.content_inset(),
            crate::OverflowClipBox::PaddingBox => computed_node.border(),
        };

        clip_rect.min.x += clip_inset.left;
        clip_rect.min.y += clip_inset.top;
        clip_rect.max.x -= clip_inset.right;
        clip_rect.max.y -= clip_inset.bottom;

        clip_rect = clip_rect
            .inflate(node.overflow_clip_margin.margin.max(0.) / computed_node.inverse_scale_factor);

        if node.overflow.x == OverflowAxis::Visible {
            clip_rect.min.x = -f32::INFINITY;
            clip_rect.max.x = f32::INFINITY;
        }
        if node.overflow.y == OverflowAxis::Visible {
            clip_rect.min.y = -f32::INFINITY;
            clip_rect.max.y = f32::INFINITY;
        }
        Some(maybe_inherited_clip.map_or(clip_rect, |c| c.intersect(clip_rect)))
    };

    for child in ui_children.iter_ui_children(entity) {
        update_clipping(commands, ui_children, node_query, child, children_clip);
    }
}

pub fn resolve_target_camera_system(
    default_ui_camera: DefaultUiCamera,
    ui_scale: Res<UiScale>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    images: Res<Assets<Image>>,
    camera_query: Query<&Camera>,
    window_query: Query<(Entity, &Window)>,
    target_camera_query: Query<&UiTargetCamera>,
    ui_root_nodes: UiRootNodes,
    mut context_query: Query<(
        &mut ComputedNodeScaleFactor,
        &mut ComputedNodeTargetSize,
        &mut ComputedNodeTargetCamera,
    )>,
    manual_texture_views: Res<ManualTextureViews>,
    ui_children: UiChildren,
    reparented_nodes: Query<(Entity, &ChildOf), (Changed<ChildOf>, With<ComputedNodeTargetCamera>)>,
    mut visited: Local<EntityHashSet>,
) {
    visited.clear();
    let default_camera_entity = default_ui_camera.get();
    let primary_window = primary_window_query.get_single().ok();

    for root_entity in ui_root_nodes.iter() {
        let camera_entity = target_camera_query
            .get(root_entity)
            .ok()
            .map(UiTargetCamera::entity)
            .or(default_camera_entity)
            .unwrap_or(Entity::PLACEHOLDER);

        let (new_scale_factor, new_target_size) = camera_query
            .get(camera_entity)
            .ok()
            .and_then(|camera| camera.target.normalize(primary_window))
            .and_then(|normalized_render_target| {
                normalized_render_target.get_render_target_info(
                    window_query.iter(),
                    &images,
                    &manual_texture_views,
                )
            })
            .map(|info| (info.scale_factor, info.physical_size))
            .map(|(sf, r)| (sf * ui_scale.0, r))
            .unwrap_or((ui_scale.0, UVec2::ZERO));

        update_contexts_recursively(
            root_entity,
            new_scale_factor,
            new_target_size,
            camera_entity,
            &ui_children,
            &mut context_query,
            &mut visited,
        );
    }

    for (entity, child_of) in reparented_nodes.iter() {
        let Ok((new_scale_factor, new_target_size, camera_entity)) = context_query.get(child_of.0)
        else {
            continue;
        };

        update_contexts_recursively(
            entity,
            new_scale_factor.0,
            new_target_size.0,
            camera_entity.0,
            &ui_children,
            &mut context_query,
            &mut visited,
        );
    }
}

fn update_contexts_recursively(
    entity: Entity,
    scale_factor: f32,
    target_size: UVec2,
    camera: Entity,
    ui_children: &UiChildren,
    query: &mut Query<(
        &mut ComputedNodeScaleFactor,
        &mut ComputedNodeTargetSize,
        &mut ComputedNodeTargetCamera,
    )>,
    visited: &mut EntityHashSet,
) {
    if !visited.insert(entity) {
        return;
    }
    if query
        .get_mut(entity)
        .map(
            |(mut node_scale_factor, mut node_target_size, mut node_camera)| {
                node_scale_factor.set_if_neq(ComputedNodeScaleFactor(scale_factor))
                    | node_target_size.set_if_neq(ComputedNodeTargetSize(target_size))
                    | node_camera.set_if_neq(ComputedNodeTargetCamera(camera))
            },
        )
        .unwrap_or(false)
    {
        for child in ui_children.iter_ui_children(entity) {
            update_contexts_recursively(
                child,
                scale_factor,
                target_size,
                camera,
                ui_children,
                query,
                visited,
            );
        }
    }
}
