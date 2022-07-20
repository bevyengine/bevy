use super::Buffer;
use crate::renderer::{RenderDevice, RenderQueue};
use encase::{
    internal::WriteInto, DynamicStorageBuffer as DynamicStorageBufferWrapper, ShaderType,
    StorageBuffer as StorageBufferWrapper,
};
use wgpu::{util::BufferInitDescriptor, BindingResource, BufferBinding, BufferUsages};

/// Stores data to be transferred to the GPU and made accessible to shaders as a storage buffer.
///
/// Storage buffers can be made available to shaders as some combination of read/write, unlike
/// [`UniformBuffer`](crate::render_resource::UniformBuffer) which is read-only. Furthermore, storage buffers
/// can store much larger data than uniform buffers, which are best suited to relatively small data. Note however that
/// WebGL2 does not support storage buffers, so other alternatives to consider are vertex/instance buffers (see
/// [`BufferVec`](crate::render_resource::BufferVec)), or data textures ([`Texture`](crate::render_resource::Texture)),
/// depending on what is most appropriate for the use case.
///
/// Storage buffers can store runtime-sized arrays, but only if they are the last field in a structure. To store a
/// runtime-sized array of data, use [`DynamicStorageBuffer`](crate::render_resource::DynamicStorageBuffer) instead.
///
/// The contained data is stored in system RAM. [`write_buffer`](crate::render_resource::StorageBuffer::write_buffer) queues
/// copying of the data from system RAM to VRAM. Storage buffers must conform to [std430 alignment/padding requirements], which
/// is automatically enforced by this structure. If data does not need to be automatically padded or aligned,
/// consider using [`BufferVec`](crate::render_resource::BufferVec).
///
/// [std430 alignment/padding requirements]: https://www.w3.org/TR/WGSL/#address-spaces-storage
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

    /// Queues writing of data from system RAM to VRAM using the [`RenderDevice`](crate::renderer::RenderDevice)
    /// and the provided [`RenderQueue`](crate::renderer::RenderQueue).
    ///
    /// If there is no GPU-side buffer allocated to hold the data currently stored, or if a GPU-side buffer previously
    /// allocated does not have enough capacity, a new GPU-side buffer is created.
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

/// Stores data to be transferred to the GPU and made accessible to shaders as a dynamic storage buffer.
///
/// Dynamic storage buffers can be made available to shaders as some combination of read/write, unlike
/// [`UniformBuffer`](crate::render_resource::UniformBuffer) which is read-only. Furthermore, dynamic storage buffers
/// can store much larger data than uniform buffers, which are only guaranteed to be up to 16kB per binding. Dynamic storage buffers
/// support multiple separate bindings at dynamic byte offsets and so have a
/// [`push`](crate::render_resource::DynamicStorageBuffer::push) method, unlike
/// [`StorageBuffer`](crate::render_resource::StorageBuffer). Note however that
/// WebGL2 does not support dynamic storage buffers, so other alternatives to consider are vertex/instance buffers (see
/// [`BufferVec`](crate::render_resource::BufferVec)), or data textures ([`Texture`](crate::render_resource::Texture)),
/// depending on what is most appropriate for the use case.
///
/// The contained data is stored in system RAM. [`write_buffer`](crate::render_resource::DynamicStorageBuffer::write_buffer)
/// queues copying of the data from system RAM to VRAM. Dynamic storage buffers must conform to
/// [std430 alignment/padding requirements]; whenever data is [`push`](crate::render_resource::DynamicStorageBuffer::push)ed
/// into this structure, it is automatically aligned to these requirements. If data does not need to
/// be automatically padded or aligned, consider using [`BufferVec`](crate::render_resource::BufferVec).
///
/// [std430 alignment/padding requirements]: https://www.w3.org/TR/WGSL/#address-spaces-storage
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
