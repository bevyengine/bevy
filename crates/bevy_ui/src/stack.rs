//! This module contains the systems that update the stored UI nodes stack

use crate::{
    experimental::{UiChildren, UiRootNodes},
    GlobalZIndex, ZIndex,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{entity::EntityHashSet, prelude::*};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;

/// The order of the node in the UI layout.
/// Nodes with a higher stack index are drawn on top of and receive interactions before nodes with lower stack indices.
///
/// Automatically calculated in [`UiSystems::Stack`](`super::UiSystems::Stack`).
#[derive(Component, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Reflect)]
#[reflect(Component, Default)]
pub struct ComputedStackIndex {
    // order of this node's root ancestor
    pub root: u32,
    // order of the node in the local UI stack
    pub local: u32,
}

/// Local UI stack added to each UI root, contains all UI nodes descending from this root, ordered by their depth (back-to-front).
#[derive(Component, Default, PartialEq, Eq, Deref, DerefMut, Reflect)]
#[reflect(Component, Default)]
pub struct LocalUiStack(pub Vec<Entity>);

/// List of all root UI node entities in order of their depth (back-to-front).
#[derive(Debug, Resource, Default, Reflect)]
#[reflect(Resource, Default)]
pub struct UiStack(pub Vec<Entity>);

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
/// Then build the `LocalUiStack`s from a walk of the existing layout trees starting from each root node,
/// filtering branches by `Without<GlobalZIndex>`so that we don't revisit nodes.
pub fn update_ui_stack_system(
    mut commands: Commands,
    mut cache: Local<ChildBufferCache>,
    mut new_local_stack: Local<Vec<Entity>>,
    mut root_nodes: Local<Vec<(Entity, (i32, i32))>>,
    mut visited_root_nodes: Local<EntityHashSet>,
    ui_root_nodes: UiRootNodes,
    root_node_query: Query<(Entity, Option<&GlobalZIndex>, Option<&ZIndex>)>,
    zindex_global_node_query: Query<
        (Entity, &GlobalZIndex, Option<&ZIndex>),
        With<ComputedStackIndex>,
    >,
    ui_children: UiChildren,
    zindex_query: Query<Option<&ZIndex>, (With<ComputedStackIndex>, Without<GlobalZIndex>)>,
    mut update_query: Query<&mut ComputedStackIndex>,
    mut computed_ui_stack_query: Query<(Entity, &mut LocalUiStack)>,
    mut ui_stack: ResMut<UiStack>,
) {
    visited_root_nodes.clear();
    ui_stack.0.clear();

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
        visited_root_nodes.insert(id);
    }

    root_nodes.sort_by_key(|(_, z)| *z);

    for (root_index, (root_entity, _)) in root_nodes.drain(..).enumerate() {
        ui_stack.0.push(root_entity);
        new_local_stack.clear();
        update_uistack_recursive(
            &mut cache,
            root_entity,
            &ui_children,
            &zindex_query,
            &mut new_local_stack,
        );

        for (local_index, entity) in new_local_stack.iter().enumerate() {
            if let Ok(mut computed_stack_index) = update_query.get_mut(*entity) {
                computed_stack_index.set_if_neq(ComputedStackIndex {
                    root: root_index as u32,
                    local: local_index as u32,
                });
            }
        }

        if let Ok((_, mut local_ui_stack)) = computed_ui_stack_query.get_mut(root_entity) {
            if local_ui_stack.0 != *new_local_stack {
                core::mem::swap(&mut local_ui_stack.0, &mut new_local_stack);
            }
        } else {
            commands
                .entity(root_entity)
                .insert(LocalUiStack(core::mem::take(&mut new_local_stack)));
        }
    }

    for (entity, _) in &mut computed_ui_stack_query {
        if !visited_root_nodes.contains(&entity) {
            commands.entity(entity).remove::<LocalUiStack>();
        }
    }
}

fn update_uistack_recursive(
    cache: &mut ChildBufferCache,
    node_entity: Entity,
    ui_children: &UiChildren,
    zindex_query: &Query<Option<&ZIndex>, (With<ComputedStackIndex>, Without<GlobalZIndex>)>,
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
        entity::Entity,
        query::Changed,
        schedule::Schedule,
        system::Commands,
        world::{CommandQueue, World},
    };

    use crate::{ComputedStackIndex, GlobalZIndex, Node, ZIndex};

    use super::{update_ui_stack_system, LocalUiStack, UiStack};

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

    #[test]
    fn test_update_computed_ui_stacks_system() {
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
        schedule.add_systems(update_ui_stack_system);
        schedule.run(&mut world);
        let mut changed_local_stacks = world.query_filtered::<Entity, Changed<LocalUiStack>>();
        world.clear_trackers();
        schedule.run(&mut world);
        assert!(changed_local_stacks.iter(&world).next().is_none());

        let mut query = world.query::<&Label>();
        let ui_stack_roots = world.resource::<UiStack>();
        let actual_result = ui_stack_roots
            .0
            .iter()
            .flat_map(|entity| world.get::<LocalUiStack>(*entity).unwrap().iter())
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

        let last_root = *ui_stack_roots.0.last().unwrap();
        let last_stack = world.get::<LocalUiStack>(last_root).unwrap();
        assert_eq!(last_stack.len(), 1);
        let last_entity = last_stack[0];
        assert_eq!(*query.get(&world, last_entity).unwrap(), Label("0"));

        let actual_result = world
            .get::<LocalUiStack>(ui_stack_roots.0[4])
            .unwrap()
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

        let expected_result = ui_stack_roots
            .0
            .iter()
            .flat_map(|entity| world.get::<LocalUiStack>(*entity).unwrap().iter())
            .copied()
            .collect::<Vec<_>>();
        let mut computed_ui_stacks_query = world.query::<(&ComputedStackIndex, &LocalUiStack)>();
        let mut computed_ui_stacks = computed_ui_stacks_query.iter(&world).collect::<Vec<_>>();
        computed_ui_stacks.sort_by_key(|(stack_index, _)| *stack_index);
        let actual_result = computed_ui_stacks
            .into_iter()
            .flat_map(|(_, ui_stack)| ui_stack.iter())
            .copied()
            .collect::<Vec<_>>();
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
        schedule.add_systems(update_ui_stack_system);
        schedule.run(&mut world);

        let mut query = world.query::<&Label>();
        let ui_stack_roots = world.resource::<UiStack>();
        let actual_result = ui_stack_roots
            .0
            .iter()
            .flat_map(|entity| world.get::<LocalUiStack>(*entity).unwrap().iter())
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

        assert_eq!(ui_stack_roots.0.len(), expected_result.len());
        for entity in &ui_stack_roots.0 {
            assert_eq!(world.get::<LocalUiStack>(*entity).unwrap().len(), 1);
        }
    }
}
