use crate::{
    core::Windows,
    prelude::World,
    render::{
        render_resource::{RenderResourceAssignments, ResourceProvider},
        renderer::Renderer,
        texture::TextureDescriptor,
    },
};
use legion::prelude::Resources;

pub struct FrameTextureResourceProvider {
    pub name: String,
    pub descriptor: TextureDescriptor,
}

impl FrameTextureResourceProvider {
    pub fn new(name: &str, descriptor: TextureDescriptor) -> Self {
        FrameTextureResourceProvider {
            name: name.to_string(),
            descriptor,
        }
    }

    pub fn update(&mut self, renderer: &mut dyn Renderer, _world: &World, resources: &Resources) {
        let windows = resources.get::<Windows>().unwrap();
        let window = windows.get_primary().unwrap();
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

impl ResourceProvider for FrameTextureResourceProvider {
    fn initialize(
        &mut self,
        renderer: &mut dyn Renderer,
        world: &mut World,
        resources: &Resources,
    ) {
        self.update(renderer, world, resources);
    }

    fn resize(
        &mut self,
        renderer: &mut dyn Renderer,
        world: &mut World,
        resources: &Resources,
        _width: u32,
        _height: u32,
    ) {
        self.update(renderer, world, resources);
    }
}
