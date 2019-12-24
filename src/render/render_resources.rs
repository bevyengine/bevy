use crate::{render::*, LocalToWorld, Translation};

use legion::prelude::*;
use std::sync::Arc;
use std::rc::Rc;
use std::mem;
use zerocopy::AsBytes;

use wgpu::{BindGroupLayout, CommandEncoder, Device};

pub struct RenderResources {
    pub local_bind_group_layout: Rc<BindGroupLayout>,
    pub light_uniform_buffer: Arc<UniformBuffer>,
    pub lights_are_dirty: bool,
    pub max_lights: usize,
}

impl RenderResources {
    pub fn new(device: &mut Device, max_lights: usize) -> RenderResources {
        let light_uniform_size =
        (max_lights * mem::size_of::<LightRaw>()) as wgpu::BufferAddress;

        let local_bind_group_layout =
            Rc::new(device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
            }));

        let light_uniform_buffer = Arc::new(UniformBuffer {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                size: light_uniform_size,
                usage: wgpu::BufferUsage::UNIFORM
                    | wgpu::BufferUsage::COPY_SRC
                    | wgpu::BufferUsage::COPY_DST,
            }),
            size: light_uniform_size,
        });

        RenderResources {
            local_bind_group_layout,
            light_uniform_buffer,
            lights_are_dirty: true,
            max_lights
        }
    }
    pub fn update_lights(&mut self, device: &Device, encoder: &mut CommandEncoder, world: &mut World) {
        if self.lights_are_dirty {
            let mut light_query = <(Read<Light>, Read<LocalToWorld>, Read<Translation>)>::query();
            let light_count = light_query.iter(world).count();

            self.lights_are_dirty = false;
            let size = mem::size_of::<LightRaw>();
            let total_size = size * light_count;
            let temp_buf_data =
                device.create_buffer_mapped(total_size, wgpu::BufferUsage::COPY_SRC);
            for ((light, local_to_world, translation), slot) in light_query
                .iter(world)
                .zip(temp_buf_data.data.chunks_exact_mut(size))
            {
                slot.copy_from_slice(LightRaw::from(&light, &local_to_world.0, &translation).as_bytes());
            }
            encoder.copy_buffer_to_buffer(
                &temp_buf_data.finish(),
                0,
                &self.light_uniform_buffer.buffer,
                0,
                total_size as wgpu::BufferAddress,
            );
        }
    }
}
