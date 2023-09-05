use bevy_asset::{Handle, HandleId};
use bevy_ecs::system::{Res, ResMut, Resource};
use bevy_render::{
    mesh::{GpuBufferInfo, GpuMesh},
    prelude::Mesh,
    render_asset::RenderAssets,
    render_resource::{
        raytrace::*, Buffer, CommandEncoderDescriptor, IndexFormat, PrimitiveTopology,
    },
    renderer::{RenderDevice, RenderQueue},
};
use bevy_utils::HashMap;
use std::ops::Deref;

#[derive(Resource, Default)]
pub struct BlasStorage {
    storage: HashMap<HandleId, Blas>,
}

impl BlasStorage {
    pub fn get(&self, mesh: &Handle<Mesh>) -> Option<&Blas> {
        self.storage.get(&mesh.id())
    }
}

// TODO: Detect changed meshes and rebuild BLAS
// TODO: Remove no-longer accessed meshes
// TODO: BLAS compaction
// TODO: Async compute queue for BLAS creation
// TODO: Ensure this system runs in parallel with other rendering stuff / in the background
pub fn prepare_blas(
    mut blas_storage: ResMut<BlasStorage>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // Iterate all GpuMeshes and filter to compatible meshes without an existing BLAS
    let meshes = render_meshes
        .iter()
        .filter(|(mesh, gpu_mesh)| filter_compatible_meshes(mesh, gpu_mesh, &blas_storage))
        .collect::<Vec<_>>();

    // Create BLAS, blas size for each mesh
    let blas_resources = meshes
        .iter()
        .map(|(mesh, gpu_mesh)| {
            let (index_buffer, index_count, index_format, index_buffer_offset) =
                map_buffer_info(&gpu_mesh.buffer_info);

            let blas_size = BlasTriangleGeometrySizeDescriptor {
                vertex_format: Mesh::ATTRIBUTE_POSITION.format,
                vertex_count: gpu_mesh.vertex_count,
                index_format,
                index_count,
                flags: AccelerationStructureGeometryFlags::OPAQUE,
            };

            let blas = render_device.wgpu_device().create_blas(
                &CreateBlasDescriptor {
                    label: Some("blas"),
                    flags: AccelerationStructureFlags::PREFER_FAST_TRACE,
                    update_mode: AccelerationStructureUpdateMode::Build,
                },
                BlasGeometrySizeDescriptors::Triangles {
                    desc: vec![blas_size.clone()],
                },
            );
            blas_storage.storage.insert(mesh.id(), blas);

            (
                mesh.clone_weak(),
                gpu_mesh,
                blas_size,
                index_buffer,
                index_buffer_offset,
            )
        })
        .collect::<Vec<_>>();

    // Create list of BlasBuildEntries using blas_resources
    let build_entries = blas_resources
        .iter()
        .map(
            |(mesh, gpu_mesh, blas_size, index_buffer, index_buffer_offset)| BlasBuildEntry {
                blas: blas_storage.get(&mesh).unwrap(),
                geometry: BlasGeometries::TriangleGeometries(vec![BlasTriangleGeometry {
                    size: &blas_size,
                    vertex_buffer: &gpu_mesh.vertex_buffer,
                    first_vertex: 0,
                    vertex_stride: gpu_mesh.layout.layout().array_stride,
                    index_buffer: index_buffer.map(Deref::deref),
                    index_buffer_offset: *index_buffer_offset,
                    transform_buffer: None,
                    transform_buffer_offset: None,
                }]),
            },
        )
        .collect::<Vec<_>>();

    // Build geometry into each BLAS
    let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("build_blas_command_encoder"),
    });
    command_encoder.build_acceleration_structures(&build_entries, &[]);
    render_queue.submit([command_encoder.finish()]);
}

fn filter_compatible_meshes(
    mesh: &Handle<Mesh>,
    gpu_mesh: &GpuMesh,
    blas_storage: &BlasStorage,
) -> bool {
    !blas_storage.storage.contains_key(&mesh.id())
        && gpu_mesh.primitive_topology == PrimitiveTopology::TriangleList
        // TODO: Check contains position+normal+uv+tangent, not exact match
        && gpu_mesh.layout.attribute_ids()
            == &[
                Mesh::ATTRIBUTE_POSITION.id,
                Mesh::ATTRIBUTE_NORMAL.id,
                Mesh::ATTRIBUTE_UV_0.id,
                Mesh::ATTRIBUTE_TANGENT.id,
            ]
        && matches!(
            gpu_mesh.buffer_info,
            GpuBufferInfo::Indexed {
                index_format: IndexFormat::Uint32,
                ..
            }
        )
}

fn map_buffer_info(
    buffer_info: &GpuBufferInfo,
) -> (
    Option<&Buffer>,
    Option<u32>,
    Option<IndexFormat>,
    Option<u64>,
) {
    match buffer_info {
        GpuBufferInfo::Indexed {
            buffer,
            count,
            index_format,
        } => (Some(buffer), Some(*count), Some(*index_format), Some(0)),
        GpuBufferInfo::NonIndexed => unreachable!(),
    }
}
