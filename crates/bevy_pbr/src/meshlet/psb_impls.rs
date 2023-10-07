use super::{
    persistent_buffer::PersistentGpuBufferable, Meshlet, MeshletBoundingCone, MeshletBoundingSphere,
};
use bevy_math::Vec4;
use bytemuck::{Pod, Zeroable};
use std::{borrow::Cow, sync::Arc};

impl PersistentGpuBufferable for Arc<[u8]> {
    type ExtraData = ();

    fn size_in_bytes(&self) -> u64 {
        self.len() as u64
    }

    fn as_bytes_le<'a>(&'a self, _: Self::ExtraData) -> Cow<'a, [u8]> {
        Cow::Borrowed(self)
    }
}

impl PersistentGpuBufferable for Arc<[u32]> {
    type ExtraData = u64;

    fn size_in_bytes(&self) -> u64 {
        self.len() as u64 * 4
    }

    fn as_bytes_le<'a>(&'a self, offset: Self::ExtraData) -> Cow<'a, [u8]> {
        let offset = offset as u32 / 48;

        self.iter()
            .flat_map(|index| (*index + offset).to_le_bytes())
            .collect()
    }
}

impl PersistentGpuBufferable for Arc<[Meshlet]> {
    type ExtraData = (u64, u64);

    fn size_in_bytes(&self) -> u64 {
        self.len() as u64 * 16
    }

    fn as_bytes_le<'a>(&'a self, (vertex_offset, index_offset): Self::ExtraData) -> Cow<'a, [u8]> {
        let vertex_offset = vertex_offset as u32 / 4;
        let index_offset = index_offset as u32;

        self.iter()
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

impl PersistentGpuBufferable for Arc<[MeshletBoundingSphere]> {
    type ExtraData = ();

    fn size_in_bytes(&self) -> u64 {
        self.len() as u64 * 16
    }

    fn as_bytes_le<'a>(&'a self, _: Self::ExtraData) -> Cow<'a, [u8]> {
        Cow::Borrowed(bytemuck::cast_slice(self))
    }
}

impl PersistentGpuBufferable for Arc<[MeshletBoundingCone]> {
    type ExtraData = ();

    fn size_in_bytes(&self) -> u64 {
        self.len() as u64 * 32
    }

    fn as_bytes_le<'a>(&'a self, _: Self::ExtraData) -> Cow<'a, [u8]> {
        self.iter()
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
