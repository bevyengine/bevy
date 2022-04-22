use super::Buffer;
use crate::renderer::{RenderDevice, RenderQueue};
use bevy_crevice::std430::{self, AsStd430, Std430};
use bevy_utils::tracing::warn;
use std::num::NonZeroU64;
use wgpu::{BindingResource, BufferBinding, BufferDescriptor, BufferUsages};

/// A helper for a storage buffer binding with a body, or a variable-sized array, or both.
pub struct StorageBuffer<T: AsStd430, U: AsStd430 = ()> {
    body: U,
    values: Vec<T>,
    scratch: Vec<u8>,
    storage_buffer: Option<Buffer>,
}

impl<T: AsStd430, U: AsStd430 + Default> Default for StorageBuffer<T, U> {
    /// Creates a new [`StorageBuffer`]
    ///
    /// This does not immediately allocate system/video RAM buffers.
    fn default() -> Self {
        Self {
            body: U::default(),
            values: Vec::new(),
            scratch: Vec::new(),
            storage_buffer: None,
        }
    }
}

impl<T: AsStd430, U: AsStd430> StorageBuffer<T, U> {
    // NOTE: AsStd430::std430_size_static() uses size_of internally but trait functions cannot be
    // marked as const functions
    const BODY_SIZE: usize = std::mem::size_of::<U::Output>();
    const ITEM_SIZE: usize = std::mem::size_of::<T::Output>();

    /// Gets the reference to the underlying buffer, if one has been allocated.
    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.storage_buffer.as_ref()
    }

    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(BufferBinding {
            buffer: self.buffer()?,
            offset: 0,
            size: Some(NonZeroU64::new((self.size()) as u64).unwrap()),
        }))
    }

    #[inline]
    pub fn set_body(&mut self, body: U) {
        self.body = body;
    }

    fn reserve_buffer(&mut self, device: &RenderDevice) -> bool {
        let size = self.size();
        if self.storage_buffer.is_none() || size > self.scratch.len() {
            self.scratch.resize(size, 0);
            self.storage_buffer = Some(device.create_buffer(&BufferDescriptor {
                label: None,
                size: size as wgpu::BufferAddress,
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                mapped_at_creation: false,
            }));
            true
        } else {
            false
        }
    }

    fn size(&self) -> usize {
        let mut size = 0;
        size += Self::BODY_SIZE;
        if Self::ITEM_SIZE > 0 {
            if size > 0 {
                // Pad according to the array item type's alignment
                size = (size + <U as AsStd430>::Output::ALIGNMENT - 1)
                    & !(<U as AsStd430>::Output::ALIGNMENT - 1);
            }
            // Variable size arrays must have at least 1 element
            size += Self::ITEM_SIZE * self.values.len().max(1);
        }
        size
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.reserve_buffer(device);
        if let Some(storage_buffer) = &self.storage_buffer {
            let range = 0..self.size();
            let mut writer = std430::Writer::new(&mut self.scratch[range.clone()]);
            let mut offset = 0;
            // First write the struct body if there is one
            if Self::BODY_SIZE > 0 {
                if let Ok(new_offset) = writer.write(&self.body).map_err(|e| warn!("{:?}", e)) {
                    offset = new_offset;
                }
            }
            if Self::ITEM_SIZE > 0 {
                if self.values.is_empty() {
                    // Zero-out the padding and dummy array item in the case of the array being empty
                    for i in offset..self.size() {
                        self.scratch[i] = 0;
                    }
                } else {
                    // Then write the array. Note that padding bytes may be added between the body
                    // and the array in order to align the array to the alignment requirements of its
                    // items
                    writer
                        .write(self.values.as_slice())
                        .map_err(|e| warn!("{:?}", e))
                        .ok();
                }
            }
            queue.write_buffer(storage_buffer, 0, &self.scratch[range]);
        }
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut [T] {
        &mut self.values
    }

    #[inline]
    pub fn clear(&mut self) {
        self.values.clear();
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        self.values.push(value);
    }

    #[inline]
    pub fn append(&mut self, values: &mut Vec<T>) {
        self.values.append(values);
    }
}

#[cfg(test)]
mod tests {
    use super::StorageBuffer;
    use bevy_crevice::std430;
    use bevy_crevice::std430::AsStd430;
    use bevy_crevice::std430::Std430;
    use bevy_math::Vec3;
    use bevy_math::Vec4;

    //Note:
    //A Vec3 has 12 bytes and needs to be padded to 16 bytes, when converted to std430
    //https://www.w3.org/TR/WGSL/#alignment-and-size
    #[derive(AsStd430, Default)]
    struct NotInherentlyAligned {
        data: Vec3,
    }

    //Note:
    //A Vec4 has 16 bytes and does not need to be padded to fit in std430
    //https://www.w3.org/TR/WGSL/#alignment-and-size
    #[derive(AsStd430)]
    struct InherentlyAligned {
        data: Vec4,
    }

    #[test]
    fn storage_buffer_correctly_sized_nonaligned() {
        let mut buffer: StorageBuffer<NotInherentlyAligned> = StorageBuffer::default();
        buffer.push(NotInherentlyAligned { data: Vec3::ONE });

        let actual_size = buffer.size();

        let data = [NotInherentlyAligned { data: Vec3::ONE }].as_std430();
        let data_as_bytes = data.as_bytes();

        assert_eq!(actual_size, data_as_bytes.len());
    }

    #[test]
    fn storage_buffer_correctly_sized_aligned() {
        let mut buffer: StorageBuffer<InherentlyAligned> = StorageBuffer::default();
        buffer.push(InherentlyAligned { data: Vec4::ONE });

        let actual_size = buffer.size();

        let data = [InherentlyAligned { data: Vec4::ONE }].as_std430();
        let data_as_bytes = data.as_bytes();

        assert_eq!(actual_size, data_as_bytes.len());
    }

    #[test]
    fn storage_buffer_correctly_sized_item_and_body() {
        let mut buffer: StorageBuffer<NotInherentlyAligned, NotInherentlyAligned> =
            StorageBuffer::default();
        buffer.push(NotInherentlyAligned { data: Vec3::ONE });
        buffer.set_body(NotInherentlyAligned { data: Vec3::ONE });

        let calculated_size = buffer.size();

        //Emulate Write
        let mut scratch = Vec::<u8>::new();
        scratch.resize(calculated_size, 0);
        let mut writer = std430::Writer::new(&mut scratch[0..calculated_size]);
        writer
            .write(&buffer.body)
            .expect("Buffer has enough space to write the body.");
        writer
            .write(buffer.values.as_slice())
            .expect("Buffer has enough space to write the values.");
    }
}
