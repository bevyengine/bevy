//! This module contains [`GhostNode`] and utilities to flatten the UI hierarchy, traversing past ghost nodes.

use bevy_ecs::{prelude::*, system::SystemParam};
use bevy_hierarchy::{Children, HierarchyQueryExt, Parent};
use bevy_reflect::prelude::*;
use bevy_render::view::{InheritedVisibility, ViewVisibility, Visibility};
use bevy_transform::prelude::{GlobalTransform, Transform};
use smallvec::SmallVec;

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

/// System param that allows iteration of UI children, skipping over [`GhostNode`].
///
/// Traverses the hierarchy depth-first to ensure child order.
#[derive(SystemParam)]
pub struct UiChildren<'w, 's> {
    query: Query<'w, 's, (Option<&'static Children>, Option<&'static GhostNode>)>,
}

impl<'w, 's> UiChildren<'w, 's> {
    pub fn iter(&'s self, entity: Entity) -> UiChildrenIter<'w, 's> {
        UiChildrenIter {
            stack: self
                .query
                .get(entity)
                .map_or(SmallVec::new(), |(children, _)| {
                    children.into_iter().flatten().rev().copied().collect()
                }),
            query: &self.query,
        }
    }
}

pub struct UiChildrenIter<'w, 's> {
    stack: SmallVec<[Entity; 8]>,
    query: &'s Query<'w, 's, (Option<&'static Children>, Option<&'static GhostNode>)>,
}

impl<'w, 's> Iterator for UiChildrenIter<'w, 's> {
    type Item = Entity;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entity = self.stack.pop()?;
            let (children, ghost_node) = self.query.get(entity).ok()?;
            if ghost_node.is_none() {
                return Some(entity);
            }
            if let Some(children) = children {
                self.stack.extend(children.iter().copied());
            }
        }
    }
}
