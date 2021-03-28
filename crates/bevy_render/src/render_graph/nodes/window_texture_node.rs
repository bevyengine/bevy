use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{RenderContext, RenderResourceId, RenderResourceType},
    texture::TextureDescriptor,
};
use bevy_app::Events;
use bevy_ecs::world::World;
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use std::borrow::Cow;

pub struct WindowTextureNode {
    window_id: WindowId,
    descriptor: TextureDescriptor,
}

impl WindowTextureNode {
    pub const OUT_TEXTURE: &'static str = "texture";

    pub fn new(window_id: WindowId, descriptor: TextureDescriptor) -> Self {
        WindowTextureNode {
            window_id,
            descriptor,
        }
    }
}

impl Node for WindowTextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(WindowTextureNode::OUT_TEXTURE),
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
        const WINDOW_TEXTURE: usize = 0;
        let window_created_events = world.get_resource::<Events<WindowCreated>>().unwrap();
        let window_resized_events = world.get_resource::<Events<WindowResized>>().unwrap();
        let window_created_event_reader =
            window_created_events.get_reader(format!("texture_window_{}", self.window_id).as_str());
        let window_resized_event_reader =
            window_resized_events.get_reader(format!("texture_window_{}", self.window_id).as_str());
        let windows = world.get_resource::<Windows>().unwrap();

        let window = windows
            .get(self.window_id)
            .expect("Window texture node refers to a non-existent window.");

        if window_created_event_reader
            .iter()
            .any(|e| e.id == window.id())
            || window_resized_event_reader
                .iter()
                .any(|e| e.id == window.id())
        {
            let render_resource_context = render_context.resources_mut();
            if let Some(RenderResourceId::Texture(old_texture)) = output.get(WINDOW_TEXTURE) {
                render_resource_context.remove_texture(old_texture);
            }

            self.descriptor.size.width = window.physical_width();
            self.descriptor.size.height = window.physical_height();
            let texture_resource = render_resource_context.create_texture(self.descriptor);
            output.set(WINDOW_TEXTURE, RenderResourceId::Texture(texture_resource));
        }
    }
}
