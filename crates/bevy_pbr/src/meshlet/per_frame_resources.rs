use super::gpu_scene::{MeshletGpuScene, MeshletMeshGpuSceneSlice};
use crate::{MeshTransforms, MeshUniform};
use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::{
        encase::private::WriteInto, BindGroup, BindGroupDescriptor, BindGroupEntry,
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
        BufferBindingType, BufferDescriptor, BufferInitDescriptor, BufferUsages,
        DrawIndexedIndirect, ShaderSize, ShaderStages, StorageBuffer,
    },
    renderer::{RenderDevice, RenderQueue},
};

#[derive(Resource)]
pub struct MeshletPerFrameResources {
    pub culling_bind_group_layout: BindGroupLayout,
    pub draw_bind_group_layout: BindGroupLayout,
}

impl FromWorld for MeshletPerFrameResources {
    fn from_world(world: &mut World) -> Self {
        // TODO: min_binding_sizes
        let entries = &[
            // Instance uniforms
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Instanced meshlet instance indices
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Instanced meshlet meshlet indices
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE | ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Draw command buffer
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Draw index buffer
            BindGroupLayoutEntry {
                binding: 4,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ];

        let render_device = world.resource::<RenderDevice>();
        Self {
            culling_bind_group_layout: render_device.create_bind_group_layout(
                &BindGroupLayoutDescriptor {
                    label: Some("meshlet_per_frame_culling_bind_group_layout"),
                    entries,
                },
            ),
            draw_bind_group_layout: render_device.create_bind_group_layout(
                &BindGroupLayoutDescriptor {
                    label: Some("meshlet_per_frame_draw_bind_group_layout"),
                    entries: &entries[0..3],
                },
            ),
        }
    }
}

impl MeshletPerFrameResources {
    pub fn create<'a>(
        &self,
        instances: impl Iterator<Item = (&'a MeshletMeshGpuSceneSlice, &'a MeshTransforms)>,
        gpu_scene: &MeshletGpuScene,
        render_queue: &RenderQueue,
        render_device: &RenderDevice,
    ) -> (BindGroup, BindGroup, Buffer, Buffer) {
        let total_instanced_meshlet_count = gpu_scene.total_instanced_meshlet_count() as usize;

        // TODO: Do this in extract_meshlet_meshes()
        let mut instance_uniforms = Vec::new();
        let mut instanced_meshlet_instance_indices =
            Vec::with_capacity(total_instanced_meshlet_count);
        let mut instanced_meshlet_meshlet_indices =
            Vec::with_capacity(total_instanced_meshlet_count);
        for (instance_index, (scene_slice, transform)) in instances.enumerate() {
            instance_uniforms.push(MeshUniform::from(transform));

            for meshlet_index in scene_slice.0.clone() {
                instanced_meshlet_instance_indices.push(instance_index as u32);
                instanced_meshlet_meshlet_indices.push(meshlet_index);
            }
        }

        // TODO: Create these resources and bind groups in seperate systems ahead of time
        let instance_uniforms = new_storage_buffer(
            "meshlet_instance_uniforms",
            instance_uniforms,
            render_queue,
            render_device,
        );
        let instanced_meshlet_instance_indices = new_storage_buffer(
            "meshlet_instanced_meshlet_instance_indices",
            instanced_meshlet_instance_indices,
            render_queue,
            render_device,
        );
        let instanced_meshlet_meshlet_indices = new_storage_buffer(
            "meshlet_instanced_meshlet_meshlet_indices",
            instanced_meshlet_meshlet_indices,
            render_queue,
            render_device,
        );
        let draw_command_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("meshlet_draw_command_buffer"),
            contents: DrawIndexedIndirect {
                vertex_count: 0,
                instance_count: 1,
                base_index: 0,
                vertex_offset: 0,
                base_instance: 0,
            }
            .as_bytes(),
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
        });
        let draw_index_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("meshlet_draw_index_buffer"),
            size: 12 * gpu_scene.total_instanced_triangle_count() as u64,
            usage: BufferUsages::STORAGE | BufferUsages::INDEX,
            mapped_at_creation: false,
        });

        let entries = &[
            BindGroupEntry {
                binding: 0,
                resource: instance_uniforms.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 1,
                resource: instanced_meshlet_instance_indices.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 2,
                resource: instanced_meshlet_meshlet_indices.binding().unwrap(),
            },
            BindGroupEntry {
                binding: 3,
                resource: draw_command_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 4,
                resource: draw_index_buffer.as_entire_binding(),
            },
        ];

        (
            render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("meshlet_per_frame_culling_bind_group"),
                layout: &self.culling_bind_group_layout,
                entries,
            }),
            render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("meshlet_per_frame_draw_bind_group"),
                layout: &self.draw_bind_group_layout,
                entries: &entries[0..3],
            }),
            draw_command_buffer,
            draw_index_buffer,
        )
    }
}

fn new_storage_buffer<T: ShaderSize + WriteInto>(
    label: &'static str,
    data: Vec<T>,
    render_queue: &RenderQueue,
    render_device: &RenderDevice,
) -> StorageBuffer<Vec<T>> {
    let mut buffer = StorageBuffer::from(data);
    buffer.set_label(Some(label));
    buffer.write_buffer(render_device, render_queue);
    buffer
}
