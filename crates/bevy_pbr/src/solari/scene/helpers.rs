use bevy_math::Mat4;
use bevy_render::{
    render_resource::{encase::private::WriteInto, ShaderSize, StorageBuffer},
    renderer::{RenderDevice, RenderQueue},
};
use bevy_utils::HashMap;
use std::{fmt::Debug, hash::Hash};

pub struct IndexedVec<T, K, I>
where
    K: Hash + Eq + Clone,
    I: TryFrom<usize> + Copy,
    <I as TryFrom<usize>>::Error: Debug,
{
    pub vec: Vec<T>,
    pub index: HashMap<K, I>,
}

impl<T, K, I> IndexedVec<T, K, I>
where
    K: Hash + Eq + Clone,
    I: TryFrom<usize> + Copy,
    <I as TryFrom<usize>>::Error: Debug,
{
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            index: HashMap::new(),
        }
    }

    pub fn get_index<F: FnOnce(K) -> T>(&mut self, index_key: K, create_value: F) -> I {
        *self.index.entry(index_key.clone()).or_insert_with(|| {
            // TODO: Validate we haven't gone over 2^16/32 items (-1 for textures)
            let i = self.vec.len().try_into().unwrap();
            self.vec.push(create_value(index_key));
            i
        })
    }
}

pub fn pack_object_indices(mesh_index: u16, material_index: u16) -> u32 {
    ((mesh_index as u32) << 16) | (material_index as u32)
}

pub fn new_storage_buffer<T: ShaderSize + WriteInto>(
    vec: Vec<T>,
    label: &'static str,
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
) -> StorageBuffer<Vec<T>> {
    let mut buffer = StorageBuffer::from(vec);
    buffer.set_label(Some(label));
    buffer.write_buffer(render_device, render_queue);
    buffer
}

pub fn tlas_transform(transform: &Mat4) -> [f32; 12] {
    transform.transpose().to_cols_array()[..12]
        .try_into()
        .unwrap()
}
