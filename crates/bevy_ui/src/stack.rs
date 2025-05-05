//! This module contains the systems that update the stored UI nodes stack

use bevy_ecs::prelude::*;
use bevy_platform::collections::HashSet;

use crate::{
    experimental::{UiChildren, UiRootNodes},
    ComputedNode, GlobalZIndex, ZIndex,
};

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
pub(crate) struct ChildBufferCache {
    pub inner: Vec<Vec<(Entity, i32)>>,
}

impl ChildBufferCache {
    fn pop(&mut self) -> Vec<(Entity, i32)> {
        self.inner.pop().unwrap_or_default()
    }

    fn push(&mut self, vec: Vec<(Entity, i32)>) {
        self.inner.push(vec);
    }
}

/// Generates the render stack for UI nodes.
///
/// Create a list of root nodes from parentless entities and entities with a `GlobalZIndex` component.
/// Then build the `UiStack` from a walk of the existing layout trees starting from each root node,
/// filtering branches by `Without<GlobalZIndex>`so that we don't revisit nodes.
pub fn ui_stack_system(
    mut cache: Local<ChildBufferCache>,
    mut root_nodes: Local<Vec<(Entity, (i32, i32))>>,
    mut visited_root_nodes: Local<HashSet<Entity>>,
    mut ui_stack: ResMut<UiStack>,
    ui_root_nodes: UiRootNodes,
    root_node_query: Query<(Entity, Option<&GlobalZIndex>, Option<&ZIndex>)>,
    zindex_global_node_query: Query<(Entity, &GlobalZIndex, Option<&ZIndex>), With<ComputedNode>>,
    ui_children: UiChildren,
    zindex_query: Query<Option<&ZIndex>, (With<ComputedNode>, Without<GlobalZIndex>)>,
    mut update_query: Query<&mut ComputedNode>,
) {
    ui_stack.uinodes.clear();
    visited_root_nodes.clear();

    for (id, maybe_global_zindex, maybe_zindex) in root_node_query.iter_many(ui_root_nodes.iter()) {
        root_nodes.push((
            id,
            (
                maybe_global_zindex.map(|zindex| zindex.0).unwrap_or(0),
                maybe_zindex.map(|zindex| zindex.0).unwrap_or(0),
            ),
        ));
        visited_root_nodes.insert(id);
    }

    for (id, global_zindex, maybe_zindex) in zindex_global_node_query.iter() {
        if visited_root_nodes.contains(&id) {
            continue;
        }

        root_nodes.push((
            id,
            (
                global_zindex.0,
                maybe_zindex.map(|zindex| zindex.0).unwrap_or(0),
            ),
        ));
    }

    root_nodes.sort_by_key(|(_, z)| *z);

    for (root_entity, _) in root_nodes.drain(..) {
        update_uistack_recursive(
            &mut cache,
            root_entity,
            &ui_children,
            &zindex_query,
            &mut ui_stack.uinodes,
        );
    }

    for (i, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok(mut node) = update_query.get_mut(*entity) {
            node.bypass_change_detection().stack_index = i as u32;
        }
    }
}

fn update_uistack_recursive(
    cache: &mut ChildBufferCache,
    node_entity: Entity,
    ui_children: &UiChildren,
    zindex_query: &Query<Option<&ZIndex>, (With<ComputedNode>, Without<GlobalZIndex>)>,
    ui_stack: &mut Vec<Entity>,
) {
    ui_stack.push(node_entity);

    let mut child_buffer = cache.pop();
    child_buffer.extend(
        ui_children
            .iter_ui_children(node_entity)
            .filter_map(|child_entity| {
                zindex_query
                    .get(child_entity)
                    .ok()
                    .map(|zindex| (child_entity, zindex.map(|zindex| zindex.0).unwrap_or(0)))
            }),
    );
    child_buffer.sort_by_key(|k| k.1);
    for (child_entity, _) in child_buffer.drain(..) {
        update_uistack_recursive(cache, child_entity, ui_children, zindex_query, ui_stack);
    }
    cache.push(child_buffer);
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        schedule::Schedule,
        system::Commands,
        world::{CommandQueue, World},
    };

    use crate::{GlobalZIndex, Node, UiStack, ZIndex};

    use super::ui_stack_system;

    #[derive(Component, PartialEq, Debug, Clone)]
    struct Label(&'static str);

    fn node_with_global_and_local_zindex(
        name: &'static str,
        global_zindex: i32,
        local_zindex: i32,
    ) -> (Label, Node, GlobalZIndex, ZIndex) {
        (
            Label(name),
            Node::default(),
            GlobalZIndex(global_zindex),
            ZIndex(local_zindex),
        )
    }

    fn node_with_global_zindex(
        name: &'static str,
        global_zindex: i32,
    ) -> (Label, Node, GlobalZIndex) {
        (Label(name), Node::default(), GlobalZIndex(global_zindex))
    }

    fn node_with_zindex(name: &'static str, zindex: i32) -> (Label, Node, ZIndex) {
        (Label(name), Node::default(), ZIndex(zindex))
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
        commands.spawn(node_with_global_zindex("0", 2));

        commands
            .spawn(node_with_zindex("1", 1))
            .with_children(|parent| {
                parent
                    .spawn(node_without_zindex("1-0"))
                    .with_children(|parent| {
                        parent.spawn(node_without_zindex("1-0-0"));
                        parent.spawn(node_without_zindex("1-0-1"));
                        parent.spawn(node_with_zindex("1-0-2", -1));
                    });
                parent.spawn(node_without_zindex("1-1"));
                parent
                    .spawn(node_with_global_zindex("1-2", -1))
                    .with_children(|parent| {
                        parent.spawn(node_without_zindex("1-2-0"));
                        parent.spawn(node_with_global_zindex("1-2-1", -3));
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

        commands.spawn(node_with_global_zindex("3", -2));

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
            (Label("1-2-1")), // GlobalZIndex(-3)
            (Label("3")),     // GlobalZIndex(-2)
            (Label("1-2")),   // GlobalZIndex(-1)
            (Label("1-2-0")),
            (Label("1-2-2")),
            (Label("1-2-3")),
            (Label("2")),
            (Label("2-0")),
            (Label("2-1")),
            (Label("2-1-0")),
            (Label("1")), // ZIndex(1)
            (Label("1-0")),
            (Label("1-0-2")), // ZIndex(-1)
            (Label("1-0-0")),
            (Label("1-0-1")),
            (Label("1-1")),
            (Label("1-3")),
            (Label("0")), // GlobalZIndex(2)
        ];
        assert_eq!(actual_result, expected_result);
    }

    #[test]
    fn test_with_equal_global_zindex_zindex_decides_order() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        commands.spawn(node_with_global_and_local_zindex("0", -1, 1));
        commands.spawn(node_with_global_and_local_zindex("1", -1, 2));
        commands.spawn(node_with_global_and_local_zindex("2", 1, 3));
        commands.spawn(node_with_global_and_local_zindex("3", 1, -3));
        commands
            .spawn(node_without_zindex("4"))
            .with_children(|builder| {
                builder.spawn(node_with_global_and_local_zindex("5", 0, -1));
                builder.spawn(node_with_global_and_local_zindex("6", 0, 1));
                builder.spawn(node_with_global_and_local_zindex("7", -1, -1));
                builder.spawn(node_with_global_zindex("8", 1));
            });

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
            (Label("7")),
            (Label("0")),
            (Label("1")),
            (Label("5")),
            (Label("4")),
            (Label("6")),
            (Label("3")),
            (Label("8")),
            (Label("2")),
        ];

        assert_eq!(actual_result, expected_result);
    }
}
