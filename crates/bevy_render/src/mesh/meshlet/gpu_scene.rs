use super::MeshletMesh;
use crate::render_resource::{BindGroup, BindGroupLayout};
use bevy_asset::Handle;
use bevy_ecs::{
    system::Resource,
    world::{FromWorld, World},
};
use bevy_utils::HashMap;
use wgpu::Buffer;

pub fn extract_mesh_meshlets() {
    // TODO: Take meshlet mesh handles + transform, remove them, and replace with (some kind of index thats gets extracted every frame)

    // TODO: After uploading, unload asset data from CPU
    // TODO: Handle modified and removed meshlet meshes
}

#[derive(Resource)]
pub struct MeshletGpuBuffers {
    mesh_vertex_data: Buffer,
    meshlet_vertex_buffers: Buffer,
    meshlet_index_buffers: Buffer,
    meshlets: Buffer,
    meshlet_bounding_spheres: Buffer,
    meshlet_bounding_cone: Buffer,

    pub bind_group_layout: BindGroupLayout,
    pub bind_group: BindGroup,
}

impl FromWorld for MeshletGpuBuffers {
    fn from_world(world: &mut World) -> Self {
        todo!()
    }
}

impl MeshletGpuBuffers {
    pub fn handle_meshlet_mesh_events() {
        todo!()
    }
}
