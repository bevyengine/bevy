use bevy_asset::Asset;
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct MeshletMesh {
    pub vertex_data: Arc<[u8]>,
    pub meshlet_vertices: Arc<[u32]>,
    pub meshlet_indices: Arc<[u8]>,
    pub meshlets: Arc<[Meshlet]>,
    pub meshlet_bounding_spheres: Arc<[MeshletBoundingSphere]>,
    pub meshlet_bounding_cones: Arc<[MeshletBoundingCone]>,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct Meshlet {
    pub meshlet_vertices_index: u32,
    pub meshlet_indices_index: u32,
    pub meshlet_vertex_count: u32,
    pub meshlet_triangle_count: u32,
}

#[derive(Serialize, Deserialize, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct MeshletBoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

#[derive(Serialize, Deserialize, Copy, Clone)]
pub struct MeshletBoundingCone {
    pub apex: Vec3,
    pub axis: Vec3,
}
