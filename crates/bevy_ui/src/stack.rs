//! This module contains the systems that update the stored UI nodes stack

use crate::{GhostNode, GlobalZIndex, ZIndex};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    entity::{EntityHashMap, EntityHashSet},
    prelude::*,
};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use core::ops::Range;

/// The order of the node in the UI layout.
/// Nodes with a higher stack index are drawn on top of and receive interactions before nodes with lower stack indices.
///
/// Automatically calculated in [`UiSystems::Stack`](`super::UiSystems::Stack`).
#[derive(Component, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(Component, Default)]
pub struct ComputedStackIndex(pub u32);

/// The current UI stack, which contains all UI nodes ordered by their depth (back-to-front).
///
/// The first entry is the furthest node from the camera and is the first one to get rendered
/// while the last entry is the first node to receive interactions.
#[derive(Debug, Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct UiStack {
    /// Partition of the `uinodes` list into disjoint slices of nodes that all share the same camera target.
    pub partition: Vec<Range<usize>>,
    /// List of UI nodes ordered from back-to-front
    pub uinodes: Vec<Entity>,
}

/// A `StackRoot` is a non-ghost UI node that is either parentless, has a `GlobalZIndex` component,
/// or has only `GhostNode` ancestors.
/// The stack root and its descendants, up to any nested `StackRoots`, occupy a contiguous range in the render stack.
#[derive(Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct StackRoot {
    global_z: i32,
    local_z: i32,
    new_or_changed: bool,
    previous_index: usize,
}

/// Generates the render stack for UI nodes.
///
/// Create a list of `StackRoot`s from parentless entities and entities with a `GlobalZIndex` component.
/// Then build the `UiStack` from a walk of the existing layout trees starting from each stack root,
/// filtering branches by `Without<GlobalZIndex>`so that we don't revisit nodes.
pub fn ui_stack_system(
    mut local_cache: Local<Vec<(Entity, i32, Option<&'static Children>)>>,
    mut stack_roots: Local<Vec<(Entity, StackRoot)>>,
    mut stack_root_order: Local<EntityHashMap<usize>>,
    mut visited_stack_roots: Local<EntityHashSet>,
    mut ui_stack: ResMut<UiStack>,
    ui_root_nodes: Query<
        Entity,
        (
            With<ComputedStackIndex>,
            Without<ChildOf>,
            Without<GhostNode>,
        ),
    >,
    ui_root_ghost_nodes: Query<
        &Children,
        (With<ComputedStackIndex>, Without<ChildOf>, With<GhostNode>),
    >,
    root_node_query: Query<(Entity, Option<Ref<GlobalZIndex>>, Option<Ref<ZIndex>>)>,
    zindex_global_node_query: Query<
        (Entity, Ref<GlobalZIndex>, Option<Ref<ZIndex>>),
        (With<ComputedStackIndex>, Without<GhostNode>),
    >,
    ui_children: Query<
        (
            Option<&Children>,
            Has<GhostNode>,
            Has<GlobalZIndex>,
            Option<&ZIndex>,
        ),
        With<ComputedStackIndex>,
    >,
    mut update_query: Query<&mut ComputedStackIndex>,
) {
    let mut child_buffer = core::mem::take(&mut *local_cache)
        .into_iter()
        .map(|_| unreachable!())
        .collect();

    stack_root_order.clear();
    for (order, partition) in ui_stack.partition.iter().enumerate() {
        stack_root_order.insert(ui_stack.uinodes[partition.start], order);
    }
    ui_stack.partition.clear();
    ui_stack.uinodes.clear();
    visited_stack_roots.clear();

    for children in ui_root_ghost_nodes.iter() {
        update_uistack_recursive(
            &mut child_buffer,
            children,
            true,
            &ui_children,
            &mut ui_stack.uinodes,
        );
    }

    for (id, maybe_global_zindex, maybe_zindex) in root_node_query
        .iter_many(
            ui_root_nodes
                .iter()
                .chain(child_buffer.iter().map(|&(entity, _, _)| entity)),
        )
        .matched()
    {
        let previous = stack_root_order.get(&id).copied();
        stack_roots.push((
            id,
            StackRoot {
                global_z: maybe_global_zindex.map(|z| z.0).unwrap_or(0),
                local_z: maybe_zindex.map(|z| z.0).unwrap_or(0),
                new_or_changed: previous.is_none()
                    || maybe_global_zindex.as_ref().is_some_and(Ref::is_changed)
                    || maybe_zindex.as_ref().is_some_and(Ref::is_changed),
                previous_index: previous.unwrap_or(usize::MAX),
            },
        ));
        visited_stack_roots.insert(id);
    }
    child_buffer.clear();

    for (id, global_zindex, maybe_zindex) in zindex_global_node_query.iter() {
        if visited_stack_roots.contains(&id) {
            continue;
        }

        let previous = stack_root_order.get(&id).copied();
        stack_roots.push((
            id,
            StackRoot {
                global_z: global_zindex.0,
                local_z: maybe_zindex.map(|z| z.0).unwrap_or(0),
                new_or_changed: previous.is_none()
                    || global_zindex.is_changed()
                    || maybe_zindex.as_ref().is_some_and(Ref::is_changed),
                previous_index: previous.unwrap_or(usize::MAX),
            },
        ));
    }

    // An unstable sort is sufficient here. Roots that are equal must be new, and we
    // only care about maintaining stability across frames.
    stack_roots.sort_unstable_by(|(_, a), (_, b)| a.cmp(b));

    for (root_entity, _) in stack_roots.drain(..) {
        let Ok((children, _, _, _)) = ui_children.get(root_entity) else {
            continue;
        };
        let start = ui_stack.uinodes.len();
        ui_stack.uinodes.push(root_entity);
        if let Some(children) = children {
            update_uistack_recursive(
                &mut child_buffer,
                children,
                false,
                &ui_children,
                &mut ui_stack.uinodes,
            );
        }
        let end = ui_stack.uinodes.len();
        ui_stack.partition.push(start..end);
    }

    for (i, entity) in ui_stack.uinodes.iter().enumerate() {
        if let Ok(mut stack_index) = update_query.get_mut(*entity) {
            stack_index.set_if_neq(ComputedStackIndex(i as u32));
        }
    }

    child_buffer.clear();
    *local_cache = child_buffer.into_iter().map(|_| unreachable!()).collect();
}

fn update_uistack_recursive<'a>(
    child_buffer: &mut Vec<(Entity, i32, Option<&'a Children>)>,
    children: &'a Children,
    is_ghost: bool,
    ui_children: &'a Query<
        (
            Option<&Children>,
            Has<GhostNode>,
            Has<GlobalZIndex>,
            Option<&ZIndex>,
        ),
        With<ComputedStackIndex>,
    >,
    ui_stack: &mut Vec<Entity>,
) {
    let start = child_buffer.len();
    for &child_entity in children {
        let Ok((children, is_ghost, has_global_zindex, zindex)) = ui_children.get(child_entity)
        else {
            continue;
        };
        if is_ghost {
            if let Some(children) = children {
                update_uistack_recursive(child_buffer, children, true, ui_children, ui_stack);
            }
        } else if !has_global_zindex {
            child_buffer.push((
                child_entity,
                zindex.map(|zindex| zindex.0).unwrap_or(0),
                children,
            ));
        }
    }
    if is_ghost {
        return;
    }
    let end = child_buffer.len();

    child_buffer[start..end].sort_by_key(|child| child.1);

    for index in start..end {
        let (child_entity, _, maybe_children) = child_buffer[index];
        ui_stack.push(child_entity);
        if let Some(children) = maybe_children {
            update_uistack_recursive(child_buffer, children, false, ui_children, ui_stack);
        }
    }

    child_buffer.truncate(start);
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        children,
        component::Component,
        hierarchy::ChildOf,
        schedule::Schedule,
        system::Commands,
        world::{CommandQueue, World},
    };

    use crate::{GhostNode, GlobalZIndex, Node, UiStack, ZIndex};

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

        // Test partitioning
        let last_part = ui_stack.partition.last().unwrap();
        assert_eq!(last_part.len(), 1);
        let last_entity = ui_stack.uinodes[last_part.start];
        assert_eq!(*query.get(&world, last_entity).unwrap(), Label("0"));

        let actual_result = ui_stack.uinodes[ui_stack.partition[4].clone()]
            .iter()
            .map(|entity| query.get(&world, *entity).unwrap().clone())
            .collect::<Vec<_>>();
        let expected_result = vec![
            (Label("1")), // ZIndex(1)
            (Label("1-0")),
            (Label("1-0-2")), // ZIndex(-1)
            (Label("1-0-0")),
            (Label("1-0-1")),
            (Label("1-1")),
            (Label("1-3")),
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

        assert_eq!(ui_stack.partition.len(), expected_result.len());
        for (i, part) in ui_stack.partition.iter().enumerate() {
            assert_eq!(*part, i..i + 1);
        }
    }

    #[test]
    fn order_of_stack_roots_should_be_preserved_between_frames() {
        #[derive(Component)]
        struct Marker;
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        for _ in 0..10 {
            world.spawn((Node::default(), GlobalZIndex(0)));
        }

        schedule.run(&mut world);

        let uinodes = world.resource::<UiStack>().uinodes.clone();

        for marked_entity in uinodes.iter().take(3) {
            world.entity_mut(*marked_entity).insert(Marker);
        }

        schedule.run(&mut world);

        assert_eq!(uinodes, world.resource::<UiStack>().uinodes);
    }

    #[test]
    fn last_updated_stack_root_should_be_on_top() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        for _ in 0..10 {
            world.spawn((Node::default(), GlobalZIndex(0)));
        }

        schedule.run(&mut world);

        let first = world.resource::<UiStack>().uinodes[0];

        world.entity_mut(first).insert(GlobalZIndex(0));

        schedule.run(&mut world);

        assert_eq!(first, *world.resource::<UiStack>().uinodes.last().unwrap());

        let first = world.resource::<UiStack>().uinodes[0];

        world.entity_mut(first).insert(ZIndex(0));

        schedule.run(&mut world);

        assert_eq!(first, *world.resource::<UiStack>().uinodes.last().unwrap());
    }

    #[test]
    fn order_of_parented_stack_roots_should_be_preserved_between_frames() {
        #[derive(Component)]
        struct Marker;
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let parent = world.spawn(Node::default()).id();
        for _ in 0..10 {
            world.spawn((Node::default(), GlobalZIndex(0), ChildOf(parent)));
        }

        schedule.run(&mut world);

        let uinodes = world.resource::<UiStack>().uinodes.clone();

        for marked_entity in uinodes.iter().filter(|entity| **entity != parent).take(3) {
            world.entity_mut(*marked_entity).insert(Marker);
        }

        schedule.run(&mut world);

        assert_eq!(uinodes, world.resource::<UiStack>().uinodes);
    }

    #[test]
    fn ghost_nodes_flatten_z_order() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        world.spawn((
            Node::default(),
            Label("root"),
            children![
                (Node::default(), Label("node")),
                (
                    GhostNode,
                    Label("ghost"),
                    children![(Node::default(), Label("ghost_child"), ZIndex(-1))]
                )
            ],
        ));

        schedule.run(&mut world);

        let mut query = world.query::<&Label>();
        let result = world
            .resource::<UiStack>()
            .uinodes
            .iter()
            .map(|entity| query.get(&world, *entity).unwrap().clone())
            .collect::<Vec<_>>();

        assert_eq!(
            result,
            vec![Label("root"), Label("ghost_child"), Label("node")]
        );
    }

    #[test]
    fn ghost_nodes_are_not_added_to_uistack() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        world.spawn((Node::default(), Label("root")));

        world.spawn((GhostNode, Label("ghost")));

        schedule.run(&mut world);
        schedule.run(&mut world);

        let mut query = world.query::<&Label>();
        let result = world
            .resource::<UiStack>()
            .uinodes
            .iter()
            .map(|entity| query.get(&world, *entity).unwrap().clone())
            .collect::<Vec<_>>();

        assert_eq!(result, vec![Label("root")]);
        assert_eq!(world.resource::<UiStack>().partition, vec![0..1]);
    }

    #[test]
    fn non_ghost_descendants_are_promoted_to_stack_roots() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        world.spawn((Node::default(), Label("n_Z_2"), ZIndex(2)));
        world.spawn((Node::default(), Label("n_0")));
        world.spawn((
            GhostNode,
            Label("ghost_0"),
            children![(Node::default(), Label("g_Z_-1"), ZIndex(-1))],
        ));
        world.spawn((Node::default(), Label("n_G_-1"), GlobalZIndex(-1)));

        world.spawn((
            GhostNode,
            Label("ghost_1"),
            children![(Node::default(), Label("g_G_1"), GlobalZIndex(1))],
        ));

        world.spawn((
            GhostNode,
            Label("ghost_2"),
            children![
                (Node::default(), Label("g_Z_1"), ZIndex(1)),
                (Node::default(), Label("g_Z_-2"), ZIndex(-2))
            ],
        ));

        schedule.run(&mut world);

        let mut query = world.query::<&Label>();
        let result = world
            .resource::<UiStack>()
            .uinodes
            .iter()
            .map(|entity| query.get(&world, *entity).unwrap().clone())
            .collect::<Vec<_>>();

        assert_eq!(
            result,
            vec![
                Label("n_G_-1"),
                Label("g_Z_-2"),
                Label("g_Z_-1"),
                Label("n_0"),
                Label("g_Z_1"),
                Label("n_Z_2"),
                Label("g_G_1"),
            ]
        );
    }

    #[test]
    fn descendants_of_nested_ghosts_are_stack_roots() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        world.spawn((
            GhostNode,
            Label("ghost_root"),
            children![(
                GhostNode,
                Label("ghost_1"),
                children![
                    (Node::default(), Label("g1"), ZIndex(1)),
                    (Node::default(), Label("g2"), ZIndex(2)),
                    (
                        GhostNode,
                        Label("ghost_2"),
                        children![
                            (Node::default(), Label("g3"), ZIndex(3)),
                            (Node::default(), Label("g4"), ZIndex(4)),
                        ]
                    ),
                    (
                        GhostNode,
                        Label("ghost_3"),
                        children![
                            (
                                GhostNode,
                                Label("ghost_4"),
                                children![(Node::default(), Label("g5"), ZIndex(5))]
                            ),
                            (Node::default(), Label("g6"), ZIndex(6)),
                        ]
                    ),
                    (Node::default(), Label("g7"), ZIndex(7)),
                ]
            )],
        ));

        schedule.run(&mut world);

        let mut query = world.query::<&Label>();
        let result = world
            .resource::<UiStack>()
            .uinodes
            .iter()
            .map(|entity| query.get(&world, *entity).unwrap().clone())
            .collect::<Vec<_>>();

        assert_eq!(
            result,
            vec![
                Label("g1"),
                Label("g2"),
                Label("g3"),
                Label("g4"),
                Label("g5"),
                Label("g6"),
                Label("g7"),
            ]
        );
        assert_eq!(
            world.resource::<UiStack>().partition,
            (0..7).map(|i| i..i + 1).collect::<Vec<_>>()
        );
    }

    #[test]
    fn nested_ghosts_preserve_stacking_contexts() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let root = world.spawn(Node::default()).id();
        let before = world.spawn((Node::default(), ChildOf(root))).id();
        let ghost = world
            .spawn((GhostNode, ZIndex(10), GlobalZIndex(10), ChildOf(root)))
            .id();
        let nested_ghost = world.spawn((GhostNode, ChildOf(ghost))).id();
        let child = world.spawn((Node::default(), ChildOf(nested_ghost))).id();
        let grandchild = world
            .spawn((Node::default(), ZIndex(-1), ChildOf(child)))
            .id();
        let global = world
            .spawn((Node::default(), GlobalZIndex(1), ChildOf(nested_ghost)))
            .id();
        let global_child = world.spawn((Node::default(), ChildOf(global))).id();
        let after = world.spawn((Node::default(), ChildOf(root))).id();

        for _ in 0..2 {
            schedule.run(&mut world);

            let ui_stack = world.resource::<UiStack>();
            assert_eq!(
                ui_stack.uinodes,
                vec![root, before, child, grandchild, after, global, global_child]
            );
            assert_eq!(ui_stack.partition, vec![0..5, 5..7]);
        }
    }
}
