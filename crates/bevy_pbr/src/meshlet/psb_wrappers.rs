use super::{
    persistent_buffer::PersistentGpuBufferable, Meshlet, MeshletBoundingCone, MeshletBoundingSphere,
};
use bevy_math::Vec4;
use bytemuck::{Pod, Zeroable};
use std::{borrow::Cow, sync::Arc};

pub struct ByteArrayPsb(pub Arc<[u8]>);

pub struct MeshletMeshVerticesPsb(pub Arc<[u32]>);

pub struct MeshletMeshMeshletsPsb(pub Arc<[Meshlet]>);

pub struct MeshletMeshBoundingSpheresPsb(pub Arc<[MeshletBoundingSphere]>);

pub struct MeshletMeshBoundingConesPsb(pub Arc<[MeshletBoundingCone]>);

impl PersistentGpuBufferable for ByteArrayPsb {
    type ExtraData = ();

    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64
    }

    fn as_bytes_le<'a>(&'a self, _: Self::ExtraData) -> Cow<'a, [u8]> {
        Cow::Borrowed(&self.0)
    }
}

impl PersistentGpuBufferable for MeshletMeshVerticesPsb {
    type ExtraData = u64;

    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 4
    }

    fn as_bytes_le<'a>(&'a self, offset: Self::ExtraData) -> Cow<'a, [u8]> {
        let offset = offset as u32 / 48;

        self.0
            .iter()
            .flat_map(|index| (*index + offset).to_le_bytes())
            .collect()
    }
}

impl PersistentGpuBufferable for MeshletMeshMeshletsPsb {
    type ExtraData = (u64, u64);

    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 16
    }

    fn as_bytes_le<'a>(&'a self, (vertex_offset, index_offset): Self::ExtraData) -> Cow<'a, [u8]> {
        let vertex_offset = vertex_offset as u32 / 4;
        let index_offset = index_offset as u32;

        self.0
            .iter()
            .flat_map(|meshlet| {
                bytemuck::cast::<_, [u8; 16]>(Meshlet {
                    meshlet_vertices_index: meshlet.meshlet_vertices_index + vertex_offset,
                    meshlet_indices_index: meshlet.meshlet_indices_index + index_offset,
                    ..*meshlet
                })
            })
            .collect()
    }
}

impl PersistentGpuBufferable for MeshletMeshBoundingSpheresPsb {
    type ExtraData = ();

    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 16
    }

    fn as_bytes_le<'a>(&'a self, _: Self::ExtraData) -> Cow<'a, [u8]> {
        Cow::Borrowed(bytemuck::cast_slice(&self.0))
    }
}

impl PersistentGpuBufferable for MeshletMeshBoundingConesPsb {
    type ExtraData = ();

    fn size_in_bytes(&self) -> u64 {
        self.0.len() as u64 * 32
    }

    fn as_bytes_le<'a>(&'a self, _: Self::ExtraData) -> Cow<'a, [u8]> {
        self.0
            .iter()
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
