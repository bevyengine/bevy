use super::Node;
use bevy_derive::EntityArchetype;

#[derive(EntityArchetype)]
#[module(meta = false)]
pub struct UiEntity {
    pub node: Node,
}
