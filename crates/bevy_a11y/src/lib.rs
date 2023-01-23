use std::num::NonZeroU128;

pub use accesskit;
use accesskit::{Node, NodeId};
use bevy_app::Plugin;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::{Component, Entity},
    system::Resource,
};

#[derive(Component, Clone, Default, Deref, DerefMut)]
pub struct AccessibilityNode(pub Node);

impl From<Node> for AccessibilityNode {
    fn from(node: Node) -> Self {
        Self(node)
    }
}

pub trait AccessKitEntityExt {
    fn from_node_id(id: &NodeId) -> Entity {
        Entity::from_bits(id.0.get() as u64 - 2)
    }

    fn to_node_id(&self) -> NodeId;
}

impl AccessKitEntityExt for Entity {
    fn to_node_id(&self) -> NodeId {
        let id = NonZeroU128::new(self.to_bits() as u128 + 1);
        NodeId(id.unwrap())
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct Focus(Option<Entity>);

pub struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<Focus>();
    }
}
