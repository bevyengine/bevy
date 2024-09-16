use super::{
    asset::{Meshlet, MeshletBoundingSpheres},
    persistent_buffer::PersistentGpuBuffer,
    MeshletMesh,
};
use bevy_asset::{AssetId, Assets};
use bevy_ecs::{
    system::{Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::BufferAddress,
    renderer::{RenderDevice, RenderQueue},
};
use bevy_utils::HashMap;
use std::{mem::size_of, ops::Range, sync::Arc};

/// Manages uploading [`MeshletMesh`] asset data to the GPU.
#[derive(Resource)]
pub struct MeshletMeshManager {
    pub vertex_data: PersistentGpuBuffer<Arc<[u8]>>,
    pub vertex_ids: PersistentGpuBuffer<Arc<[u32]>>,
    pub indices: PersistentGpuBuffer<Arc<[u8]>>,
    pub meshlets: PersistentGpuBuffer<Arc<[Meshlet]>>,
    pub meshlet_bounding_spheres: PersistentGpuBuffer<Arc<[MeshletBoundingSpheres]>>,
    meshlet_mesh_slices: HashMap<AssetId<MeshletMesh>, [Range<BufferAddress>; 5]>,
}

impl FromWorld for MeshletMeshManager {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        Self {
            vertex_data: PersistentGpuBuffer::new("meshlet_vertex_data", render_device),
            vertex_ids: PersistentGpuBuffer::new("meshlet_vertex_ids", render_device),
            indices: PersistentGpuBuffer::new("meshlet_indices", render_device),
            meshlets: PersistentGpuBuffer::new("meshlets", render_device),
            meshlet_bounding_spheres: PersistentGpuBuffer::new(
                "meshlet_bounding_spheres",
                render_device,
            ),
            meshlet_mesh_slices: HashMap::new(),
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

            let vertex_data_slice = self
                .vertex_data
                .queue_write(Arc::clone(&meshlet_mesh.vertex_data), ());
            let vertex_ids_slice = self.vertex_ids.queue_write(
                Arc::clone(&meshlet_mesh.vertex_ids),
                vertex_data_slice.start,
            );
            let indices_slice = self
                .indices
                .queue_write(Arc::clone(&meshlet_mesh.indices), ());
            let meshlets_slice = self.meshlets.queue_write(
                Arc::clone(&meshlet_mesh.meshlets),
                (vertex_ids_slice.start, indices_slice.start),
            );
            let meshlet_bounding_spheres_slice = self
                .meshlet_bounding_spheres
                .queue_write(Arc::clone(&meshlet_mesh.bounding_spheres), ());

            [
                vertex_data_slice,
                vertex_ids_slice,
                indices_slice,
                meshlets_slice,
                meshlet_bounding_spheres_slice,
            ]
        };

        // If the MeshletMesh asset has not been uploaded to the GPU yet, queue it for uploading
        let [_, _, _, meshlets_slice, _] = self
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
            [vertex_data_slice, vertex_ids_slice, indices_slice, meshlets_slice, meshlet_bounding_spheres_slice],
        ) = self.meshlet_mesh_slices.remove(asset_id)
        {
            self.vertex_data.mark_slice_unused(vertex_data_slice);
            self.vertex_ids.mark_slice_unused(vertex_ids_slice);
            self.indices.mark_slice_unused(indices_slice);
            self.meshlets.mark_slice_unused(meshlets_slice);
            self.meshlet_bounding_spheres
                .mark_slice_unused(meshlet_bounding_spheres_slice);
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
        .vertex_data
        .perform_writes(&render_queue, &render_device);
    meshlet_mesh_manager
        .vertex_ids
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
}
