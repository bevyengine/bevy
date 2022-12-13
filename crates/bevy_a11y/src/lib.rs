use std::num::NonZeroU128;

pub use accesskit;
use accesskit::{Node, NodeId};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::{Component, Entity};

#[derive(Component, Clone, Default, Deref, DerefMut)]
pub struct AccessibilityNode(pub Node);

impl From<Node> for AccessibilityNode {
    fn from(node: Node) -> Self {
        Self(node)
    }
}

pub trait AccessKitEntityExt {
    fn from_node_id(id: &NodeId) -> Entity {
        Entity::from_bits((id.0.get() - 1) as u64)
    }

    fn to_node_id(&self) -> NodeId;
}

impl AccessKitEntityExt for Entity {
    fn to_node_id(&self) -> NodeId {
        let id = NonZeroU128::new((self.to_bits() + 1) as u128);
        NodeId(id.unwrap())
    }
}
