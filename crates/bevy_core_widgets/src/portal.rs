//! Relationships for defining "portal children", where the term "portal" refers to a mechanism
//! whereby a logical child node can be physically located at a different point in the hierarchy.
//! The "portal" represents a logical connection between the child and it's parent which is not
//! a normal child relationship.

use bevy_ecs::{component::Component, entity::Entity, hierarchy::ChildOf, query::QueryData};

/// Defines the portal child relationship. For purposes of despawning, a portal child behaves
/// as if it's a real child. However, for purpose of rendering and layout, a portal child behaves
/// as if it's a root element. Certain events can also bubble via the portal relationship.
#[derive(Component, Clone, PartialEq, Eq, Debug)]
#[relationship(relationship_target = PortalChildren)]
pub struct PortalChildOf(#[entities] pub Entity);

impl PortalChildOf {
    /// The parent entity of this child entity.
    #[inline]
    pub fn parent(&self) -> Entity {
        self.0
    }
}

/// Tracks the portal children of this entity.
#[derive(Component, Default, Debug, PartialEq, Eq)]
#[relationship_target(relationship = PortalChildOf, linked_spawn)]
pub struct PortalChildren(Vec<Entity>);

/// A traversal that uses either the [`ChildOf`] or [`PortalChildOf`] relationship. If the
/// entity has both relations, the latter takes precedence.
#[derive(QueryData)]
pub struct PortalTraversal {
    pub(crate) child_of: Option<&'static ChildOf>,
    pub(crate) portal_child_of: Option<&'static PortalChildOf>,
}
