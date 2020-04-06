use crate::{
    render_resource::{RenderResourceAssignments, ResourceProvider},
    renderer::Renderer,
    texture::TextureDescriptor,
};
use bevy_window::Windows;
use legion::prelude::*;

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

    pub fn update(&mut self, renderer: &mut dyn Renderer, resources: &Resources) {
        let windows = resources.get::<Windows>().unwrap();
        let window = windows.get_primary().unwrap();

        if self.descriptor.size.width != window.width
            || self.descriptor.size.height != window.height
        {
            self.descriptor.size.width = window.width;
            self.descriptor.size.height = window.height;

            let mut render_resource_assignments =
                resources.get_mut::<RenderResourceAssignments>().unwrap();
            if let Some(old_resource) = render_resource_assignments.get(&self.name) {
                renderer.remove_texture(old_resource);
            }

            let texture_resource = renderer.create_texture(&self.descriptor, None);
            render_resource_assignments.set(&self.name, texture_resource);
        }
    }
}

impl ResourceProvider for FrameTextureResourceProvider {
    fn update(&mut self, renderer: &mut dyn Renderer, _world: &mut World, resources: &Resources) {
        self.update(renderer, resources)
    }
}
