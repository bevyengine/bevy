//! This module contains the systems that update the stored UI nodes stack

use bevy_ecs::prelude::*;
use bevy_hierarchy::prelude::*;

use crate::{GlobalZIndex, Node, ZIndex};

/// The current UI stack, which contains all UI nodes ordered by their depth (back-to-front).
///
/// The first entry is the furthest node from the camera and is the first one to get rendered
/// while the last entry is the first node to receive interactions.
#[derive(Debug, Resource, Default)]
pub struct UiStack {
    /// List of UI nodes ordered from back-to-front
    pub uinodes: Vec<Entity>,
}

fn update_uistack_recursively(
    entity: Entity,
    uinodes: &mut Vec<Entity>,
    children_query: &Query<&Children>,
    zindex_query: &Query<Option<&ZIndex>, (With<Node>, Without<GlobalZIndex>)>,
) {
    uinodes.push(entity);

    if let Ok(children) = children_query.get(entity) {
        let mut z_children: Vec<(Entity, i32)> = children
            .iter()
            .filter_map(|entity| {
                zindex_query
                    .get(*entity)
                    .ok()
                    .map(|zindex| (*entity, zindex.map(|zindex| zindex.0).unwrap_or(0)))
            })
            .collect();
        z_children.sort_by_key(|k| k.1);
        for (child_id, _) in z_children {
            update_uistack_recursively(child_id, uinodes, children_query, zindex_query);
        }
    }
}

/// Generates the render stack for UI nodes.
pub fn ui_stack_system(
    mut ui_stack: ResMut<UiStack>,
    root_node_query: Query<
        (Entity, Option<&GlobalZIndex>, Option<&ZIndex>),
        (With<Node>, Without<Parent>),
    >,
    zindex_global_node_query: Query<
        (Entity, &GlobalZIndex, Option<&ZIndex>),
        (With<Node>, With<Parent>),
    >,
    children_query: Query<&Children>,
    zindex_query: Query<Option<&ZIndex>, (With<Node>, Without<GlobalZIndex>)>,
    mut update_query: Query<&mut Node>,
) {
    ui_stack.uinodes.clear();
    let uinodes = &mut ui_stack.uinodes;

    let global_nodes = zindex_global_node_query
        .iter()
        .map(|(id, global_zindex, maybe_zindex)| {
            (
                id,
                (
                    global_zindex.0,
                    maybe_zindex.map(|zindex| zindex.0).unwrap_or(0),
                ),
            )
        });

    let mut root_nodes: Vec<_> = root_node_query
        .iter()
        .map(|(root_id, maybe_global_zindex, maybe_zindex)| {
            (
                root_id,
                (
                    maybe_global_zindex.map(|zindex| zindex.0).unwrap_or(0),
                    maybe_zindex.map(|zindex| zindex.0).unwrap_or(0),
                ),
            )
        })
        .chain(global_nodes)
        .collect();

    root_nodes.sort_by_key(|(_, z)| *z);

    for (entity, _) in root_nodes {
        update_uistack_recursively(entity, uinodes, &children_query, &zindex_query);
    }

    for (i, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok(mut node) = update_query.get_mut(*entity) {
            node.bypass_change_detection().stack_index = i as u32;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        component::Component,
        schedule::Schedule,
        system::Commands,
        world::{CommandQueue, World},
    };
    use bevy_hierarchy::{BuildChildren, ChildBuild};

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
