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
///
/// First generate a UI node tree (`StackingContext`) based on z-index.
/// Then flatten that tree into back-to-front ordered `UiStack`.
pub fn ui_stack_system(
    mut ui_stack: ResMut<UiStack>,
    root_node_query: Query<Entity, (With<Node>, Without<Parent>)>,
    zindex_query: Query<&ZIndex, With<Node>>,
    children_query: Query<&Children>,
) {
    // Generate `StackingContext` tree
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

    // Flatten `StackingContext` into `UiStack`
    ui_stack.uinodes.clear();
    ui_stack.uinodes.reserve(total_entry_count);
    fill_stack_recursively(&mut ui_stack.uinodes, &mut global_context);
}

/// Generate z-index based UI node tree
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
        // Reserve space for all children. In practice, some may not get pushed since
        // nodes with `ZIndex::Global` are pushed to the global (root) context.
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

    // The node will be added either to global/parent based on its z-index type: global/local.
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

/// Flatten `StackingContext` (z-index based UI node tree) into back-to-front entities list
fn fill_stack_recursively(result: &mut Vec<Entity>, stack: &mut StackingContext) {
    // Sort entries by ascending z_index, while ensuring that siblings
    // with the same local z_index will keep their ordering. This results
    // in `back-to-front` ordering, low z_index = back; high z_index = front.
    stack.entries.sort_by_key(|e| e.z_index);

    for entry in &mut stack.entries {
        // Parent node renders before/behind child nodes
        result.push(entry.entity);
        fill_stack_recursively(result, &mut entry.stack);
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        schedule::Schedule,
        system::{CommandQueue, Commands},
        world::World,
    };
    use bevy_hierarchy::BuildChildren;

    use crate::{Node, UiStack, ZIndex};

    use super::ui_stack_system;

    #[derive(Component, PartialEq, Debug, Clone)]
    struct Label(&'static str);

    fn node_with_zindex(name: &'static str, z_index: ZIndex) -> (Label, Node, ZIndex) {
        (Label(name), Node::default(), z_index)
    }

    fn node_without_zindex(name: &'static str) -> (Label, Node) {
        (Label(name), Node::default())
    }

    /// Tests the UI Stack system.
    ///
    /// This tests for siblings default ordering according to their insertion order, but it
    /// can't test the same thing for UI roots. UI roots having no parents, they do not have
    /// a stable ordering that we can test against. If we test it, it may pass now and start
    /// failing randomly in the future because of some unrelated `bevy_ecs` change.
    #[test]
    fn test_ui_stack_system() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        commands.spawn(node_with_zindex("0", ZIndex::Global(2)));

        commands
            .spawn(node_with_zindex("1", ZIndex::Local(1)))
            .with_children(|parent| {
                parent
                    .spawn(node_without_zindex("1-0"))
                    .with_children(|parent| {
                        parent.spawn(node_without_zindex("1-0-0"));
                        parent.spawn(node_without_zindex("1-0-1"));
                        parent.spawn(node_with_zindex("1-0-2", ZIndex::Local(-1)));
                    });
                parent.spawn(node_without_zindex("1-1"));
                parent
                    .spawn(node_with_zindex("1-2", ZIndex::Global(-1)))
                    .with_children(|parent| {
                        parent.spawn(node_without_zindex("1-2-0"));
                        parent.spawn(node_with_zindex("1-2-1", ZIndex::Global(-3)));
                        parent
                            .spawn(node_without_zindex("1-2-2"))
                            .with_children(|_| ());
                        parent.spawn(node_without_zindex("1-2-3"));
                    });
                parent.spawn(node_without_zindex("1-3"));
            });

        commands
            .spawn(node_without_zindex("2"))
            .with_children(|parent| {
                parent
                    .spawn(node_without_zindex("2-0"))
                    .with_children(|_parent| ());
                parent
                    .spawn(node_without_zindex("2-1"))
                    .with_children(|parent| {
                        parent.spawn(node_without_zindex("2-1-0"));
                    });
            });

        commands.spawn(node_with_zindex("3", ZIndex::Global(-2)));

        queue.apply(&mut world);

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);
        schedule.run(&mut world);

        let mut query = world.query::<&Label>();
        let ui_stack = world.resource::<UiStack>();
        let actual_result = ui_stack
            .uinodes
            .iter()
            .map(|entity| query.get(&world, *entity).unwrap().clone())
            .collect::<Vec<_>>();
        let expected_result = vec![
            (Label("1-2-1")), // ZIndex::Global(-3)
            (Label("3")),     // ZIndex::Global(-2)
            (Label("1-2")),   // ZIndex::Global(-1)
            (Label("1-2-0")),
            (Label("1-2-2")),
            (Label("1-2-3")),
            (Label("2")),
            (Label("2-0")),
            (Label("2-1")),
            (Label("2-1-0")),
            (Label("1")), // ZIndex::Local(1)
            (Label("1-0")),
            (Label("1-0-2")), // ZIndex::Local(-1)
            (Label("1-0-0")),
            (Label("1-0-1")),
            (Label("1-1")),
            (Label("1-3")),
            (Label("0")), // ZIndex::Global(2)
        ];
        assert_eq!(actual_result, expected_result);
    }
}
