//! Relationships for defining "portal children".
//!
//! The term "portal" is commonly used in web user interface libraries to mean a mechanism whereby a
//! parent element can have a logical child which is physically present elsewhere in the hierarchy.
//! In this case, it means that for rendering and layout purposes, the child acts as a root node,
//! but for purposes of event bubbling and ownership, it acts as a child.
//!
//! This is typically used for UI elements such as menus and dialogs which need to calculate their
//! positions in window coordinates, despite being owned by UI elements nested deep within the
//! hierarchy.

use bevy_ecs::{component::Component, entity::Entity, hierarchy::ChildOf, query::QueryData};

/// Defines the portal child relationship. For purposes of despawning, a portal child behaves
/// as if it's a real child. However, for purpose of rendering and layout, a portal child behaves
/// as if it's a root element. Certain events can also bubble through the portal relationship.
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

/// A traversal algorithm that uses either the [`ChildOf`] or [`PortalChildOf`] relationship. If the
/// entity has both relations, the latter takes precedence.
#[derive(QueryData)]
pub struct PortalTraversal {
    pub(crate) child_of: Option<&'static ChildOf>,
    pub(crate) portal_child_of: Option<&'static PortalChildOf>,
}
