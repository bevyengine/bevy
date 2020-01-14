use crate::{prelude::LocalToWorld, render::*};

use legion::prelude::*;
use std::mem;
use zerocopy::AsBytes;

pub const MATERIAL_BIND_GROUP_LAYOUT_NAME: &str = "material";

pub struct MaterialResourceManager;

impl RenderResourceManager for MaterialResourceManager {
    fn initialize(&self, render_graph: &mut RenderGraphData, _world: &mut World) {
        let material_bind_group_layout =
            render_graph
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    bindings: &[wgpu::BindGroupLayoutBinding {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    }],
                });

        render_graph
            .set_bind_group_layout(MATERIAL_BIND_GROUP_LAYOUT_NAME, material_bind_group_layout);
    }

    fn update<'a>(
        &mut self,
        render_graph: &mut RenderGraphData,
        encoder: &'a mut wgpu::CommandEncoder,
        world: &mut World,
    ) {
        let entities =
            <(Write<Material>, Read<LocalToWorld>)>::query().filter(!component::<Instanced>());
        let entities_count = entities.iter_mut(world).count();
        if entities_count == 0 {
            return;
        }

        let size = mem::size_of::<MaterialUniforms>();
        let temp_buf_data = render_graph
            .device
            .create_buffer_mapped(entities_count * size, wgpu::BufferUsage::COPY_SRC);

        for ((material, transform), slot) in entities
            .iter_mut(world)
            .zip(temp_buf_data.data.chunks_exact_mut(size))
        {
            slot.copy_from_slice(
                MaterialUniforms {
                    model: transform.0.to_cols_array_2d(),
                    color: material.get_color().into(),
                }
                .as_bytes(),
            );
        }

        let material_bind_group_layout = render_graph
            .get_bind_group_layout(MATERIAL_BIND_GROUP_LAYOUT_NAME)
            .unwrap();

        for mut material in <Write<Material>>::query()
            .filter(!component::<Instanced>())
            .iter_mut(world)
        {
            if let None = material.bind_group {
                let material_uniform_size =
                    mem::size_of::<MaterialUniforms>() as wgpu::BufferAddress;
                let uniform_buf = render_graph.device.create_buffer(&wgpu::BufferDescriptor {
                    size: material_uniform_size,
                    usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                });

                let bind_group =
                    render_graph
                        .device
                        .create_bind_group(&wgpu::BindGroupDescriptor {
                            layout: material_bind_group_layout,
                            bindings: &[wgpu::Binding {
                                binding: 0,
                                resource: wgpu::BindingResource::Buffer {
                                    buffer: &uniform_buf,
                                    range: 0..material_uniform_size,
                                },
                            }],
                        });

                material.bind_group = Some(bind_group);
                material.uniform_buf = Some(uniform_buf);
            }
        }

        let temp_buf = temp_buf_data.finish();
        for (i, (material, _)) in entities.iter_mut(world).enumerate() {
            encoder.copy_buffer_to_buffer(
                &temp_buf,
                (i * size) as wgpu::BufferAddress,
                material.uniform_buf.as_ref().unwrap(),
                0,
                size as wgpu::BufferAddress,
            );
        }
    }
    fn resize<'a>(
        &self,
        _render_graph: &mut RenderGraphData,
        _encoder: &'a mut wgpu::CommandEncoder,
        _world: &mut World,
    ) {
    }
}
