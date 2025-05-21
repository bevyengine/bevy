use crate::meshlet::asset::{BvhNode, MeshletAabbErrorOffset, MeshletCullData};

use super::{asset::Meshlet, persistent_buffer::PersistentGpuBufferable};
use alloc::sync::Arc;
use bevy_math::Vec2;
use bevy_render::render_resource::BufferAddress;

impl PersistentGpuBufferable for Arc<[BvhNode]> {
    type Metadata = u32;

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<BvhNode>()
    }

    fn write_bytes_le(
        &self,
        base_meshlet_index: Self::Metadata,
        buffer_slice: &mut [u8],
        buffer_offset: BufferAddress,
    ) {
        let size = size_of::<BvhNode>();
        let base_bvh_node_index = (buffer_offset / size as u64) as u32;
        for (i, &node) in self.iter().enumerate() {
            let bytes = bytemuck::cast::<_, [u8; size_of::<BvhNode>()]>(BvhNode {
                aabbs: core::array::from_fn(|i| {
                    let aabb = node.aabbs[i];
                    MeshletAabbErrorOffset {
                        child_offset: aabb.child_offset
                            + if node.child_counts[i] == u8::MAX {
                                base_bvh_node_index
                            } else {
                                base_meshlet_index
                            },
                        ..aabb
                    }
                }),
                ..node
            });
            let i = i * size;
            buffer_slice[i..(i + size)].clone_from_slice(&bytes);
        }
    }
}

impl PersistentGpuBufferable for Arc<[Meshlet]> {
    type Metadata = (u64, u64, u64);

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<Meshlet>()
    }

    fn write_bytes_le(
        &self,
        (vertex_position_offset, vertex_attribute_offset, index_offset): Self::Metadata,
        buffer_slice: &mut [u8],
        _: BufferAddress,
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

impl PersistentGpuBufferable for Arc<[MeshletCullData]> {
    type Metadata = ();

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<MeshletCullData>()
    }

    fn write_bytes_le(&self, _: Self::Metadata, buffer_slice: &mut [u8], _: BufferAddress) {
        buffer_slice.clone_from_slice(bytemuck::cast_slice(self));
    }
}

impl PersistentGpuBufferable for Arc<[u8]> {
    type Metadata = ();

    fn size_in_bytes(&self) -> usize {
        self.len()
    }

    fn write_bytes_le(&self, _: Self::Metadata, buffer_slice: &mut [u8], _: BufferAddress) {
        buffer_slice.clone_from_slice(self);
    }
}

impl PersistentGpuBufferable for Arc<[u32]> {
    type Metadata = ();

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<u32>()
    }

    fn write_bytes_le(&self, _: Self::Metadata, buffer_slice: &mut [u8], _: BufferAddress) {
        buffer_slice.clone_from_slice(bytemuck::cast_slice(self));
    }
}

impl PersistentGpuBufferable for Arc<[Vec2]> {
    type Metadata = ();

    fn size_in_bytes(&self) -> usize {
        self.len() * size_of::<Vec2>()
    }

    fn write_bytes_le(&self, _: Self::Metadata, buffer_slice: &mut [u8], _: BufferAddress) {
        buffer_slice.clone_from_slice(bytemuck::cast_slice(self));
    }
}
