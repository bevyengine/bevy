use bevy_derive::EntityArchetype;
use super::Node;

#[derive(EntityArchetype)]
#[module(meta = false)]
pub struct UiEntity {
    pub node: Node,
}
