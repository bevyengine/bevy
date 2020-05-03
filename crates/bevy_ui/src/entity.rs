use super::Node;
use bevy_derive::EntityArchetype;
use bevy_render::Renderable;
use crate::Rect;

#[derive(EntityArchetype, Default)]
#[module(meta = false)]
pub struct UiEntity {
    pub node: Node,
    pub rect: Rect,
    pub renderable: Renderable,
}
