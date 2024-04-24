use super::{
    asset::{Meshlet, MeshletBoundingSpheres},
    persistent_buffer::PersistentGpuBufferable,
};
use std::{mem::size_of, sync::Arc};

const MESHLET_VERTEX_SIZE_IN_BYTES: u32 = 48;

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
    type Metadata = u64;

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<u32>()
    }

    fn write_bytes_le(&self, offset: Self::Metadata, buffer_slice: &mut [u8]) {
        let offset = offset as u32 / MESHLET_VERTEX_SIZE_IN_BYTES;

        for (i, index) in self.iter().enumerate() {
            let size = size_of::<u32>();
            let i = i * size;
            let bytes = (*index + offset).to_le_bytes();
            buffer_slice[i..(i + size)].clone_from_slice(&bytes);
        }
    }
}

impl PersistentGpuBufferable for Arc<[Meshlet]> {
    type Metadata = (u64, u64);

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<Meshlet>()
    }

    fn write_bytes_le(
        &self,
        (vertex_offset, index_offset): Self::Metadata,
        buffer_slice: &mut [u8],
    ) {
        let vertex_offset = (vertex_offset as usize / size_of::<u32>()) as u32;
        let index_offset = index_offset as u32;

        for (i, meshlet) in self.iter().enumerate() {
            let size = size_of::<Meshlet>();
            let i = i * size;
            let bytes = bytemuck::cast::<_, [u8; size_of::<Meshlet>()]>(Meshlet {
                start_vertex_id: meshlet.start_vertex_id + vertex_offset,
                start_index_id: meshlet.start_index_id + index_offset,
                triangle_count: meshlet.triangle_count,
            });
            buffer_slice[i..(i + size)].clone_from_slice(&bytes);
        }
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
