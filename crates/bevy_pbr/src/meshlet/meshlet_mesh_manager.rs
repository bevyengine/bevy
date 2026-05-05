use crate::meshlet::asset::{BvhNode, MeshletAabb, MeshletCullData};

use super::{asset::Meshlet, persistent_buffer::PersistentGpuBuffer, MeshletMesh};
use alloc::sync::Arc;
use bevy_asset::{AssetId, Assets};
use bevy_ecs::{
    resource::Resource,
    system::{Commands, Res, ResMut},
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
    pub bvh_nodes: PersistentGpuBuffer<Arc<[BvhNode]>>,
    pub meshlets: PersistentGpuBuffer<Arc<[Meshlet]>>,
    pub meshlet_cull_data: PersistentGpuBuffer<Arc<[MeshletCullData]>>,
    meshlet_mesh_slices:
        HashMap<AssetId<MeshletMesh>, ([Range<BufferAddress>; 7], MeshletAabb, u32)>,
}

pub fn init_meshlet_mesh_manager(mut commands: Commands, render_device: Res<RenderDevice>) {
    commands.insert_resource(MeshletMeshManager {
        vertex_positions: PersistentGpuBuffer::new("meshlet_vertex_positions", &render_device),
        vertex_normals: PersistentGpuBuffer::new("meshlet_vertex_normals", &render_device),
        vertex_uvs: PersistentGpuBuffer::new("meshlet_vertex_uvs", &render_device),
        indices: PersistentGpuBuffer::new("meshlet_indices", &render_device),
        bvh_nodes: PersistentGpuBuffer::new("meshlet_bvh_nodes", &render_device),
        meshlets: PersistentGpuBuffer::new("meshlets", &render_device),
        meshlet_cull_data: PersistentGpuBuffer::new("meshlet_cull_data", &render_device),
        meshlet_mesh_slices: HashMap::default(),
    });
}

impl MeshletMeshManager {
    // Returns the index of the root BVH node, as well as the depth of the BVH.
    pub fn queue_upload_if_needed(
        &mut self,
        asset_id: AssetId<MeshletMesh>,
        assets: &mut Assets<MeshletMesh>,
    ) -> (u32, MeshletAabb, u32) {
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
            let base_meshlet_index = (meshlets_slice.start / size_of::<Meshlet>() as u64) as u32;
            let bvh_node_slice = self
                .bvh_nodes
                .queue_write(Arc::clone(&meshlet_mesh.bvh), base_meshlet_index);
            let meshlet_cull_data_slice = self
                .meshlet_cull_data
                .queue_write(Arc::clone(&meshlet_mesh.meshlet_cull_data), ());

            (
                [
                    vertex_positions_slice,
                    vertex_normals_slice,
                    vertex_uvs_slice,
                    indices_slice,
                    bvh_node_slice,
                    meshlets_slice,
                    meshlet_cull_data_slice,
                ],
                meshlet_mesh.aabb,
                meshlet_mesh.bvh_depth,
            )
        };

        // If the MeshletMesh asset has not been uploaded to the GPU yet, queue it for uploading
        let ([_, _, _, _, bvh_node_slice, _, _], aabb, bvh_depth) = self
            .meshlet_mesh_slices
            .entry(asset_id)
            .or_insert_with_key(queue_meshlet_mesh)
            .clone();

        (
            (bvh_node_slice.start / size_of::<BvhNode>() as u64) as u32,
            aabb,
            bvh_depth,
        )
    }

    pub fn remove(&mut self, asset_id: &AssetId<MeshletMesh>) {
        if let Some((
            [vertex_positions_slice, vertex_normals_slice, vertex_uvs_slice, indices_slice, bvh_node_slice, meshlets_slice, meshlet_cull_data_slice],
            _,
            _,
        )) = self.meshlet_mesh_slices.remove(asset_id)
        {
            self.vertex_positions
                .mark_slice_unused(vertex_positions_slice);
            self.vertex_normals.mark_slice_unused(vertex_normals_slice);
            self.vertex_uvs.mark_slice_unused(vertex_uvs_slice);
            self.indices.mark_slice_unused(indices_slice);
            self.bvh_nodes.mark_slice_unused(bvh_node_slice);
            self.meshlets.mark_slice_unused(meshlets_slice);
            self.meshlet_cull_data
                .mark_slice_unused(meshlet_cull_data_slice);
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
        .bvh_nodes
        .perform_writes(&render_queue, &render_device);
    meshlet_mesh_manager
        .meshlets
        .perform_writes(&render_queue, &render_device);
    meshlet_mesh_manager
        .meshlet_cull_data
        .perform_writes(&render_queue, &render_device);
}
