use crate::ui_node::ComputedNodeTarget;
use crate::Node;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

/// Marker component for all entities in a UI hierarchy.
///
/// The UI systems will traverse past nodes with `UiNode` and without a `Node` and treat their first `Node` descendants as direct children of their first `Node` ancestor.
///
/// Any components necessary for transform and visibility propagation will be added automatically.
#[derive(Component, Debug, Copy, Clone, Default, Reflect)]
#[reflect(Component, Debug)]
#[require(ComputedNodeTarget)]
pub struct UiNode;

pub use inner::*;

#[cfg(feature = "ghost_nodes")]
mod inner {
    use super::*;
    use crate::experimental::{GhostChildren, GhostNode, GhostRootNodes};

    impl GhostNode for UiNode {
        type Actual = Node;
    }

    pub type UiRootNodes<'w, 's> = GhostRootNodes<'w, 's, UiNode>;
    pub type UiChildren<'w, 's> = GhostChildren<'w, 's, UiNode>;
}

#[cfg(not(feature = "ghost_nodes"))]
mod inner {
    use super::*;
    use bevy_ecs::system::SystemParam;

    pub type UiRootNodes<'w, 's> = Query<'w, 's, Entity, (With<Node>, Without<ChildOf>)>;

    /// System param that gives access to UI children utilities.
    #[derive(SystemParam)]
    pub struct UiChildren<'w, 's> {
        ui_children_query: Query<'w, 's, Option<&'static Children>, With<Node>>,
        changed_children_query: Query<'w, 's, Entity, Changed<Children>>,
        parents_query: Query<'w, 's, &'static ChildOf>,
    }

    impl<'w, 's> UiChildren<'w, 's> {
        /// Iterates the children of `entity`.
        pub fn iter_actual_children(&'s self, entity: Entity) -> impl Iterator<Item = Entity> + 's {
            self.ui_children_query
                .get(entity)
                .ok()
                .flatten()
                .map(|children| children.as_ref())
                .unwrap_or(&[])
                .iter()
                .copied()
        }

        /// Returns the UI parent of the provided entity.
        pub fn get_parent(&'s self, entity: Entity) -> Option<Entity> {
            self.parents_query.get(entity).ok().map(|parent| parent.0)
        }

        /// Given an entity in the UI hierarchy, check if its set of children has changed, e.g if children has been added/removed or if the order has changed.
        pub fn is_changed(&'s self, entity: Entity) -> bool {
            self.changed_children_query.contains(entity)
        }

        /// Returns `true` if the given entity is a [`Node`].
        pub fn is_actual(&'s self, entity: Entity) -> bool {
            self.ui_children_query.contains(entity)
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
