use bevy_math::Vec4;
use bytemuck::{Pod, Zeroable};

use super::{
    persistent_buffer::PersistentGpuBufferable, Meshlet, MeshletBoundingCone, MeshletBoundingSphere,
};
use std::{borrow::Cow, sync::Arc};

pub struct ByteArrayPsb(pub Arc<[u8]>);

pub struct MeshletMeshVerticesPsb(pub Arc<[u32]>);

pub struct MeshletMeshMeshletsPsb(pub Arc<[Meshlet]>);

pub struct MeshletMeshBoundingSpheresPsb(pub Arc<[MeshletBoundingSphere]>);

pub struct MeshletMeshBoundingConesPsb(pub Arc<[MeshletBoundingCone]>);

impl PersistentGpuBufferable for ByteArrayPsb {
    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64
    }

    fn as_bytes_le<'a>(&'a self, _start_address: u64) -> Cow<'a, [u8]> {
        Cow::Borrowed(&self.0)
    }
}

impl PersistentGpuBufferable for MeshletMeshVerticesPsb {
    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 4
    }

    fn as_bytes_le<'a>(&'a self, start_address: u64) -> Cow<'a, [u8]> {
        todo!()
    }
}

impl PersistentGpuBufferable for MeshletMeshMeshletsPsb {
    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 16
    }

    fn as_bytes_le<'a>(&'a self, start_address: u64) -> Cow<'a, [u8]> {
        todo!()
    }
}

impl PersistentGpuBufferable for MeshletMeshBoundingSpheresPsb {
    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 16
    }

    fn as_bytes_le<'a>(&'a self, _start_address: u64) -> Cow<'a, [u8]> {
        Cow::Borrowed(bytemuck::cast_slice(&self.0))
    }
}

impl PersistentGpuBufferable for MeshletMeshBoundingConesPsb {
    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 32
    }

    fn as_bytes_le<'a>(&'a self, _start_address: u64) -> Cow<'a, [u8]> {
        self.0
            .into_iter()
            .flat_map(|cone| {
                bytemuck::cast::<_, [u8; 32]>(ConePsb {
                    apex: (cone.apex, 0.0).into(),
                    axis: (cone.axis, 0.0).into(),
                })
            })
            .collect()
    }
}

#[derive(Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct ConePsb {
    apex: Vec4,
    axis: Vec4,
}
