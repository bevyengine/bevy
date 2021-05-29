use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{RenderContext, RenderResourceId, RenderResourceType},
};
use bevy_ecs::world::World;
use std::borrow::Cow;

pub struct XrSwapChainNode {
    view_index: usize,
}

impl XrSwapChainNode {
    pub const OUT_TEXTURE: &'static str = "texture";

    pub fn new(view_index: usize) -> Self {
        XrSwapChainNode { view_index }
    }
}

impl Node for XrSwapChainNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(XrSwapChainNode::OUT_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        const XR_VIEW_TEXTURE: usize = 0;
        let render_resource_context = render_context.resources_mut();

        let swap_chain_texture =
            render_resource_context.next_xr_swap_chain_texture(self.view_index);
        output.set(
            XR_VIEW_TEXTURE,
            RenderResourceId::Texture(swap_chain_texture),
        );
    }
}
