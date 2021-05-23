use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots, WindowTextureNode},
    renderer::{RenderContext, RenderResourceId, RenderResourceType},
    texture::TextureDescriptor,
};
use bevy_app::{Events, ManualEventReader};
use bevy_ecs::world::World;
use bevy_openxr_core::event::XRViewSurfaceCreated;
use std::borrow::Cow;

/// MAIN_SAMPLED_COLOR_ATTACHMENT node in OpenXR implementation, used instead of `WindowTextureNode`
/// otherwise matches `WindowTextureNode`, except the descriptor.size (`Extent3d`) is set from XR viewport events
pub struct XRWindowTextureNode {
    descriptor: TextureDescriptor,
    xr_view_created_reader: ManualEventReader<XRViewSurfaceCreated>,
}

impl XRWindowTextureNode {
    pub fn new(descriptor: TextureDescriptor) -> Self {
        XRWindowTextureNode {
            descriptor,
            xr_view_created_reader: Default::default(),
        }
    }
}

impl Node for XRWindowTextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(WindowTextureNode::OUT_TEXTURE),
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

        let xr_view_created_events = world
            .get_resource::<Events<XRViewSurfaceCreated>>()
            .unwrap();

        for event in self
            .xr_view_created_reader
            .iter(&xr_view_created_events)
            .last()
        {
            // Configure texture size. This usually happens only at the start of openxr session
            let render_resource_context = render_context.resources_mut();
            if let Some(RenderResourceId::Texture(old_texture)) = output.get(WINDOW_TEXTURE) {
                render_resource_context.remove_texture(old_texture);
            }

            self.descriptor.size.width = event.width;
            self.descriptor.size.height = event.height;

            // using GL multiview, two eyes - FIXME: eventually set the depth based on view count from event data
            self.descriptor.size.depth_or_array_layers = 2;

            let texture_resource = render_resource_context.create_texture(self.descriptor);
            output.set(WINDOW_TEXTURE, RenderResourceId::Texture(texture_resource));
        }
    }
}
