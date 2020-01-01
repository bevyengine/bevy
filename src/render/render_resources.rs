use crate::{render::*, LocalToWorld, Translation, math};

use legion::prelude::*;
use std::sync::Arc;
use std::rc::Rc;
use std::mem;
use zerocopy::AsBytes;

use wgpu::{BindGroupLayout, CommandEncoder, Device};

pub const LIGHT_UNIFORM_BUFFER_NAME: &str = "lights";
pub const FORWARD_UNIFORM_BUFFER_NAME: &str = "forward";
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
    fn initialize(&self, render_graph: &mut RenderGraphData, world: &mut World) {
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
    fn resize<'a>(&self, render_graph: &mut RenderGraphData, encoder: &'a mut wgpu::CommandEncoder, world: &mut World) { }
}

pub struct CameraResourceManager;

impl RenderResourceManager for CameraResourceManager {
    fn initialize(&self, render_graph: &mut RenderGraphData, world: &mut World) {
        let light_count = <Read<Light>>::query().iter_immutable(world).count();
        let forward_uniforms = ForwardUniforms {
            proj: math::Mat4::identity().to_cols_array_2d(),
            num_lights: [light_count as u32, 0, 0, 0],
        };

        let uniform_size = mem::size_of::<ForwardUniforms>() as wgpu::BufferAddress;
        let buffer = render_graph.device.create_buffer_with_data(
            forward_uniforms.as_bytes(),
            wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        );

        let uniform_buffer = UniformBuffer {
            buffer: buffer,
            size: uniform_size,
        };
        render_graph.set_uniform_buffer(FORWARD_UNIFORM_BUFFER_NAME, uniform_buffer);
    }
    fn update<'a>(&mut self, render_graph: &mut RenderGraphData, encoder: &'a mut wgpu::CommandEncoder, world: &mut World) {

    }
    fn resize<'a>(&self, render_graph: &mut RenderGraphData, encoder: &'a mut wgpu::CommandEncoder, world: &mut World) {
        for (mut camera, local_to_world) in <(Write<Camera>, Read<LocalToWorld>)>::query().iter(world) {
            camera.update(render_graph.swap_chain_descriptor.width, render_graph.swap_chain_descriptor.height);
            let camera_matrix: [[f32; 4]; 4] = (camera.view_matrix * local_to_world.0).to_cols_array_2d();
            let matrix_size = mem::size_of::<[[f32; 4]; 4]>() as u64;
            let temp_camera_buffer =
                render_graph.device.create_buffer_with_data(camera_matrix.as_bytes(), wgpu::BufferUsage::COPY_SRC);
            let forward_uniform_buffer = render_graph.get_uniform_buffer(FORWARD_UNIFORM_BUFFER_NAME).unwrap();
            encoder.copy_buffer_to_buffer(&temp_camera_buffer, 0, &forward_uniform_buffer.buffer, 0, matrix_size);
        }
    }
}

pub struct MaterialResourceManager;

impl RenderResourceManager for MaterialResourceManager {
    fn initialize(&self, render_graph: &mut RenderGraphData, world: &mut World) {

    }
    fn update<'a>(&mut self, render_graph: &mut RenderGraphData, encoder: &'a mut wgpu::CommandEncoder, world: &mut World) {
        let mut entities = <(Write<Material>, Read<LocalToWorld>)>::query()
            .filter(!component::<Instanced>());
        let entities_count = entities.iter(world).count();
        let size = mem::size_of::<MaterialUniforms>();
        let temp_buf_data = render_graph.device
            .create_buffer_mapped(entities_count * size, wgpu::BufferUsage::COPY_SRC);

        for ((material, transform), slot) in entities.iter(world)
            .zip(temp_buf_data.data.chunks_exact_mut(size))
        {
            slot.copy_from_slice(
                MaterialUniforms {
                    model: transform.0.to_cols_array_2d(),
                    color: material.color.into(),
                }
                .as_bytes(),
            );
        }
        
        // TODO: dont use inline local
        let local_bind_group_layout = render_graph.get_bind_group_layout("local").unwrap();

        for mut material in <Write<Material>>::query().filter(!component::<Instanced>()).iter(world) {
            if let None = material.bind_group {
                let material_uniform_size = mem::size_of::<MaterialUniforms>() as wgpu::BufferAddress;
                let uniform_buf = render_graph.device.create_buffer(&wgpu::BufferDescriptor {
                    size: material_uniform_size,
                    usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                });

                let bind_group = render_graph.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: local_bind_group_layout,
                    bindings: &[wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &uniform_buf,
                            range: 0 .. material_uniform_size,
                        },
                    }],
                });

                material.bind_group = Some(bind_group);
                material.uniform_buf = Some(uniform_buf);
            }
        }

        let temp_buf = temp_buf_data.finish();
        for (i, (material, _)) in entities.iter(world).enumerate() {
            encoder.copy_buffer_to_buffer(
                &temp_buf,
                (i * size) as wgpu::BufferAddress,
                material.uniform_buf.as_ref().unwrap(),
                0,
                size as wgpu::BufferAddress,
            );
        }
    }
    fn resize<'a>(&self, render_graph: &mut RenderGraphData, encoder: &'a mut wgpu::CommandEncoder, world: &mut World) {

    }

}

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
