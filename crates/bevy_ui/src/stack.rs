//! This module contains the systems that update the stored UI nodes stack

use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;

use crate::{Node, ZIndex};

/// The current UI stack, which contains all UI nodes ordered by their depth.
///
/// The first entry is the furthest node from the camera and is the first one to get rendered
/// while the last entry is the first node to receive interactions.
#[derive(Debug, Resource, Default)]
pub struct UiStack {
    pub uinodes: Vec<Entity>,
}

#[derive(Default)]
struct StackingContext {
    pub entries: Vec<StackingContextEntry>,
}

struct StackingContextEntry {
    pub z_index: i32,
    pub entity: Entity,
    pub stack: StackingContext,
}

/// Generates the render stack for UI nodes.
pub fn ui_stack_system(
    mut ui_stack: ResMut<UiStack>,
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    zindex_query: Query<&ZIndex, With<Node>>,
    children_query: Query<&Children>,
) {
    let mut global_context = StackingContext::default();

    let mut total_entry_count: usize = 0;
    for entity in &root_node_query {
        insert_context_hierarchy(
            &zindex_query,
            &children_query,
            entity,
            &mut global_context,
            None,
            &mut total_entry_count,
        );
    }

    *ui_stack = UiStack {
        uinodes: Vec::<Entity>::with_capacity(total_entry_count),
    };

    fill_stack_recursively(&mut ui_stack.uinodes, &mut global_context);
}

fn insert_context_hierarchy(
    zindex_query: &Query<&ZIndex, With<Node>>,
    children_query: &Query<&Children>,
    entity: Entity,
    global_context: &mut StackingContext,
    parent_context: Option<&mut StackingContext>,
    total_entry_count: &mut usize,
) {
    let mut new_context = StackingContext::default();
    if let Ok(children) = children_query.get(entity) {
        // reserve space for all children. in practice, some may not get pushed.
        new_context.entries.reserve_exact(children.len());

        for entity in children {
            insert_context_hierarchy(
                zindex_query,
                children_query,
                *entity,
                global_context,
                Some(&mut new_context),
                total_entry_count,
            );
        }
    }

    let z_index = zindex_query.get(entity).unwrap_or(&ZIndex::Local(0));
    let (entity_context, z_index) = match z_index {
        ZIndex::Local(value) => (parent_context.unwrap_or(global_context), *value),
        ZIndex::Global(value) => (global_context, *value),
    };

    *total_entry_count += 1;
    entity_context.entries.push(StackingContextEntry {
        z_index,
        entity,
        stack: new_context,
    });
}

fn fill_stack_recursively(result: &mut Vec<Entity>, stack: &mut StackingContext) {
    // sort entries by ascending z_index, while ensuring that siblings
    // with the same local z_index will keep their ordering.
    stack.entries.sort_by_key(|e| e.z_index);

    for entry in &mut stack.entries {
        result.push(entry.entity);
        fill_stack_recursively(result, &mut entry.stack);
    }
}
