use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::RenderContext,
    texture::TextureDescriptor, shader::FieldBindType,
};
use bevy_app::{EventReader, Events};
use bevy_window::{WindowCreated, WindowReference, WindowResized, Windows};
use legion::prelude::*;
use std::borrow::Cow;

pub struct WindowTextureNode {
    window_reference: WindowReference,
    descriptor: TextureDescriptor,
    window_created_event_reader: EventReader<WindowCreated>,
    window_resized_event_reader: EventReader<WindowResized>,
}

impl WindowTextureNode {
    pub const OUT_TEXTURE: &'static str = "texture";
    pub fn new(
        window_reference: WindowReference,
        descriptor: TextureDescriptor,
        window_created_event_reader: EventReader<WindowCreated>,
        window_resized_event_reader: EventReader<WindowResized>,
    ) -> Self {
        WindowTextureNode {
            window_reference,
            descriptor,
            window_created_event_reader,
            window_resized_event_reader,
        }
    }
}

impl Node for WindowTextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(WindowTextureNode::OUT_TEXTURE),
            resource_type: FieldBindType::Texture,
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
        let window_created_events = resources.get::<Events<WindowCreated>>().unwrap();
        let window_resized_events = resources.get::<Events<WindowResized>>().unwrap();
        let windows = resources.get::<Windows>().unwrap();

        let window = match self.window_reference {
            WindowReference::Primary => windows.get_primary().expect("No primary window exists"),
            WindowReference::Id(id) => windows
                .get(id)
                .expect("Received window resized event for non-existent window"),
        };

        if self
            .window_created_event_reader
            .find_latest(&window_created_events, |e| e.id == window.id)
            .is_some()
            || self
                .window_resized_event_reader
                .find_latest(&window_resized_events, |e| e.id == window.id)
                .is_some()
        {
            let render_resources = render_context.resources_mut();
            if let Some(old_texture) = output.get(WINDOW_TEXTURE) {
                render_resources.remove_texture(old_texture);
            }

            self.descriptor.size.width = window.width;
            self.descriptor.size.height = window.height;
            let texture_resource = render_resources.create_texture(self.descriptor);
            output.set(WINDOW_TEXTURE, texture_resource);
        }
    }
}
