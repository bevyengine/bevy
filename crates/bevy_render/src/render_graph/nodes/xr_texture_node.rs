use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{RenderContext, RenderResourceType},
    texture::TextureDescriptor,
};
use bevy_ecs::world::World;
use std::borrow::Cow;

pub struct XrTextureNode {
    view_index: usize,
    descriptor: TextureDescriptor,
}

impl XrTextureNode {
    pub const OUT_TEXTURE: &'static str = "texture";

    pub fn new(view_index: usize, descriptor: TextureDescriptor) -> Self {
        XrTextureNode {
            view_index,
            descriptor,
        }
    }
}

impl Node for XrTextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(XrTextureNode::OUT_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        _world: &World,
        _render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
    }
}
