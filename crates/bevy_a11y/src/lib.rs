//! Accessibility for Bevy

#![warn(missing_docs)]

use std::num::NonZeroU128;

pub use accesskit;
use accesskit::{Node, NodeId};
use bevy_app::Plugin;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{Component, Entity},
    system::Resource,
};

/// Component to wrap a [`Node`], representing this entity to the platform's
/// accessibility API.
#[derive(Component, Clone, Default, Deref, DerefMut)]
pub struct AccessibilityNode(pub Node);

impl From<Node> for AccessibilityNode {
    fn from(node: Node) -> Self {
        Self(node)
    }
}

/// Extensions to ease integrating entities with [AccessKit](https://accesskit.dev).
pub trait AccessKitEntityExt {
    /// Convert an entity to a stable [`NodeId`].
    fn to_node_id(&self) -> NodeId;
}

impl AccessKitEntityExt for Entity {
    fn to_node_id(&self) -> NodeId {
        let id = NonZeroU128::new(self.to_bits() as u128 + 1);
        NodeId(id.unwrap())
    }
}

/// Resource representing which entity has keyboard focus, if any.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct Focus(Option<Entity>);

/// Plugin managing non-GUI aspects of integrating with accessibility APIs.
pub struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<Focus>();
    }
}
