use crate::prelude::World;
use crate::render::render_graph_2::Renderer;
use crate::render::render_graph_2::ResourceProvider;
use crate::render::render_graph_2::TextureDescriptor;

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

    pub fn update(&mut self, renderer: &mut dyn Renderer, world: &World) {
        let window = world.resources.get::<winit::window::Window>().unwrap();
        let window_size = window.inner_size();
        self.descriptor.size.width = window_size.width;
        self.descriptor.size.height = window_size.height;
        renderer.create_texture(&self.name, &self.descriptor);
    }
}

impl ResourceProvider for FrameTextureResourceProvider {
    fn initialize(&mut self, renderer: &mut dyn Renderer, world: &mut World) {
        self.update(renderer, world);
    }

    fn update(&mut self, _renderer: &mut dyn Renderer, _world: &mut World) {}

    fn resize(&mut self, renderer: &mut dyn Renderer, world: &mut World, _width: u32, _height: u32) {
        self.update(renderer, world);
    }
}
