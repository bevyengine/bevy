//! This module contains the systems that update the stored UI nodes stack

use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;

use crate::{Node, ZIndex};

/// The current UI stack, which contains all UI nodes ordered by their depth (back-to-front).
///
/// The first entry is the furthest node from the camera and is the first one to get rendered
/// while the last entry is the first node to receive interactions.
#[derive(Debug, Resource, Default)]
pub struct UiStack {
    /// List of UI nodes ordered from back-to-front
    pub uinodes: Vec<Entity>,
}

/// Generates the render stack for UI nodes.
///
/// First generate a UI node tree (`StackingContext`) based on z-index.
/// Then flatten that tree into back-to-front ordered `UiStack`.
pub fn ui_stack_system(
    mut ui_stack: ResMut<UiStack>,
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    node_query: Query<Option<&Children>, With<Node>>,
    zindex_query: Query<&ZIndex>,
) {
    ui_stack.uinodes.clear();
    let uinodes = &mut ui_stack.uinodes;

    fn update_uistack_recursively(
        entity: Entity,
        uinodes: &mut Vec<Entity>,
        node_query: &Query<Option<&Children>, With<Node>>,
        zindex_query: &Query<&ZIndex>,
    ) {
        let Ok(children) = node_query.get(entity) else {
            return;
        };

        uinodes.push(entity);

        if let Some(children) = children {
            let mut z_children: Vec<(Entity, i32)> = children
                .iter()
                .map(|&child_id| {
                    (
                        child_id,
                        match zindex_query.get(child_id) {
                            Ok(ZIndex(z)) => *z,
                            _ => 0,
                        },
                    )
                })
                .collect();
            z_children.sort_by_key(|k| k.1);
            for (child_id, _) in z_children {
                update_uistack_recursively(child_id, uinodes, node_query, zindex_query)
            }
        }
    }

    for entity in &root_node_query {
        update_uistack_recursively(entity, uinodes, &node_query, &zindex_query)
    }
}
