//! This module contains [`GhostNode`] and utilities to flatten the UI hierarchy, traversing past ghost nodes.

use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_hierarchy::{HierarchyQueryExt, Parent};
use bevy_reflect::prelude::*;
use bevy_render::view::{InheritedVisibility, ViewVisibility, Visibility};
use bevy_transform::prelude::{GlobalTransform, Transform};

use crate::{Node, TargetCamera};

/// Marker component for entities that should be ignored by within UI hierarchies.
///
/// The UI systems will traverse past these and consider their first non-ghost descendants as direct children of their first non-ghost ancestor.
///
/// Any components necessary for transform and visibility propagation will be added automatically.
#[derive(Component, Default, Debug, Copy, Clone, Reflect)]
#[reflect(Component, Debug)]
#[require(
    Visibility,
    InheritedVisibility,
    ViewVisibility,
    Transform,
    GlobalTransform
)]
pub struct GhostNode;

/// System param that allows iteration of all UI root nodes.
///
/// A UI root node is either a [`Node`] without a [`Parent`], or with only [`GhostNode`] ancestors.
#[derive(SystemParam)]
pub struct UiRootNodes<'w, 's> {
    ghost_node_query: Query<'w, 's, &'static GhostNode>,
    potential_root_node_query: Query<
        'w,
        's,
        (
            Entity,
            Option<&'static TargetCamera>,
            Option<&'static Parent>,
        ),
        With<Node>,
    >,
    parents_query: Query<'w, 's, &'static Parent>,
}

impl<'w, 's> UiRootNodes<'w, 's> {
    pub fn iter(&self) -> impl Iterator<Item = (Entity, Option<&TargetCamera>)> {
        // TODO: Optimize?
        //  - ghost_node_query: Entity, Children, With<GhostNode>, Without<Parent>
        //  - root_node_query: Entity, TargetCamera, With<Node>, Without<Parent>
        //  - Chain instead to avoid having to iterate ancestors on all non root Node's?
        //  - Utilize a filtered parents_query? Filter on GhostNode?

        // Or maybe: Allow both GhostNode and Node to be root nodes?
        //  - Unless there a case where one might want one multiple different UI contexts under a the same GhostNode ancestors?
        self.potential_root_node_query
            .iter()
            .filter(|(entity, _, parent)| {
                parent.is_none() // regular root node
                    || self // check if all ancestors are ghost nodes and if so treat as UI root node
                        .parents_query
                        .iter_ancestors(*entity)
                        .all(|ancestor| self.ghost_node_query.contains(ancestor))
            })
            .map(|(entity, target_camera, _)| (entity, target_camera))
    }
}
