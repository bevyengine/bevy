use crate::{
    render_graph_2::{Node, ResourceSlots, ResourceSlotInfo},
    render_resource::ResourceInfo,
    renderer_2::RenderContext,
    texture::TextureDescriptor,
};
use bevy_app::{EventReader, Events};
use bevy_window::WindowResized;
use legion::prelude::*;
use std::borrow::Cow;

pub struct WindowTextureNode {
    pub descriptor: TextureDescriptor,
    window_resized_event_reader: EventReader<WindowResized>,
}

impl Node for WindowTextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed("texture"),
            resource_type: ResourceInfo::Texture,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        const WINDOW_TEXTURE: usize = 0;
        let window_resized_events = resources.get::<Events<WindowResized>>().unwrap();
        if let Some(event) = window_resized_events.latest(&mut self.window_resized_event_reader) {
            let render_resources = render_context.resources_mut();
            if let Some(old_texture) = output.get(WINDOW_TEXTURE) {
                render_resources.remove_texture(old_texture);
            }

            self.descriptor.size.width = event.width;
            self.descriptor.size.height = event.height;
            let texture_resource = render_resources.create_texture(&self.descriptor);
            output.set(WINDOW_TEXTURE, texture_resource);
        }
    }
}
