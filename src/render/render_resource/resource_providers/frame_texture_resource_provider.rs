use crate::{
    prelude::World,
    render::{render_resource::ResourceProvider, renderer::Renderer, texture::TextureDescriptor},
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
        let window = resources.get::<winit::window::Window>().unwrap();
        let window_size = window.inner_size();
        self.descriptor.size.width = window_size.width;
        self.descriptor.size.height = window_size.height;

        if let Some(old_resource) = renderer
            .get_render_resources()
            .get_named_resource(&self.name)
        {
            renderer.remove_texture(old_resource);
        }

        let texture_resource = renderer.create_texture(&self.descriptor, None);
        renderer
            .get_render_resources_mut()
            .set_named_resource(&self.name, texture_resource);
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

    fn update(&mut self, _renderer: &mut dyn Renderer, _world: &mut World, _resources: &Resources) {
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
