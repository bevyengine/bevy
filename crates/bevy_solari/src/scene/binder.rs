use super::{blas::BlasManager, RaytracingMesh3d};
use bevy_ecs::{
    resource::Resource,
    system::{Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_math::Mat4;
use bevy_pbr::{MeshMaterial3d, StandardMaterial};
use bevy_render::{
    mesh::allocator::MeshAllocator,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
};
use bevy_transform::components::GlobalTransform;
use std::num::{NonZeroU32, NonZeroU64};

const MAX_MESH_COUNT: Option<NonZeroU32> = NonZeroU32::new(2u32.pow(16));
// const MAX_TEXTURE_COUNT: Option<NonZeroU32> = NonZeroU32::new(10_000);

#[derive(Resource)]
pub struct RaytracingSceneBindings {
    pub bind_group: Option<BindGroup>,
    pub bind_group_layout: BindGroupLayout,
}

pub fn prepare_raytracing_scene_bindings(
    instances: Query<(
        &RaytracingMesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &GlobalTransform,
    )>,
    mesh_allocator: Res<MeshAllocator>,
    blas_manager: Res<BlasManager>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut raytracing_scene_bindings: ResMut<RaytracingSceneBindings>,
) {
    raytracing_scene_bindings.bind_group = None;

    if instances.iter().len() == 0 {
        return;
    }

    let mut vertex_buffers = Vec::new();
    let mut index_buffers = Vec::new();
    let mut tlas = TlasPackage::new(render_device.wgpu_device().create_tlas(
        &CreateTlasDescriptor {
            label: Some("tlas"),
            flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: AccelerationStructureUpdateMode::Build,
            max_instances: instances.iter().len() as u32,
        },
    ));
    let mut transforms = StorageBuffer::<Vec<Mat4>>::default();

    let mut instance_index = 0;
    for (mesh, _material, transform) in &instances {
        if let Some(blas) = blas_manager.get(&mesh.id()) {
            let vertex_slice = mesh_allocator.mesh_vertex_slice(&mesh.id()).unwrap();
            let index_slice = mesh_allocator.mesh_index_slice(&mesh.id()).unwrap();

            vertex_buffers.push(BufferBinding {
                buffer: &vertex_slice.buffer,
                offset: vertex_slice.range.start as u64 * 48,
                size: NonZeroU64::new(vertex_slice.range.len() as u64),
            });
            index_buffers.push(BufferBinding {
                buffer: &index_slice.buffer,
                offset: index_slice.range.start as u64 * 4,
                size: NonZeroU64::new(index_slice.range.len() as u64),
            });

            let transform = transform.compute_matrix();
            *tlas.get_mut_single(instance_index).unwrap() = Some(TlasInstance::new(
                blas,
                tlas_transform(&transform),
                instance_index as u32,
                0xFF,
            ));

            transforms.get_mut().push(transform);

            instance_index += 1;
        }
    }

    if vertex_buffers.is_empty() {
        return;
    }

    transforms.write_buffer(&render_device, &render_queue);

    let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("build_tlas_command_encoder"),
    });
    command_encoder.build_acceleration_structures(&[], [&tlas]);
    render_queue.submit([command_encoder.finish()]);

    raytracing_scene_bindings.bind_group = Some(render_device.create_bind_group(
        "raytracing_scene_bind_group",
        &raytracing_scene_bindings.bind_group_layout,
        &BindGroupEntries::with_indices((
            (0, vertex_buffers.as_slice()),
            (1, index_buffers.as_slice()),
            (4, tlas.as_binding()),
            (5, transforms.binding().unwrap()),
        )),
    ));
}

impl FromWorld for RaytracingSceneBindings {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout_entries = &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: MAX_MESH_COUNT,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: MAX_MESH_COUNT,
            },
            // BindGroupLayoutEntry {
            //     binding: 2,
            //     visibility: ShaderStages::COMPUTE,
            //     ty: BindingType::Texture {
            //         sample_type: TextureSampleType::Float { filterable: true },
            //         view_dimension: TextureViewDimension::D2,
            //         multisampled: false,
            //     },
            //     count: MAX_TEXTURE_COUNT,
            // },
            // BindGroupLayoutEntry {
            //     binding: 3,
            //     visibility: ShaderStages::COMPUTE,
            //     ty: BindingType::Sampler(SamplerBindingType::Filtering),
            //     count: MAX_TEXTURE_COUNT,
            // },
            BindGroupLayoutEntry {
                binding: 4,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::AccelerationStructure,
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 5,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // BindGroupLayoutEntry {
            //     binding: 6,
            //     visibility: ShaderStages::COMPUTE,
            //     ty: BindingType::Buffer {
            //         ty: BufferBindingType::Storage { read_only: true },
            //         has_dynamic_offset: false,
            //         min_binding_size: None,
            //     },
            //     count: None,
            // },
        ];

        Self {
            bind_group: None,
            bind_group_layout: render_device.create_bind_group_layout(
                "raytracing_scene_bind_group_layout",
                bind_group_layout_entries,
            ),
        }
    }
}

fn tlas_transform(transform: &Mat4) -> [f32; 12] {
    transform.transpose().to_cols_array()[..12]
        .try_into()
        .unwrap()
}
