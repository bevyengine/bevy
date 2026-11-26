//! Typed access to [`ShaderBuffer`] records.

use alloc::vec::Vec;
use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use super::{ShaderBuffer, ShaderBufferData};
use crate::render_resource::GpuElement;

impl ShaderBuffer {
    /// Creates an initialized buffer of `T` records and remembers the record size.
    pub fn from_elements<T: GpuElement>(
        elements: Vec<T>,
        asset_usage: bevy_asset::RenderAssetUsages,
    ) -> Self {
        Self {
            element_size: size_of::<T>() as u32,
            ..Self::new(elements, asset_usage)
        }
    }

    /// A typed view of the records, or `None` if the buffer wasn't created with
    /// [`Self::from_elements`] of `T`, or its CPU data is absent or not whole records.
    pub fn view<T: GpuElement>(&mut self) -> Option<ShaderBufferView<'_, T>> {
        let ShaderBufferData::Initialized { data, buffer_size } = &self.data else {
            return None;
        };
        (self.element_size as usize == size_of::<T>() && data.len() as u64 == *buffer_size)
            .then_some(ShaderBufferView {
                buffer: self,
                marker: PhantomData,
            })
    }
}

/// A [`ShaderBuffer`]'s records as a `[T]`, from [`ShaderBuffer::view`], plus the
/// operations that change the length.
pub struct ShaderBufferView<'a, T> {
    buffer: &'a mut ShaderBuffer,
    marker: PhantomData<T>,
}

impl<T: GpuElement> Deref for ShaderBufferView<'_, T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.buffer.cast_slice().unwrap_or_default()
    }
}

impl<T: GpuElement> DerefMut for ShaderBufferView<'_, T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.buffer.cast_slice_mut().unwrap_or_default()
    }
}

impl<T: GpuElement> ShaderBufferView<'_, T> {
    /// Appends a record, growing the buffer.
    pub fn push(&mut self, value: T) {
        self.buffer.extend_from_slice(&[value]);
    }

    /// Appends records, growing the buffer.
    pub fn extend_from_slice(&mut self, values: &[T]) {
        self.buffer.extend_from_slice(values);
    }

    /// Removes all records, keeping the allocation.
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_resource::{debug_assert_std140, debug_assert_std430, ShaderType};
    use alloc::vec;
    use bevy_asset::RenderAssetUsages;
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, ShaderType)]
    struct Record {
        position: bevy_math::Vec2,
        id: u32,
        flags: u32,
    }

    crate::assert_gpu_layout!(record_is_std430, std430, Record, Scalars);
    crate::assert_gpu_layout!(record_is_std140, std140, Record);

    fn record(id: u32) -> Record {
        Record {
            position: bevy_math::Vec2::splat(id as f32),
            id,
            flags: 0,
        }
    }

    #[test]
    fn view_edits_records_in_place() {
        let mut buffer = ShaderBuffer::from_elements(vec![record(0); 2], RenderAssetUsages::all());
        let mut view = buffer.view::<Record>().unwrap();
        view[1] = record(1);
        view.push(record(2));
        view[..1].copy_from_slice(&[record(5)]);
        assert_eq!(*view, [record(5), record(1), record(2)]);
        assert_eq!(buffer.buffer_size(), 48);
    }

    #[test]
    fn view_requires_the_record_type() {
        let mut buffer = ShaderBuffer::from_elements(vec![1.0f32; 4], RenderAssetUsages::all());
        assert!(buffer.view::<Record>().is_none());
        assert!(buffer.view::<f32>().is_some());
        assert!(ShaderBuffer::new(vec![1.0f32; 4], RenderAssetUsages::all())
            .view::<f32>()
            .is_none());
    }

    #[test]
    fn accepts_matching_layouts() {
        debug_assert_std430::<Record>();
        debug_assert_std140::<Record>();
    }

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
    struct PackedVec3 {
        a: f32,
        v: bevy_math::Vec3,
        b: f32,
    }

    #[test]
    #[should_panic(expected = "std430")]
    fn rejects_an_unpadded_vec3() {
        debug_assert_std430::<PackedVec3>();
    }

    #[repr(C)]
    #[derive(Clone, Copy, Pod, Zeroable, ShaderType)]
    struct Scalars {
        values: [f32; 4],
    }

    #[test]
    fn std430_packs_scalar_arrays_that_std140_rejects() {
        debug_assert_std430::<Scalars>();
        assert!(std::panic::catch_unwind(|| debug_assert_std140::<Scalars>()).is_err());
    }
}
