use crate::{render::*, LocalToWorld, Translation};

use legion::prelude::*;
use std::mem;
use zerocopy::AsBytes;

pub const LIGHT_UNIFORM_BUFFER_NAME: &str = "lights";

pub struct LightResourceManager {
    pub lights_are_dirty: bool,
    pub max_lights: usize,
}

impl LightResourceManager {
    pub fn new(max_lights: usize) -> Self {
        LightResourceManager {
            lights_are_dirty: true,
            max_lights: max_lights,
        }
    }
}

impl RenderResourceManager for LightResourceManager {
    fn initialize(&self, render_graph: &mut RenderGraphData, _world: &mut World) {
        let light_uniform_size =
        (self.max_lights * mem::size_of::<LightRaw>()) as wgpu::BufferAddress;

        let light_uniform_buffer = UniformBuffer {
            buffer: render_graph.device.create_buffer(&wgpu::BufferDescriptor {
                size: light_uniform_size,
                usage: wgpu::BufferUsage::UNIFORM
                    | wgpu::BufferUsage::COPY_SRC
                    | wgpu::BufferUsage::COPY_DST,
            }),
            size: light_uniform_size,
        };

        render_graph.set_uniform_buffer(LIGHT_UNIFORM_BUFFER_NAME, light_uniform_buffer);
    }
    fn update<'a>(&mut self, render_graph: &mut RenderGraphData, encoder: &'a mut wgpu::CommandEncoder, world: &mut World) {
        if self.lights_are_dirty {
            let mut light_query = <(Read<Light>, Read<LocalToWorld>, Read<Translation>)>::query();
            let light_count = light_query.iter(world).count();

            self.lights_are_dirty = false;
            let size = mem::size_of::<LightRaw>();
            let total_size = size * light_count;
            let temp_buf_data =
                render_graph.device.create_buffer_mapped(total_size, wgpu::BufferUsage::COPY_SRC);
            for ((light, local_to_world, translation), slot) in light_query
                .iter(world)
                .zip(temp_buf_data.data.chunks_exact_mut(size))
            {
                slot.copy_from_slice(LightRaw::from(&light, &local_to_world.0, &translation).as_bytes());
            }

            let light_uniform_buffer = render_graph.get_uniform_buffer(LIGHT_UNIFORM_BUFFER_NAME).unwrap();
            encoder.copy_buffer_to_buffer(
                &temp_buf_data.finish(),
                0,
                &light_uniform_buffer.buffer,
                0,
                total_size as wgpu::BufferAddress,
            );

        }
    }
    fn resize<'a>(&self, _render_graph: &mut RenderGraphData, _encoder: &'a mut wgpu::CommandEncoder, _world: &mut World) { }
}