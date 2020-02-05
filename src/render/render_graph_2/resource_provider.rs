use crate::{render::{
    render_graph_2::{resource_name, Renderer},
    ActiveCamera, Camera, Light, LightRaw,
}, transform::prelude::Translation};
use bevy_transform::prelude::LocalToWorld;
use legion::prelude::*;
use zerocopy::AsBytes;

pub trait ResourceProvider {
    fn initialize(&mut self, renderer: &mut dyn Renderer, world: &mut World);
    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World);
    fn resize(&mut self, renderer: &mut dyn Renderer, world: &mut World, width: u32, height: u32);
}

pub struct CameraResourceProvider;

impl ResourceProvider for CameraResourceProvider {
    fn initialize(&mut self, renderer: &mut dyn Renderer, _world: &mut World) {
        renderer.create_buffer(
            resource_name::uniform::CAMERA,
            std::mem::size_of::<[[f32; 4]; 4]>() as u64,
            wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
        );
    }

    fn update(&mut self, _renderer: &mut dyn Renderer, _world: &mut World) {}
    fn resize(&mut self, renderer: &mut dyn Renderer, world: &mut World, width: u32, height: u32) {
        let matrix_size = std::mem::size_of::<[[f32; 4]; 4]>();
        for (mut camera, local_to_world, _) in
            <(Write<Camera>, Read<LocalToWorld>, Read<ActiveCamera>)>::query().iter_mut(world)
        {
            camera.update(width, height);
            let camera_matrix: [[f32; 4]; 4] =
                (camera.view_matrix * local_to_world.0).to_cols_array_2d();

            renderer.create_buffer_mapped(
                "camera_tmp",
                matrix_size,
                wgpu::BufferUsage::COPY_SRC,
                &mut |data| {
                    data[0..matrix_size].copy_from_slice(camera_matrix.as_bytes());
                },
            );

            renderer.copy_buffer_to_buffer(
                "camera_tmp",
                0,
                resource_name::uniform::CAMERA,
                0,
                matrix_size as u64,
            );
        }
    }
}

pub struct LightResourceProvider {
    pub lights_are_dirty: bool,
    pub max_lights: usize,
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
            max_lights: max_lights,
        }
    }
}

impl ResourceProvider for LightResourceProvider {
    fn initialize(&mut self, renderer: &mut dyn Renderer, _world: &mut World) {
        let light_uniform_size =
            (std::mem::size_of::<LightCount>() + self.max_lights * std::mem::size_of::<LightRaw>()) as wgpu::BufferAddress;

        renderer.create_buffer(
            resource_name::uniform::LIGHTS,
            light_uniform_size,
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::COPY_DST,
        );
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
            renderer
                .create_buffer_mapped("LIGHT_TMP", total_size, wgpu::BufferUsage::COPY_SRC, &mut |data| {
                    for ((light, local_to_world, translation), slot) in light_query
                        .iter(world)
                        .zip(data.chunks_exact_mut(size))
                    {
                        slot.copy_from_slice(
                            LightRaw::from(&light, &local_to_world.0, &translation).as_bytes(),
                        );
                    }
                });
            renderer
                .create_buffer_mapped("LIGHT_COUNT_TMP", light_count_size, wgpu::BufferUsage::COPY_SRC, &mut |data| {
                    data.copy_from_slice([light_count as u32, 0, 0, 0].as_bytes());
                });

            renderer.copy_buffer_to_buffer(
                "LIGHT_COUNT_TMP",
                0,
                resource_name::uniform::LIGHTS,
                0,
                light_count_size as wgpu::BufferAddress,
            );

            renderer.copy_buffer_to_buffer(
                "LIGHT_TMP",
                light_count_size as u64,
                resource_name::uniform::LIGHTS,
                0,
                total_size as wgpu::BufferAddress,
            );
        }
    }
    fn resize(&mut self, renderer: &mut dyn Renderer, world: &mut World, width: u32, height: u32) {}
}
