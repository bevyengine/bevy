use super::{
    asset::{Meshlet, MeshletBoundingSpheres, MeshletSimplificationError},
    persistent_buffer::PersistentGpuBufferable,
};
use alloc::sync::Arc;
use bevy_math::Vec2;

impl PersistentGpuBufferable for Arc<[Meshlet]> {
    type Metadata = (u64, u64, u64);

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<Meshlet>()
    }

    fn write_bytes_le(
        &self,
        (vertex_position_offset, vertex_attribute_offset, index_offset): Self::Metadata,
        buffer_slice: &mut [u8],
    ) {
        let vertex_position_offset = (vertex_position_offset * 8) as u32;
        let vertex_attribute_offset = (vertex_attribute_offset as usize / size_of::<u32>()) as u32;
        let index_offset = index_offset as u32;

        for (i, meshlet) in self.iter().enumerate() {
            let size = size_of::<Meshlet>();
            let i = i * size;
            let bytes = bytemuck::cast::<_, [u8; size_of::<Meshlet>()]>(Meshlet {
                start_vertex_position_bit: meshlet.start_vertex_position_bit
                    + vertex_position_offset,
                start_vertex_attribute_id: meshlet.start_vertex_attribute_id
                    + vertex_attribute_offset,
                start_index_id: meshlet.start_index_id + index_offset,
                ..*meshlet
            });
            buffer_slice[i..(i + size)].clone_from_slice(&bytes);
        }
    }
}

impl PersistentGpuBufferable for Arc<[u8]> {
    type Metadata = ();

    fn size_in_bytes(&self) -> usize {
        self.len()
    }

    fn write_bytes_le(&self, _: Self::Metadata, buffer_slice: &mut [u8]) {
        buffer_slice.clone_from_slice(self);
    }
}

impl PersistentGpuBufferable for Arc<[u32]> {
    type Metadata = ();

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<u32>()
    }

    fn write_bytes_le(&self, _: Self::Metadata, buffer_slice: &mut [u8]) {
        buffer_slice.clone_from_slice(bytemuck::cast_slice(self));
    }
}

impl PersistentGpuBufferable for Arc<[Vec2]> {
    type Metadata = ();

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<Vec2>()
    }

    fn write_bytes_le(&self, _: Self::Metadata, buffer_slice: &mut [u8]) {
        buffer_slice.clone_from_slice(bytemuck::cast_slice(self));
    }
}

impl PersistentGpuBufferable for Arc<[MeshletBoundingSpheres]> {
    type Metadata = ();

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<MeshletBoundingSpheres>()
    }

    fn write_bytes_le(&self, _: Self::Metadata, buffer_slice: &mut [u8]) {
        buffer_slice.clone_from_slice(bytemuck::cast_slice(self));
    }
}

impl PersistentGpuBufferable for Arc<[MeshletSimplificationError]> {
    type Metadata = ();

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<MeshletSimplificationError>()
    }

    fn write_bytes_le(&self, _: Self::Metadata, buffer_slice: &mut [u8]) {
        buffer_slice.clone_from_slice(bytemuck::cast_slice(self));
    }
}
