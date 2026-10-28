//! This module contains the systems that update the stored UI nodes stack

use crate::{GhostNode, GlobalZIndex, ZIndex};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{entity::EntityHashMap, prelude::*};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;

/// The order of the node in the UI layout.
/// Nodes with a higher stack index are drawn on top of and receive interactions before nodes with lower stack indices.
///
/// Automatically calculated in [`UiSystems::Stack`](`super::UiSystems::Stack`).
#[derive(Component, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(Component, Default)]
pub struct ComputedStackIndex(pub u32);

/// The current UI stack, which contains all UI nodes ordered by their depth (back-to-front).
///
/// The first node yielded by [`UiStack::iter`] is the furthest node from the camera and is the first one to get rendered
/// while the last node is the first node to receive interactions.
#[derive(Debug, Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct UiStack {
    /// Lists of UI nodes ordered from back-to-front, each sharing the same camera target.
    pub partition: Vec<Vec<Entity>>,
}

impl UiStack {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = Entity> + '_ {
        self.partition.iter().flatten().copied()
    }
}

#[derive(Clone, Copy, Ord, PartialOrd, PartialEq, Eq)]
pub(crate) struct StackRoot {
    global_z: i32,
    local_z: i32,
    new_or_changed: bool,
    previous_index: usize,
    partition: Option<Entity>,
}

/// Generates the render stack for UI nodes.
pub fn ui_stack_system(
    mut local_cache: Local<Vec<(Entity, i32, Option<&'static Children>, bool, Option<i32>)>>,
    mut stack_roots: Local<Vec<(Entity, StackRoot)>>,
    mut stack_lists: Local<Vec<(StackRoot, usize, Vec<Entity>)>>,
    mut stack_list_order: Local<Vec<usize>>,
    mut node_buffers: Local<Vec<Vec<Entity>>>,
    mut stack_root_order: Local<EntityHashMap<usize>>,
    mut ui_stack: ResMut<UiStack>,
    ui_root_nodes: Query<
        (Entity, Option<Ref<GlobalZIndex>>, Option<Ref<ZIndex>>),
        (With<ComputedStackIndex>, Without<ChildOf>),
    >,
    ui_children: Query<
        (
            Option<&Children>,
            Has<GhostNode>,
            Option<&GlobalZIndex>,
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

    for (order, mut nodes) in stack_list_order.drain(..).zip(ui_stack.partition.drain(..)) {
        nodes.clear();
        node_buffers[order] = nodes;
    }

    for (id, maybe_global_zindex, maybe_zindex) in ui_root_nodes.iter() {
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
                partition: None,
            },
        ));
    }
    stack_roots
        .sort_unstable_by_key(|(_, root)| (root.local_z, root.new_or_changed, root.previous_index));

    for (index, (root_entity, mut stack_root)) in stack_roots.iter().copied().enumerate() {
        let Ok((children, is_ghost, global_zindex, zindex)) = ui_children.get(root_entity) else {
            continue;
        };
        stack_root.new_or_changed = false;
        stack_root.previous_index = index;
        if !is_ghost {
            stack_root.partition = Some(root_entity);
            let order = stack_lists.len();
            if order == node_buffers.len() {
                node_buffers.push(Vec::new());
            }
            let mut nodes = core::mem::take(&mut node_buffers[order]);
            nodes.push(root_entity);
            stack_lists.push((stack_root, order, nodes));
        }
        if let Some(children) = children {
            update_uistack_recursive(
                &mut child_buffer,
                children,
                Some(stack_root),
                is_ghost && global_zindex.is_none() && zindex.is_none(),
                &ui_children,
                &mut stack_lists,
                &mut node_buffers,
            );
        }
    }

    stack_root_order.clear();
    for (index, (entity, _)) in stack_roots.drain(..).enumerate() {
        stack_root_order.insert(entity, index);
    }
    stack_lists.sort_unstable_by_key(|(root, order, _)| {
        (root.global_z, root.local_z, root.previous_index, *order)
    });

    for (_, order, nodes) in stack_lists.drain(..) {
        stack_list_order.push(order);
        ui_stack.partition.push(nodes);
    }

    for (i, entity) in ui_stack.iter().enumerate() {
        if let Ok(mut stack_index) = update_query.get_mut(entity) {
            stack_index.set_if_neq(ComputedStackIndex(i as u32));
        }
    }

    child_buffer.clear();
    *local_cache = child_buffer.into_iter().map(|_| unreachable!()).collect();
}

fn update_uistack_recursive<'a>(
    child_buffer: &mut Vec<(Entity, i32, Option<&'a Children>, bool, Option<i32>)>,
    children: &'a Children,
    stack_root: Option<StackRoot>,
    use_child_zindex: bool,
    ui_children: &'a Query<
        (
            Option<&Children>,
            Has<GhostNode>,
            Option<&GlobalZIndex>,
            Option<&ZIndex>,
        ),
        With<ComputedStackIndex>,
    >,
    stack_lists: &mut Vec<(StackRoot, usize, Vec<Entity>)>,
    node_buffers: &mut Vec<Vec<Entity>>,
) {
    let start = child_buffer.len();
    for &child_entity in children {
        let Ok((children, is_ghost, global_zindex, zindex)) = ui_children.get(child_entity) else {
            continue;
        };
        if is_ghost && global_zindex.is_none() && zindex.is_none() {
            if let Some(children) = children {
                update_uistack_recursive(
                    child_buffer,
                    children,
                    None,
                    use_child_zindex,
                    ui_children,
                    stack_lists,
                    node_buffers,
                );
            }
        } else {
            child_buffer.push((
                child_entity,
                zindex.map(|zindex| zindex.0).unwrap_or(0),
                children,
                is_ghost,
                global_zindex.map(|z| z.0),
            ));
        }
    }
    let Some(stack_root) = stack_root else { return };
    let end = child_buffer.len();

    child_buffer[start..end].sort_by_key(|child| child.1);

    for index in start..end {
        let (child_entity, zindex, maybe_children, is_ghost, global_zindex) = child_buffer[index];
        let mut child_root = stack_root;
        if use_child_zindex {
            child_root.local_z = zindex;
        }
        if let Some(global_zindex) = global_zindex {
            child_root.global_z = global_zindex;
            child_root.partition = None;
        }
        if !is_ghost {
            child_root.partition.get_or_insert(child_entity);
            if let Some((root, _, nodes)) = stack_lists.last_mut()
                && *root == child_root
            {
                nodes.push(child_entity);
            } else {
                let order = stack_lists.len();
                if order == node_buffers.len() {
                    node_buffers.push(Vec::new());
                }
                let mut nodes = core::mem::take(&mut node_buffers[order]);
                nodes.push(child_entity);
                stack_lists.push((child_root, order, nodes));
            }
        }
        if let Some(children) = maybe_children {
            update_uistack_recursive(
                child_buffer,
                children,
                Some(child_root),
                false,
                ui_children,
                stack_lists,
                node_buffers,
            );
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

    use crate::{ComputedUiTargetCamera, GhostNode, GlobalZIndex, Node, UiStack, ZIndex};

    use super::{ui_stack_system, ComputedStackIndex};

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
            .iter()
            .map(|entity| query.get(&world, entity).unwrap().clone())
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
        let last_entity = last_part[0];
        assert_eq!(*query.get(&world, last_entity).unwrap(), Label("0"));

        let actual_result = ui_stack.partition[5..7]
            .iter()
            .flatten()
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
            .iter()
            .map(|entity| query.get(&world, entity).unwrap().clone())
            .collect::<Vec<_>>();

        let expected_result = vec![
            (Label("7")),
            (Label("0")),
            (Label("1")),
            (Label("4")),
            (Label("5")),
            (Label("6")),
            (Label("3")),
            (Label("8")),
            (Label("2")),
        ];

        assert_eq!(actual_result, expected_result);

        assert_eq!(ui_stack.partition.len(), expected_result.len());
        for part in &ui_stack.partition {
            assert_eq!(part.len(), 1);
        }
    }

    #[test]
    fn stack_iteration_after_despawning_nodes() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let root = world.spawn(Node::default()).id();
        let child = world
            .spawn((Node::default(), ChildOf(root), GlobalZIndex(1)))
            .id();
        let leaf = world.spawn((Node::default(), ChildOf(child))).id();
        let sibling = world.spawn((Node::default(), ChildOf(root))).id();

        schedule.run(&mut world);

        let ui_stack = world.resource::<UiStack>();
        assert!(ui_stack.iter().eq([root, sibling, child, leaf]));
        assert!(ui_stack.iter().rev().eq([leaf, child, sibling, root]));
        for (index, entity) in ui_stack.iter().enumerate() {
            assert_eq!(
                world.get::<ComputedStackIndex>(entity).unwrap().0,
                index as u32
            );
        }

        world.despawn(child);
        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([root, sibling]));

        world.despawn(root);
        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().partition.is_empty());
        assert!(world.resource::<UiStack>().iter().next().is_none());

        let replacement = world.spawn(Node::default()).id();
        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([replacement]));
        assert_eq!(world.get::<ComputedStackIndex>(replacement).unwrap().0, 0);
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

        let uinodes = world.resource::<UiStack>().iter().collect::<Vec<_>>();

        for marked_entity in uinodes.iter().take(3) {
            world.entity_mut(*marked_entity).insert(Marker);
        }

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq(uinodes));
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

        let first = world.resource::<UiStack>().iter().next().unwrap();

        world.entity_mut(first).insert(GlobalZIndex(0));

        schedule.run(&mut world);

        assert_eq!(
            first,
            world.resource::<UiStack>().iter().next_back().unwrap()
        );

        let first = world.resource::<UiStack>().iter().next().unwrap();

        world.entity_mut(first).insert(ZIndex(0));

        schedule.run(&mut world);

        assert_eq!(
            first,
            world.resource::<UiStack>().iter().next_back().unwrap()
        );
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

        let uinodes = world.resource::<UiStack>().iter().collect::<Vec<_>>();

        for marked_entity in uinodes.iter().filter(|entity| **entity != parent).take(3) {
            world.entity_mut(*marked_entity).insert(Marker);
        }

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq(uinodes));
    }

    #[test]
    fn equal_global_zindex_preserves_sibling_stacking_order() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let a = world
            .spawn((Node::default(), ChildOf(r), GlobalZIndex(1)))
            .id();
        let b = world
            .spawn((Node::default(), ChildOf(r), GlobalZIndex(1)))
            .id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([r, a, b]));

        world.entity_mut(a).insert(GlobalZIndex(1));

        for _ in 0..2 {
            schedule.run(&mut world);
            assert!(world.resource::<UiStack>().iter().eq([r, a, b]));
        }
    }

    #[test]
    fn equal_global_zindex_preserves_ancestor_stacking_order() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let a = world.spawn((Node::default(), ChildOf(r))).id();
        let b = world.spawn((Node::default(), ChildOf(r), ZIndex(1))).id();
        let c = world
            .spawn((Node::default(), ChildOf(a), GlobalZIndex(1), ZIndex(100)))
            .id();
        let d = world
            .spawn((Node::default(), ChildOf(b), GlobalZIndex(1), ZIndex(-100)))
            .id();

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, a, b, c, d]));

        world.entity_mut(b).insert(ZIndex(-1));
        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, b, a, d, c]));
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
            .iter()
            .map(|entity| query.get(&world, entity).unwrap().clone())
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
            .iter()
            .map(|entity| query.get(&world, entity).unwrap().clone())
            .collect::<Vec<_>>();

        assert_eq!(result, vec![Label("root")]);
        assert_eq!(world.resource::<UiStack>().partition.len(), 1);
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
            .iter()
            .map(|entity| query.get(&world, entity).unwrap().clone())
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
            .iter()
            .map(|entity| query.get(&world, entity).unwrap().clone())
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
            world
                .resource::<UiStack>()
                .partition
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            vec![1; 7]
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
                ui_stack.iter().collect::<Vec<_>>(),
                vec![root, before, after, global, global_child, child, grandchild]
            );
            assert_eq!(
                ui_stack.partition,
                vec![
                    vec![root, before, after],
                    vec![global, global_child],
                    vec![child, grandchild]
                ]
            );
        }
    }

    #[test]
    fn ghost_zindex_overrides_the_flattened_ordering_but_not_the_local_ordering() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let a = world.spawn((Node::default(), ChildOf(r), ZIndex(2))).id();
        let b = world.spawn((Node::default(), ChildOf(r), ZIndex(0))).id();
        let g = world.spawn((GhostNode, ChildOf(r), ZIndex(1))).id();
        let c = world.spawn((Node::default(), ChildOf(g), ZIndex(10))).id();
        let d = world.spawn((Node::default(), ChildOf(g))).id();
        let h = world.spawn((GhostNode, ChildOf(g), ZIndex(0))).id();
        let e = world.spawn((Node::default(), ChildOf(h), ZIndex(100))).id();

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, b, d, e, c, a]));
    }

    #[test]
    fn ghost_global_zindex_preserves_local_ordering_within_the_same_global_layer() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let g = world.spawn((GhostNode, ChildOf(r), GlobalZIndex(1))).id();
        let a = world.spawn((Node::default(), ChildOf(g))).id();
        let h = world.spawn((GhostNode, ChildOf(g), ZIndex(1))).id();
        let b = world
            .spawn((Node::default(), ChildOf(h), GlobalZIndex(1)))
            .id();
        let c = world.spawn((Node::default(), ChildOf(g), ZIndex(2))).id();
        let d = world
            .spawn((Node::default(), ChildOf(r), GlobalZIndex(1)))
            .id();

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, a, b, c, d]));

        world.entity_mut(b).insert(GlobalZIndex(2));
        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, a, c, d, b]));
    }

    #[test]
    fn nested_ghost_global_zindex_can_return_to_an_ancestor_layer() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let g = world.spawn((GhostNode, ChildOf(r), GlobalZIndex(1))).id();
        let a = world.spawn((Node::default(), ChildOf(g))).id();
        let h = world.spawn((GhostNode, ChildOf(g), GlobalZIndex(2))).id();
        let b = world.spawn((Node::default(), ChildOf(h))).id();
        let c = world
            .spawn((Node::default(), ChildOf(h), GlobalZIndex(1)))
            .id();
        let d = world.spawn((Node::default(), ChildOf(g))).id();

        for _ in 0..2 {
            schedule.run(&mut world);

            assert!(world.resource::<UiStack>().iter().eq([r, a, c, d, b]));
        }
    }

    #[test]
    fn inserting_ghost_zindex_groups_its_children() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let a = world.spawn((Node::default(), ChildOf(r), ZIndex(1))).id();
        let g = world.spawn((GhostNode, ChildOf(r))).id();
        let b = world.spawn((Node::default(), ChildOf(g), ZIndex(2))).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([r, a, b]));

        world.entity_mut(g).insert(ZIndex(0));
        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, b, a]));
    }

    #[test]
    fn removing_ghost_zindex_restores_flattened_ordering() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let a = world.spawn((Node::default(), ChildOf(r), ZIndex(1))).id();
        let g = world.spawn((GhostNode, ChildOf(r), ZIndex(0))).id();
        let b = world.spawn((Node::default(), ChildOf(g), ZIndex(2))).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([r, b, a]));

        world.entity_mut(g).remove::<ZIndex>();
        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, a, b]));
    }

    #[test]
    fn inserting_ghost_global_zindex_moves_its_children() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn((Node::default(), GlobalZIndex(1))).id();
        let a = world.spawn((Node::default(), ChildOf(r))).id();
        let g = world.spawn((GhostNode, ChildOf(r))).id();
        let b = world.spawn((Node::default(), ChildOf(g))).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([r, a, b]));

        world.entity_mut(g).insert(GlobalZIndex(-1));
        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([b, r, a]));
    }

    #[test]
    fn removing_ghost_global_zindex_restores_inherited_layer() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn((Node::default(), GlobalZIndex(1))).id();
        let a = world.spawn((Node::default(), ChildOf(r))).id();
        let g = world.spawn((GhostNode, ChildOf(r), GlobalZIndex(-1))).id();
        let b = world.spawn((Node::default(), ChildOf(g))).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([b, r, a]));

        world.entity_mut(g).remove::<GlobalZIndex>();
        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, a, b]));
    }

    #[test]
    fn root_ghost_zindex_orders_its_children_as_a_group() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn((Node::default(), ZIndex(1))).id();
        let g = world.spawn((GhostNode, ZIndex(2))).id();
        let a = world.spawn((Node::default(), ChildOf(g), ZIndex(10))).id();
        let b = world.spawn((Node::default(), ChildOf(g), ZIndex(-10))).id();

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, b, a]));
    }

    #[test]
    fn root_ghost_global_zindex_sets_its_childrens_layer() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn((Node::default(), GlobalZIndex(1))).id();
        let g = world.spawn((GhostNode, GlobalZIndex(2))).id();
        let a = world.spawn((Node::default(), ChildOf(g), ZIndex(10))).id();
        let b = world.spawn((Node::default(), ChildOf(g), ZIndex(-10))).id();

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, b, a]));
    }

    #[test]
    fn reordering_children_reorders_their_same_layer_descendants() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let a = world.spawn((Node::default(), ChildOf(r))).id();
        let b = world.spawn((Node::default(), ChildOf(r))).id();
        let c = world
            .spawn((Node::default(), ChildOf(a), GlobalZIndex(1)))
            .id();
        let d = world
            .spawn((Node::default(), ChildOf(b), GlobalZIndex(1)))
            .id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([r, a, b, c, d]));

        world.entity_mut(r).replace_children(&[b, a]);
        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, b, a, d, c]));
    }

    #[test]
    fn reparenting_a_node_updates_its_same_layer_order() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let a = world.spawn((GhostNode, ChildOf(r), GlobalZIndex(1))).id();
        let b = world.spawn((GhostNode, ChildOf(r), GlobalZIndex(1))).id();
        let c = world.spawn((Node::default(), ChildOf(a))).id();
        let d = world.spawn((Node::default(), ChildOf(b))).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([r, c, d]));

        world.entity_mut(c).insert(ChildOf(b));
        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, d, c]));
    }

    #[test]
    fn partitions_do_not_mix_camera_targets() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let camera_a = ComputedUiTargetCamera {
            camera: Some(world.spawn_empty().id()),
        };
        let camera_b = ComputedUiTargetCamera {
            camera: Some(world.spawn_empty().id()),
        };
        let r = world.spawn((Node::default(), ZIndex(0), camera_a)).id();
        let a = world.spawn((Node::default(), ChildOf(r), camera_a)).id();
        let b = world
            .spawn((Node::default(), ChildOf(r), GlobalZIndex(2), camera_a))
            .id();
        let g = world.spawn((GhostNode, ZIndex(1), camera_b)).id();
        let c = world.spawn((Node::default(), ChildOf(g), camera_b)).id();
        let d = world.spawn((Node::default(), ChildOf(c), camera_b)).id();
        let e = world
            .spawn((Node::default(), ChildOf(g), GlobalZIndex(2), camera_b))
            .id();

        schedule.run(&mut world);

        let ui_stack = world.resource::<UiStack>();
        assert!(ui_stack.iter().eq([r, a, c, d, b, e]));
        for nodes in &ui_stack.partition {
            let camera = world.get::<ComputedUiTargetCamera>(nodes[0]).unwrap();
            assert!(nodes
                .iter()
                .all(|entity| world.get::<ComputedUiTargetCamera>(*entity).unwrap() == camera));
        }
    }

    #[test]
    fn computed_stack_indices_update_after_global_zindex_changes() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let a = world
            .spawn((Node::default(), ChildOf(r), GlobalZIndex(-1)))
            .id();
        let b = world
            .spawn((Node::default(), ChildOf(r), GlobalZIndex(1)))
            .id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([a, r, b]));

        world.entity_mut(a).insert(GlobalZIndex(2));
        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, b, a]));
        assert_eq!(world.get::<ComputedStackIndex>(r).unwrap().0, 0);
        assert_eq!(world.get::<ComputedStackIndex>(b).unwrap().0, 1);
        assert_eq!(world.get::<ComputedStackIndex>(a).unwrap().0, 2);
    }

    #[test]
    fn computed_stack_indices_update_after_reordering_children() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let r = world.spawn(Node::default()).id();
        let a = world.spawn((Node::default(), ChildOf(r))).id();
        let b = world.spawn((Node::default(), ChildOf(r))).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([r, a, b]));

        world.entity_mut(r).replace_children(&[b, a]);
        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r, b, a]));
        assert_eq!(world.get::<ComputedStackIndex>(r).unwrap().0, 0);
        assert_eq!(world.get::<ComputedStackIndex>(b).unwrap().0, 1);
        assert_eq!(world.get::<ComputedStackIndex>(a).unwrap().0, 2);
    }

    #[test]
    fn global_zindex_below_non_ui_parent_is_not_added_to_uistack() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let e = world.spawn_empty().id();
        world.spawn((Node::default(), ChildOf(e), GlobalZIndex(0)));

        schedule.run(&mut world);

        let r = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([r]));
    }

    #[test]
    fn changed_nested_global_zindex_is_raised_above_root_with_equal_zindex() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let a = world.spawn(Node::default()).id();
        let c = world
            .spawn((Node::default(), ChildOf(a), GlobalZIndex(1)))
            .id();

        schedule.run(&mut world);

        let b = world.spawn((Node::default(), GlobalZIndex(1))).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([a, c, b]));

        world.entity_mut(c).insert(GlobalZIndex(1));

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([a, b, c]));
    }

    #[test]
    fn changed_root_ghost_child_is_raised_above_root_with_equal_zindex() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let g = world.spawn(GhostNode).id();
        let a = world.spawn((Node::default(), ChildOf(g))).id();

        schedule.run(&mut world);

        let r = world.spawn(Node::default()).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([a, r]));

        world.entity_mut(a).insert(ZIndex(0));

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([r, a]));
    }

    #[test]
    fn new_root_ghost_child_is_raised_above_root_with_equal_zindex() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let g = world.spawn(GhostNode).id();

        schedule.run(&mut world);

        let r = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        let a = world.spawn((Node::default(), ChildOf(g))).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([r, a]));
    }

    #[test]
    fn new_root_is_raised_above_old_root() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let a = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        let b = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([a, b]));
    }

    #[test]
    fn new_root_is_raised_above_old_root_with_no_zindex() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let a = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        let b = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq([a, b]));
    }

    #[test]
    fn changed_zindex_roots_keep_their_relative_order() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let a = world.spawn(Node::default()).id();
        let b = world.spawn(Node::default()).id();
        let c = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        let o1 = world.resource::<UiStack>().iter().collect::<Vec<_>>();

        for e in [a, b, c] {
            world.entity_mut(e).insert(ZIndex(0));
        }

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq(o1.iter().copied()));

        for e in [b, c] {
            world.entity_mut(e).insert(ZIndex(0));
        }

        schedule.run(&mut world);

        let expected = [a]
            .into_iter()
            .chain(o1.iter().copied().filter(|e| *e != a));
        assert!(world.resource::<UiStack>().iter().eq(expected));
    }

    #[test]
    fn changed_global_zindex_roots_keep_their_relative_order() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let a = world.spawn(Node::default()).id();
        let b = world.spawn(Node::default()).id();
        let c = world.spawn(Node::default()).id();

        schedule.run(&mut world);

        let o1 = world.resource::<UiStack>().iter().collect::<Vec<_>>();

        for e in [a, b, c] {
            world.entity_mut(e).insert(GlobalZIndex(0));
        }

        schedule.run(&mut world);

        assert!(world.resource::<UiStack>().iter().eq(o1.iter().copied()));

        for e in [b, c] {
            world.entity_mut(e).insert(GlobalZIndex(0));
        }

        schedule.run(&mut world);

        let expected = [a]
            .into_iter()
            .chain(o1.iter().copied().filter(|e| *e != a));
        assert!(world.resource::<UiStack>().iter().eq(expected));
    }

    #[test]
    fn removing_zindex_from_root_is_raised_above_root_with_equal_zindex() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let a = world.spawn((Node::default(), ZIndex(0))).id();

        schedule.run(&mut world);

        let b = world.spawn(Node::default()).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([a, b]));

        world.entity_mut(a).remove::<ZIndex>();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([b, a]));
    }

    #[test]
    fn removing_global_zindex_from_root_is_raised_above_root_with_equal_zindex() {
        let mut world = World::default();
        world.init_resource::<UiStack>();

        let mut schedule = Schedule::default();
        schedule.add_systems(ui_stack_system);

        let a = world.spawn((Node::default(), GlobalZIndex(0))).id();

        schedule.run(&mut world);

        let b = world.spawn(Node::default()).id();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([a, b]));

        world.entity_mut(a).remove::<GlobalZIndex>();

        schedule.run(&mut world);
        assert!(world.resource::<UiStack>().iter().eq([b, a]));
    }
}
