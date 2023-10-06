use bevy_asset::Asset;
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use serde::{Deserialize, Serialize};

#[derive(Asset, TypePath, Serialize, Deserialize, Clone)]
pub struct MeshletMesh {
    pub vertex_data: Box<[u8]>,
    pub meshlet_vertex_buffer: Box<[u32]>,
    pub meshlet_index_buffer: Box<[u8]>,
    pub meshlets: Box<[Meshlet]>,
    pub meshlet_bounding_spheres: Box<[MeshletBoundingSphere]>,
    pub meshlet_bounding_cones: Box<[MeshletBoundingCone]>,
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
