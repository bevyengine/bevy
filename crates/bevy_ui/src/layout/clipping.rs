use crate::{
    experimental::{UiChildren, UiRootNodes},
    ui_transform::UiGlobalTransform,
    CalculatedClip, ComputedNode, Display, FixedNode, Node, OverrideClip,
};
use bevy_ecs::{
    entity::Entity,
    query::Has,
    system::{Commands, Query},
};

/// Updates clipping for all nodes
pub fn update_clipping_system(
    mut commands: Commands,
    root_nodes: UiRootNodes,
    mut node_query: Query<(
        &Node,
        &ComputedNode,
        &UiGlobalTransform,
        Option<&mut CalculatedClip>,
        Has<OverrideClip>,
        Has<FixedNode>,
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
        &UiGlobalTransform,
        Option<&mut CalculatedClip>,
        Has<OverrideClip>,
        Has<FixedNode>,
    )>,
    entity: Entity,
    mut maybe_inherited_clip: Option<CalculatedClip>,
) {
    let Ok((
        node,
        computed_node,
        transform,
        maybe_calculated_clip,
        has_override_clip,
        has_fixed_node,
    )) = node_query.get_mut(entity)
    else {
        return;
    };

    // If the UI node entity has an `OverrideClip` or `FixedNode` component, discard any inherited clip rect
    if has_override_clip || has_fixed_node {
        maybe_inherited_clip = None;
    }

    // If `display` is None, clip the entire node and all its descendants.
    if node.display == Display::None {
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
    let children_clip = if maybe_inherited_clip
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

    for child in ui_children.iter_ui_children(entity) {
        update_clipping(
            commands,
            ui_children,
            node_query,
            child,
            children_clip.clone(),
        );
    }
}
