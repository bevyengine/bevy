use super::{
    asset::{Meshlet, MeshletBoundingSpheres, MeshletSimplificationError},
    persistent_buffer::PersistentGpuBuffer,
    MeshletMesh,
};
use alloc::sync::Arc;
use bevy_asset::{AssetId, Assets};
use bevy_ecs::{
    resource::Resource,
    system::{Res, ResMut},
    world::{FromWorld, World},
};
use bevy_math::Vec2;
use bevy_platform::collections::HashMap;
use bevy_render::{
    render_resource::BufferAddress,
    renderer::{RenderDevice, RenderQueue},
};
use core::ops::Range;

/// Manages uploading [`MeshletMesh`] asset data to the GPU.
#[derive(Resource)]
pub struct MeshletMeshManager {
    pub vertex_positions: PersistentGpuBuffer<Arc<[u32]>>,
    pub vertex_normals: PersistentGpuBuffer<Arc<[u32]>>,
    pub vertex_uvs: PersistentGpuBuffer<Arc<[Vec2]>>,
    pub indices: PersistentGpuBuffer<Arc<[u8]>>,
    pub meshlets: PersistentGpuBuffer<Arc<[Meshlet]>>,
    pub meshlet_bounding_spheres: PersistentGpuBuffer<Arc<[MeshletBoundingSpheres]>>,
    pub meshlet_simplification_errors: PersistentGpuBuffer<Arc<[MeshletSimplificationError]>>,
    meshlet_mesh_slices: HashMap<AssetId<MeshletMesh>, [Range<BufferAddress>; 7]>,
}

impl FromWorld for MeshletMeshManager {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        Self {
            vertex_positions: PersistentGpuBuffer::new("meshlet_vertex_positions", render_device),
            vertex_normals: PersistentGpuBuffer::new("meshlet_vertex_normals", render_device),
            vertex_uvs: PersistentGpuBuffer::new("meshlet_vertex_uvs", render_device),
            indices: PersistentGpuBuffer::new("meshlet_indices", render_device),
            meshlets: PersistentGpuBuffer::new("meshlets", render_device),
            meshlet_bounding_spheres: PersistentGpuBuffer::new(
                "meshlet_bounding_spheres",
                render_device,
            ),
            meshlet_simplification_errors: PersistentGpuBuffer::new(
                "meshlet_simplification_errors",
                render_device,
            ),
            meshlet_mesh_slices: HashMap::default(),
        }
    }
}

impl MeshletMeshManager {
    pub fn queue_upload_if_needed(
        &mut self,
        asset_id: AssetId<MeshletMesh>,
        assets: &mut Assets<MeshletMesh>,
    ) -> Range<u32> {
        let queue_meshlet_mesh = |asset_id: &AssetId<MeshletMesh>| {
            let meshlet_mesh = assets.remove_untracked(*asset_id).expect(
                "MeshletMesh asset was already unloaded but is not registered with MeshletMeshManager",
            );

            let vertex_positions_slice = self
                .vertex_positions
                .queue_write(Arc::clone(&meshlet_mesh.vertex_positions), ());
            let vertex_normals_slice = self
                .vertex_normals
                .queue_write(Arc::clone(&meshlet_mesh.vertex_normals), ());
            let vertex_uvs_slice = self
                .vertex_uvs
                .queue_write(Arc::clone(&meshlet_mesh.vertex_uvs), ());
            let indices_slice = self
                .indices
                .queue_write(Arc::clone(&meshlet_mesh.indices), ());
            let meshlets_slice = self.meshlets.queue_write(
                Arc::clone(&meshlet_mesh.meshlets),
                (
                    vertex_positions_slice.start,
                    vertex_normals_slice.start,
                    indices_slice.start,
                ),
            );
            let meshlet_bounding_spheres_slice = self
                .meshlet_bounding_spheres
                .queue_write(Arc::clone(&meshlet_mesh.meshlet_bounding_spheres), ());
            let meshlet_simplification_errors_slice = self
                .meshlet_simplification_errors
                .queue_write(Arc::clone(&meshlet_mesh.meshlet_simplification_errors), ());

            [
                vertex_positions_slice,
                vertex_normals_slice,
                vertex_uvs_slice,
                indices_slice,
                meshlets_slice,
                meshlet_bounding_spheres_slice,
                meshlet_simplification_errors_slice,
            ]
        };

        // If the MeshletMesh asset has not been uploaded to the GPU yet, queue it for uploading
        let [_, _, _, _, meshlets_slice, _, _] = self
            .meshlet_mesh_slices
            .entry(asset_id)
            .or_insert_with_key(queue_meshlet_mesh)
            .clone();

        let meshlets_slice_start = meshlets_slice.start as u32 / size_of::<Meshlet>() as u32;
        let meshlets_slice_end = meshlets_slice.end as u32 / size_of::<Meshlet>() as u32;
        meshlets_slice_start..meshlets_slice_end
    }

    pub fn remove(&mut self, asset_id: &AssetId<MeshletMesh>) {
        if let Some(
            [vertex_positions_slice, vertex_normals_slice, vertex_uvs_slice, indices_slice, meshlets_slice, meshlet_bounding_spheres_slice, meshlet_simplification_errors_slice],
        ) = self.meshlet_mesh_slices.remove(asset_id)
        {
            self.vertex_positions
                .mark_slice_unused(vertex_positions_slice);
            self.vertex_normals.mark_slice_unused(vertex_normals_slice);
            self.vertex_uvs.mark_slice_unused(vertex_uvs_slice);
            self.indices.mark_slice_unused(indices_slice);
            self.meshlets.mark_slice_unused(meshlets_slice);
            self.meshlet_bounding_spheres
                .mark_slice_unused(meshlet_bounding_spheres_slice);
            self.meshlet_simplification_errors
                .mark_slice_unused(meshlet_simplification_errors_slice);
        }
    }
}

/// Upload all newly queued [`MeshletMesh`] asset data to the GPU.
pub fn perform_pending_meshlet_mesh_writes(
    mut meshlet_mesh_manager: ResMut<MeshletMeshManager>,
    render_queue: Res<RenderQueue>,
    render_device: Res<RenderDevice>,
) {
    meshlet_mesh_manager
        .vertex_positions
        .perform_writes(&render_queue, &render_device);
    meshlet_mesh_manager
        .vertex_normals
        .perform_writes(&render_queue, &render_device);
    meshlet_mesh_manager
        .vertex_uvs
        .perform_writes(&render_queue, &render_device);
    meshlet_mesh_manager
        .indices
        .perform_writes(&render_queue, &render_device);
    meshlet_mesh_manager
        .meshlets
        .perform_writes(&render_queue, &render_device);
    meshlet_mesh_manager
        .meshlet_bounding_spheres
        .perform_writes(&render_queue, &render_device);
    meshlet_mesh_manager
        .meshlet_simplification_errors
        .perform_writes(&render_queue, &render_device);
}
