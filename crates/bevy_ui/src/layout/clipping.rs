use crate::{
    layout_tree::ComputedLayout, ui_transform::UiGlobalTransform, CalculatedClip, Display,
    FixedNode, GhostNode, Node, OverrideClip, UiTreeChanged,
};

use super::ComputedNode;
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    query::{Has, With, Without},
    system::{Commands, Query},
    world::Ref,
};

/// Updates clipping for all nodes
pub fn update_clipping_system(
    mut commands: Commands,
    root_nodes: Query<Entity, (With<Node>, Without<ChildOf>)>,
    fixed_nodes_query: Query<(Entity, Has<GhostNode>), (With<FixedNode>, With<ChildOf>)>,
    mut node_query: Query<(
        &Node,
        &ComputedNode,
        &ComputedLayout,
        &UiGlobalTransform,
        Option<&mut CalculatedClip>,
        Has<OverrideClip>,
        Has<FixedNode>,
        Has<GhostNode>,
        Ref<UiTreeChanged>,
    )>,
    ui_children: Query<&Children, With<Node>>,
) {
    for root_node in root_nodes.iter().chain(
        fixed_nodes_query
            .iter()
            .filter_map(|(entity, is_ghost)| (!is_ghost).then_some(entity)),
    ) {
        update_clipping(
            &mut commands,
            &ui_children,
            &mut node_query,
            root_node,
            None,
            false,
            true,
        );
    }
}

// Needs more tests
fn update_clipping(
    commands: &mut Commands,
    ui_children: &Query<&Children, With<Node>>,
    node_query: &mut Query<(
        &Node,
        &ComputedNode,
        &ComputedLayout,
        &UiGlobalTransform,
        Option<&mut CalculatedClip>,
        Has<OverrideClip>,
        Has<FixedNode>,
        Has<GhostNode>,
        Ref<UiTreeChanged>,
    )>,
    entity: Entity,
    mut maybe_inherited_clip: Option<CalculatedClip>,
    force_update: bool,
    is_root: bool,
) {
    let Ok((
        node,
        computed_node,
        computed_layout,
        transform,
        maybe_calculated_clip,
        has_override_clip,
        has_fixed_node,
        has_ghost_node,
        tree_changed,
    )) = node_query.get_mut(entity)
    else {
        return;
    };

    // `FixedNode` is ignored if a root or `GhostNode`
    if has_fixed_node && !has_ghost_node && !is_root {
        return;
    }

    // If `OverrideClip` or `Node::override` was changed, `tree_changed.is_changed` should be `true``
    if !force_update && !tree_changed.is_changed() {
        return;
    }

    if !has_ghost_node
        && !force_update
        && !computed_layout.layout_changed()
        && !computed_layout.subtree_dirty()
    {
        return;
    }

    // If the UI node entity has an `OverrideClip`, discard any inherited clip rect
    if has_override_clip {
        maybe_inherited_clip = None;
    }

    // If `display` is None, clip the entire node and all its descendants.
    if !has_ghost_node && node.display == Display::None {
        maybe_inherited_clip = Some(CalculatedClip::FullyClipped);
    }

    // Update this node's CalculatedClip component
    if let Some(mut calculated_clip) = maybe_calculated_clip {
        if let Some(inherited_clip) = maybe_inherited_clip.as_ref() {
            // Replace the previous calculated clip with the inherited clipping rect
            if *calculated_clip != *inherited_clip {
                *calculated_clip = inherited_clip.clone();
            }
        } else {
            // No inherited clipping rect, remove the component
            commands.entity(entity).remove::<CalculatedClip>();
        }
    } else if let Some(inherited_clip) = maybe_inherited_clip.as_ref() {
        // No previous calculated clip, add a new CalculatedClip component with the inherited clipping rect
        commands.entity(entity).try_insert(inherited_clip.clone());
    }

    // Calculate new clip rectangle for children nodes
    let children_clip = if has_ghost_node {
        maybe_inherited_clip
    } else if maybe_inherited_clip
        .as_ref()
        .is_some_and(CalculatedClip::is_fully_clipped)
        || node.overflow.is_visible()
    {
        // The current node doesn't clip, propagate the optional inherited clipping rect to any children
        maybe_inherited_clip
    } else if let Some(clip_from_world) = transform.try_inverse() {
        let mut clip = maybe_inherited_clip.unwrap_or_default();
        clip.push_rect(
            computed_node.resolve_clip_rect(node.overflow, node.overflow_clip_margin),
            clip_from_world,
        );
        Some(clip)
    } else {
        Some(CalculatedClip::FullyClipped)
    };

    let propagated_force_update = force_update
        || tree_changed.is_changed()
        || computed_layout.layout_changed()
        || computed_layout.self_dirty();
    for &child in ui_children.get(entity).into_iter().flatten() {
        update_clipping(
            commands,
            ui_children,
            node_query,
            child,
            children_clip.clone(),
            propagated_force_update,
            false,
        );
    }
}
