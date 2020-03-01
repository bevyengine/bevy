use crate::render::{
    render_graph::{resource_name, RenderResource, Renderer, ResourceProvider},
    Light, LightRaw,
};
use bevy_transform::prelude::{LocalToWorld, Translation};
use legion::prelude::*;
use zerocopy::AsBytes;

pub struct LightResourceProvider {
    pub lights_are_dirty: bool,
    pub max_lights: usize,
    pub light_buffer: Option<RenderResource>,
    pub tmp_light_buffer: Option<RenderResource>,
    pub tmp_count_buffer: Option<RenderResource>,
}

#[repr(C)]
#[derive(Clone, Copy, AsBytes)]
pub struct LightCount {
    pub num_lights: [u32; 4],
}

impl LightResourceProvider {
    pub fn new(max_lights: usize) -> Self {
        LightResourceProvider {
            lights_are_dirty: true,
            max_lights,
            light_buffer: None,
            tmp_light_buffer: None,
            tmp_count_buffer: None,
        }
    }
}

impl ResourceProvider for LightResourceProvider {
    fn initialize(&mut self, renderer: &mut dyn Renderer, _world: &mut World) {
        let light_uniform_size = (std::mem::size_of::<LightCount>()
            + self.max_lights * std::mem::size_of::<LightRaw>())
            as wgpu::BufferAddress;

        let buffer = renderer.create_buffer(
            light_uniform_size,
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::COPY_DST,
        );
        renderer.get_render_resources_mut().set_named_resource(resource_name::uniform::LIGHTS, buffer);
        self.light_buffer = Some(buffer);
    }

    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World) {
        if self.lights_are_dirty {
            let light_query = <(Read<Light>, Read<LocalToWorld>, Read<Translation>)>::query();
            let light_count = light_query.iter(world).count();

            if light_count == 0 {
                return;
            }

            self.lights_are_dirty = false;
            let size = std::mem::size_of::<LightRaw>();
            let total_size = size * light_count;
            let light_count_size = std::mem::size_of::<LightCount>();

            if let Some(old_tmp_light_buffer) = self.tmp_light_buffer {
                renderer.remove_buffer(old_tmp_light_buffer);
            }

            if let Some(old_tmp_count_buffer) = self.tmp_count_buffer {
                renderer.remove_buffer(old_tmp_count_buffer);
            }

            self.tmp_light_buffer = Some(renderer.create_buffer_mapped(
                total_size,
                wgpu::BufferUsage::COPY_SRC,
                &mut |data| {
                    for ((light, local_to_world, translation), slot) in
                        light_query.iter(world).zip(data.chunks_exact_mut(size))
                    {
                        slot.copy_from_slice(
                            LightRaw::from(&light, &local_to_world.0, &translation).as_bytes(),
                        );
                    }
                },
            ));
            self.tmp_count_buffer = Some(renderer.create_buffer_mapped(
                light_count_size,
                wgpu::BufferUsage::COPY_SRC,
                &mut |data| {
                    data.copy_from_slice([light_count as u32, 0, 0, 0].as_bytes());
                },
            ));

            renderer.copy_buffer_to_buffer(
                self.tmp_count_buffer.unwrap(),
                0,
                self.light_buffer.unwrap(),
                0,
                light_count_size as wgpu::BufferAddress,
            );

            renderer.copy_buffer_to_buffer(
                self.tmp_light_buffer.unwrap(),
                0,
                self.light_buffer.unwrap(),
                light_count_size as u64,
                total_size as wgpu::BufferAddress,
            );
        }
    }

    fn resize(
        &mut self,
        _renderer: &mut dyn Renderer,
        _world: &mut World,
        _width: u32,
        _height: u32,
    ) {
    }
}
