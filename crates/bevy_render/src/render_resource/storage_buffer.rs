use super::Buffer;
use crate::renderer::{RenderDevice, RenderQueue};
use encase::{
    internal::WriteInto, DynamicStorageBuffer as DynamicStorageBufferWrapper, ShaderType,
    StorageBuffer as StorageBufferWrapper,
};
use wgpu::{util::BufferInitDescriptor, BindingResource, BufferBinding, BufferUsages};

pub struct StorageBuffer<T: ShaderType> {
    value: T,
    scratch: StorageBufferWrapper<Vec<u8>>,
    buffer: Option<Buffer>,
    capacity: usize,
}

impl<T: ShaderType> From<T> for StorageBuffer<T> {
    fn from(value: T) -> Self {
        Self {
            value,
            scratch: StorageBufferWrapper::new(Vec::new()),
            buffer: None,
            capacity: 0,
        }
    }
}

impl<T: ShaderType + Default> Default for StorageBuffer<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            scratch: StorageBufferWrapper::new(Vec::new()),
            buffer: None,
            capacity: 0,
        }
    }
}

impl<T: ShaderType + WriteInto> StorageBuffer<T> {
    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(
            self.buffer()?.as_entire_buffer_binding(),
        ))
    }

    pub fn set(&mut self, value: T) {
        self.value = value;
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.scratch.write(&self.value).unwrap();

        let size = self.scratch.as_ref().len();

        if self.capacity < size {
            self.buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
                label: None,
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                contents: self.scratch.as_ref(),
            }));
            self.capacity = size;
        } else if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, self.scratch.as_ref());
        }
    }
}

pub struct DynamicStorageBuffer<T: ShaderType> {
    values: Vec<T>,
    scratch: DynamicStorageBufferWrapper<Vec<u8>>,
    buffer: Option<Buffer>,
    capacity: usize,
}

impl<T: ShaderType> Default for DynamicStorageBuffer<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            scratch: DynamicStorageBufferWrapper::new(Vec::new()),
            buffer: None,
            capacity: 0,
        }
    }
}

impl<T: ShaderType + WriteInto> DynamicStorageBuffer<T> {
    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(BufferBinding {
            buffer: self.buffer()?,
            offset: 0,
            size: Some(T::min_size()),
        }))
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    #[inline]
    pub fn push(&mut self, value: T) -> u32 {
        let offset = self.scratch.write(&value).unwrap() as u32;
        self.values.push(value);
        offset
    }

    #[inline]
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        let size = self.scratch.as_ref().len();

        if self.capacity < size {
            self.buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
                label: None,
                usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
                contents: self.scratch.as_ref(),
            }));
            self.capacity = size;
        } else if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, self.scratch.as_ref());
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.values.clear();
        self.scratch.as_mut().clear();
        self.scratch.set_offset(0);
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
