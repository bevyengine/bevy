use crate::ui_node::ComputedNodeTarget;
use crate::Node;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_render::view::Visibility;
use bevy_transform::prelude::Transform;

/// Marker component for all entities in a UI hierarchy.
///
/// The UI systems will traverse past nodes with `UiNode` and without a `Node` and treat their first `Node` descendants as direct children of their first `Node` ancestor.
///
/// Any components necessary for transform and visibility propagation will be added automatically.
#[derive(Component, Debug, Copy, Clone, Default, Reflect)]
#[reflect(Component, Debug)]
#[require(Visibility, Transform, ComputedNodeTarget)]
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
