//! Accessibility for Bevy

#![warn(missing_docs)]
#![allow(clippy::type_complexity)]
#![forbid(unsafe_code)]

use std::{
    num::NonZeroU128,
    sync::{atomic::AtomicBool, Arc},
};

pub use accesskit;
use accesskit::{NodeBuilder, NodeId};
use bevy_app::Plugin;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{Component, Entity},
    system::Resource,
};

/// Resource that tracks whether an assistive technology has requested
/// accessibility information.
///
/// Useful if a third-party plugin needs to conditionally integrate with
/// `AccessKit`
#[derive(Resource, Default, Clone, Debug, Deref, DerefMut)]
pub struct AccessibilityRequested(Arc<AtomicBool>);

/// Component to wrap a [`accesskit::Node`], representing this entity to the platform's
/// accessibility API.
///
/// If an entity has a parent, and that parent also has an `AccessibilityNode`,
/// the entity's node will be a child of the parent's node.
///
/// If the entity doesn't have a parent, or if the immediate parent doesn't have
/// an `AccessibilityNode`, its node will be an immediate child of the primary window.
#[derive(Component, Clone, Deref, DerefMut)]
pub struct AccessibilityNode(pub NodeBuilder);

impl From<NodeBuilder> for AccessibilityNode {
    fn from(node: NodeBuilder) -> Self {
        Self(node)
    }
}

/// Extensions to ease integrating entities with [`AccessKit`](https://accesskit.dev).
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
        app.init_resource::<AccessibilityRequested>()
            .init_resource::<Focus>();
    }
}
