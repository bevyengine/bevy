use std::borrow::Cow;

use bevy_ecs::world::World;
use bevy_openxr_core::XRDevice;

use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{RenderContext, RenderResourceId, RenderResourceType, TextureId},
};

#[derive(Default)]
pub struct XRSwapChainNode {
    textures: Option<Vec<TextureId>>,
}

impl XRSwapChainNode {
    pub const OUT_TEXTURE: &'static str = "texture";

    pub fn new() -> Self {
        XRSwapChainNode::default()
    }
}

impl Node for XRSwapChainNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(XRSwapChainNode::OUT_TEXTURE),
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

        let mut openxr_device = world.get_resource_mut::<XRDevice>().unwrap();
        let xr_swapchain = openxr_device.get_swapchain_mut().unwrap();

        let textures = match &self.textures {
            Some(textures) => textures,
            None => {
                // uninitialized
                let render_resource_context = render_context.resources_mut();

                // take array of textures away from swapchain
                let texture_views = xr_swapchain.take_color_textures();

                // and set textures to current struct
                let textures: Vec<TextureId> = texture_views
                    .into_iter()
                    .map(|texture_view| {
                        let id = TextureId::new();
                        render_resource_context.add_texture_view(id, texture_view);
                        id
                    })
                    .collect();

                self.textures = Some(textures);
                self.textures.as_mut().unwrap()
            }
        };

        // get next texture by id
        let swap_chain_index = xr_swapchain.get_next_swapchain_image_index();
        let texture_id = textures.get(swap_chain_index).unwrap().clone();

        output.set(WINDOW_TEXTURE, RenderResourceId::Texture(texture_id));
    }
}
