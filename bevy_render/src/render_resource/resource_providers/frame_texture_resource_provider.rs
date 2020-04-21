use crate::{
    render_graph_2::{Node, ResourceBindings, ResourceSlot},
    render_resource::{RenderResourceAssignments, ResourceInfo, ResourceProvider},
    renderer_2::RenderContext,
    texture::TextureDescriptor,
};
use bevy_app::{EventReader, Events};
use bevy_window::{WindowResized, Windows};
use legion::prelude::*;

pub struct WindowTextureNode {
    pub name: String,
    pub descriptor: TextureDescriptor,
    pub window_resized_event_reader: EventReader<WindowResized>,
}

impl Node for WindowTextureNode {
    fn output(&self) -> &[ResourceSlot] {
        static OUTPUT: &[ResourceSlot] =
            &[ResourceSlot::new("window_texture", ResourceInfo::Texture)];
        OUTPUT
    }

    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceBindings,
        output: &mut ResourceBindings,
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

pub struct FrameTextureResourceProvider {
    pub name: String,
    pub descriptor: TextureDescriptor,
    pub width: u32,
    pub height: u32,
}

impl FrameTextureResourceProvider {
    pub fn new(name: &str, descriptor: TextureDescriptor) -> Self {
        FrameTextureResourceProvider {
            name: name.to_string(),
            descriptor,
            width: 0,
            height: 0,
        }
    }
}

impl ResourceProvider for FrameTextureResourceProvider {
    fn update(
        &mut self,
        render_context: &mut dyn RenderContext,
        _world: &World,
        resources: &Resources,
    ) {
        let windows = resources.get::<Windows>().unwrap();
        let window = windows.get_primary().unwrap();

        if self.descriptor.size.width != window.width
            || self.descriptor.size.height != window.height
        {
            self.descriptor.size.width = window.width;
            self.descriptor.size.height = window.height;

            let mut render_resource_assignments =
                resources.get_mut::<RenderResourceAssignments>().unwrap();
            let render_resources = render_context.resources_mut();
            if let Some(old_resource) = render_resource_assignments.get(&self.name) {
                render_resources.remove_texture(old_resource);
            }

            let texture_resource = render_resources.create_texture(&self.descriptor);
            render_resource_assignments.set(&self.name, texture_resource);
        }
    }
}
