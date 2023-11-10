use bevy_asset::Asset;
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct MeshletMesh {
    pub vertex_data: Arc<[u8]>,
    pub vertex_ids: Arc<[u32]>,
    pub indices: Arc<[u8]>,
    pub meshlets: Arc<[Meshlet]>,
    pub meshlet_bounding_spheres: Arc<[MeshletBoundingSphere]>,
}

#[derive(Serialize, Deserialize, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Meshlet {
    pub start_vertex_id: u32,
    pub start_index_id: u32,
    pub vertex_count: u32,
}

#[derive(Serialize, Deserialize, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletBoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}
