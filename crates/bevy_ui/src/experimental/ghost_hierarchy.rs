//! This module contains utilities to flatten a hierarchy using ghost nodes, traversing past ghost nodes automatically.

use bevy_ecs::{prelude::*, system::SystemParam};
use smallvec::SmallVec;

/// Allows a hierarchy of entities that all have a component implementing `GhostNode` to be flattened
/// to just a tree of only the entities with an `Actual` component.
pub trait GhostNode {
    type Actual: Component;
}

/// System param that allows iteration of all actual root nodes.
///
/// An root node is either an `Actual` node without a [`ChildOf`], or with only ghost ancestor nodes.
#[derive(SystemParam)]
pub struct GhostRootNodes<'w, 's, G>
where
    G: GhostNode + Component,
    <G as GhostNode>::Actual: Component,
{
    root_node_query: Query<'w, 's, Entity, (With<<G as GhostNode>::Actual>, Without<ChildOf>)>,
    root_ghost_node_query:
        Query<'w, 's, Entity, (With<G>, Without<<G as GhostNode>::Actual>, Without<ChildOf>)>,
    all_nodes_query: Query<'w, 's, Entity, With<<G as GhostNode>::Actual>>,
    ui_children: GhostChildren<'w, 's, G>,
}

impl<'w, 's, G> GhostRootNodes<'w, 's, G>
where
    G: GhostNode + Component,
{
    pub fn iter(&'s self) -> impl Iterator<Item = Entity> + 's {
        self.root_node_query
            .iter()
            .chain(self.root_ghost_node_query.iter().flat_map(|root_ghost| {
                self.all_nodes_query
                    .iter_many(self.ui_children.iter_actual_children(root_ghost))
            }))
    }
}

/// System param that gives access to UI children utilities, skipping over non-actual nodes.
#[derive(SystemParam)]
pub struct GhostChildren<'w, 's, G>
where
    G: GhostNode + Component,
    <G as GhostNode>::Actual: Component,
{
    actual_children_query:
        Query<'w, 's, (Option<&'static Children>, Has<<G as GhostNode>::Actual>), With<G>>,
    changed_children_query: Query<'w, 's, Entity, Changed<Children>>,
    children_query: Query<'w, 's, &'static Children>,
    ghost_nodes_query: Query<'w, 's, Entity, (With<G>, Without<<G as GhostNode>::Actual>)>,
    parents_query: Query<'w, 's, &'static ChildOf>,
}

impl<'w, 's, G> GhostChildren<'w, 's, G>
where
    G: GhostNode + Component,
    <G as GhostNode>::Actual: Component,
{
    /// Iterates the children of `entity`, skipping over non-actual nodes.
    ///
    /// Traverses the hierarchy depth-first to ensure child order.
    ///
    /// # Performance
    ///
    /// This iterator allocates if the `entity` node has more than 8 children (including ghosts).
    pub fn iter_actual_children(&'s self, entity: Entity) -> GhostChildrenIter<'w, 's, G> {
        GhostChildrenIter {
            stack: self
                .actual_children_query
                .get(entity)
                .map_or(SmallVec::new(), |(children, _)| {
                    children.into_iter().flatten().rev().copied().collect()
                }),
            query: &self.actual_children_query,
        }
    }

    /// Returns the UI parent of the provided entity,  skipping over non-actual nodes.
    pub fn get_parent(&'s self, entity: Entity) -> Option<Entity> {
        self.parents_query
            .iter_ancestors(entity)
            .find(|entity| !self.ghost_nodes_query.contains(*entity))
    }

    /// Iterates the ghosts between this entity and its actual children.
    pub fn iter_ghost_nodes(&'s self, entity: Entity) -> Box<dyn Iterator<Item = Entity> + 's> {
        Box::new(
            self.children_query
                .get(entity)
                .into_iter()
                .flat_map(|children| {
                    self.ghost_nodes_query
                        .iter_many(children)
                        .flat_map(|entity| {
                            core::iter::once(entity).chain(self.iter_ghost_nodes(entity))
                        })
                }),
        )
    }

    /// Given an entity in the UI hierarchy, check if its set of children has changed, e.g if children has been added/removed or if the order has changed.
    pub fn is_changed(&'s self, entity: Entity) -> bool {
        self.changed_children_query.contains(entity)
            || self
                .iter_ghost_nodes(entity)
                .any(|entity| self.changed_children_query.contains(entity))
    }

    /// Returns `true` if the given entity is an actual node.
    pub fn is_actual_node(&'s self, entity: Entity) -> bool {
        self.actual_children_query.contains(entity)
    }
}

pub struct GhostChildrenIter<'w, 's, G>
where
    G: GhostNode + Component,
    <G as GhostNode>::Actual: Component,
{
    stack: SmallVec<[Entity; 8]>,
    query: &'s Query<'w, 's, (Option<&'static Children>, Has<G::Actual>), With<G>>,
}

impl<'w, 's, G> Iterator for GhostChildrenIter<'w, 's, G>
where
    G: GhostNode + Component,
    <G as GhostNode>::Actual: Component,
{
    type Item = Entity;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entity = self.stack.pop()?;
            if let Ok((children, has_node)) = self.query.get(entity) {
                if has_node {
                    return Some(entity);
                }
                if let Some(children) = children {
                    self.stack.extend(children.iter().rev());
                }
            }
        }
    }
}

#[cfg(all(test, feature = "ghost_nodes"))]
mod tests {
    use bevy_ecs::{
        prelude::Component,
        system::{Query, SystemState},
        world::World,
    };

    use crate::{Node, UiChildren, UiNode, UiRootNodes};

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[test]
    fn iterate_ui_root_nodes() {
        let world = &mut World::new();

        // Normal root
        world
            .spawn((A(1), Node::default()))
            .with_children(|parent| {
                parent.spawn((A(2), Node::default()));
                parent
                    .spawn((A(3), UiNode))
                    .with_child((A(4), Node::default()));
            });

        // Ghost root
        world.spawn((A(5), UiNode)).with_children(|parent| {
            parent.spawn((A(6), Node::default()));
            parent
                .spawn((A(7), UiNode))
                .with_child((A(8), Node::default()))
                .with_child(A(9));
        });

        let mut system_state = SystemState::<(UiRootNodes, Query<&A>)>::new(world);
        let (ui_root_nodes, a_query) = system_state.get(world);

        let result: Vec<_> = a_query.iter_many(ui_root_nodes.iter()).collect();

        assert_eq!([&A(1), &A(6), &A(8)], result.as_slice());
    }

    #[test]
    fn iterate_ui_children() {
        let world = &mut World::new();

        let n1 = world.spawn((A(1), Node::default())).id();
        let n2 = world.spawn((A(2), UiNode)).id();
        let n3 = world.spawn((A(3), UiNode)).id();
        let n4 = world.spawn((A(4), Node::default())).id();
        let n5 = world.spawn((A(5), Node::default())).id();

        let n6 = world.spawn((A(6), UiNode)).id();
        let n7 = world.spawn((A(7), UiNode)).id();
        let n8 = world.spawn((A(8), Node::default())).id();
        let n9 = world.spawn((A(9), UiNode)).id();
        let n10 = world.spawn((A(10), Node::default())).id();

        let no_ui = world.spawn_empty().id();

        world.entity_mut(n1).add_children(&[n2, n3, n4, n6]);
        world.entity_mut(n2).add_children(&[n5]);

        world.entity_mut(n6).add_children(&[n7, no_ui, n9]);
        world.entity_mut(n7).add_children(&[n8]);
        world.entity_mut(n9).add_children(&[n10]);

        let mut system_state = SystemState::<(UiChildren, Query<&A>)>::new(world);
        let (ui_children, a_query) = system_state.get(world);

        let result: Vec<_> = a_query
            .iter_many(ui_children.iter_actual_children(n1))
            .collect();

        assert_eq!([&A(5), &A(4), &A(8), &A(10)], result.as_slice());
    }
}
