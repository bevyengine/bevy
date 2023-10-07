use super::{
    persistent_buffer::PersistentGpuBufferable, Meshlet, MeshletBoundingCone, MeshletBoundingSphere,
};
use std::sync::Arc;

pub struct ByteArrayPsb(pub Arc<[u8]>);

pub struct MeshletMeshVerticesPsb(pub Arc<[u32]>);

pub struct MeshletMeshMeshletsPsb(pub Arc<[Meshlet]>);

pub struct MeshletMeshBoundingSpheresPsb(pub Arc<[MeshletBoundingSphere]>);

pub struct MeshletMeshBoundingConesPsb(pub Arc<[MeshletBoundingCone]>);

impl PersistentGpuBufferable for ByteArrayPsb {
    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64
    }

    fn as_bytes_le(&self, _start_address: u64) -> &[u8] {
        &self.0
    }
}

impl PersistentGpuBufferable for MeshletMeshVerticesPsb {
    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 4
    }

    fn as_bytes_le(&self, start_address: u64) -> &[u8] {
        todo!()
    }
}

impl PersistentGpuBufferable for MeshletMeshMeshletsPsb {
    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 16
    }

    fn as_bytes_le(&self, start_address: u64) -> &[u8] {
        todo!()
    }
}

impl PersistentGpuBufferable for MeshletMeshBoundingSpheresPsb {
    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 16
    }

    fn as_bytes_le(&self, _start_address: u64) -> &[u8] {
        todo!()
    }
}

impl PersistentGpuBufferable for MeshletMeshBoundingConesPsb {
    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 32
    }

    fn as_bytes_le(&self, _start_address: u64) -> &[u8] {
        todo!()
    }
}
