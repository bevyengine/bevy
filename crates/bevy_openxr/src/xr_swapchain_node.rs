use std::{any::Any, borrow::Cow};

use bevy_ecs::world::World;
use bevy_openxr_core::XRConfigurationState;

use bevy_render::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{
        RenderContext, RenderResourceContext, RenderResourceId, RenderResourceType, TextureId,
    },
};
use bevy_wgpu::renderer::{WgpuRenderContext, WgpuRenderResourceContext};

/// Like `WindowSwapChainNode`, but for XR implementation
/// XR implementation initializes the underlying textures at the startup, and after that
/// this node will swap the textures based on texture id retrieved from XR swapchain
#[derive(Default)]
pub struct XRSwapchainNode {
    resource_ids: Option<Vec<(RenderResourceId, Option<wgpu::TextureView>)>>,
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

    fn prepare(&mut self, world: &mut World) {
        let mut xr_configuration_state = world.get_resource_mut::<XRConfigurationState>().unwrap();
        if let None = self.resource_ids {
            // move array of textures from the swapchain into render resource context
            // and set textures to this node
            self.resource_ids = Some(
                xr_configuration_state
                    .texture_views
                    .take()
                    .unwrap()
                    .into_iter()
                    .map(|texture_view| {
                        (
                            RenderResourceId::Texture(TextureId::new()),
                            Some(texture_view),
                        )
                    })
                    .collect(),
            );
        }
    }

    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        const WINDOW_TEXTURE: usize = 0;
        let xr_configuration_state = world.get_resource::<XRConfigurationState>().unwrap();

        // get next texture by id
        let render_resource_id = self
            .resource_ids
            .as_mut()
            .unwrap()
            .get_mut(xr_configuration_state.next_swap_chain_index)
            .unwrap();

        if let Some(texture_view) = render_resource_id.1.take() {
            if let RenderResourceId::Texture(texture_id) = render_resource_id.0 {
                let render_resource_context = render_context.resources_mut();

                let render_resource_context = render_resource_context
                    .downcast_mut::<WgpuRenderResourceContext>()
                    .unwrap();

                render_resource_context.add_wgpu_texture_view(texture_id, texture_view);
            }
        }

        // set output to desired resource id
        output.set(WINDOW_TEXTURE, render_resource_id.0.clone());
    }
}
