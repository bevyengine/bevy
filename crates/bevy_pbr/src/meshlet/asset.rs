use bevy_asset::Asset;
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct MeshletMesh {
    pub vertex_data: Arc<[u8]>,
    pub meshlet_vertex_buffer: Arc<[u32]>,
    pub meshlet_index_buffer: Arc<[u8]>,
    pub meshlets: Arc<[Meshlet]>,
    pub meshlet_bounding_spheres: Arc<[MeshletBoundingSphere]>,
    pub meshlet_bounding_cones: Arc<[MeshletBoundingCone]>,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct Meshlet {
    pub meshlet_vertex_buffer_index: u32,
    pub meshlet_index_buffer_index: u32,
    pub meshlet_vertex_count: u32,
    pub meshlet_triangle_count: u32,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct MeshletBoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct MeshletBoundingCone {
    pub apex: Vec3,
    pub axis: Vec3,
}
