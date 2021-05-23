use std::borrow::Cow;

use bevy_ecs::world::World;
use bevy_openxr_core::XRDevice;

use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{RenderContext, RenderResourceId, RenderResourceType, TextureId},
};

/// Like `WindowSwapChainNode`, but for XR implementation
/// XR implementation initializes the underlying textures at the startup, and after that
/// this node will swap the textures based on texture id retrieved from XR swapchain
#[derive(Default)]
pub struct XRSwapchainNode {
    resource_ids: Option<Vec<RenderResourceId>>,
}

impl XRSwapchainNode {
    pub const OUT_TEXTURE: &'static str = "texture";

    pub fn new() -> Self {
        XRSwapchainNode::default()
    }
}

impl Node for XRSwapchainNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(XRSwapchainNode::OUT_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        world: &mut World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        const WINDOW_TEXTURE: usize = 0;

        // get XR swapchain
        // FIXME: it might be possible to send the texture_views through events, in which case the world would not need to be mutable
        let mut device = world.get_resource_mut::<XRDevice>().unwrap();
        let swapchain = device.get_swapchain_mut().unwrap();

        // check if textures have been already taken out of swapchain
        let resource_ids = match &self.resource_ids {
            // yes, they have been
            Some(resource_ids) => resource_ids,
            // no, they haven't --> uninitialized
            None => {
                let render_resource_context = render_context.resources_mut();

                // move array of textures from the swapchain into render resource context
                // and set textures to this node
                let resource_ids = swapchain
                    .take_texture_views()
                    .into_iter()
                    .map(|texture_view| {
                        let id = TextureId::new();
                        render_resource_context.add_texture_view(id, texture_view);
                        RenderResourceId::Texture(id)
                    })
                    .collect();

                // insert and return a reference
                self.resource_ids.insert(resource_ids)
            }
        };

        // get next texture by id
        let swap_chain_index = swapchain.get_next_swapchain_image_index();
        let render_resource_id = resource_ids.get(swap_chain_index).unwrap();

        // set output to desired resource id
        output.set(WINDOW_TEXTURE, render_resource_id.clone());
    }
}
