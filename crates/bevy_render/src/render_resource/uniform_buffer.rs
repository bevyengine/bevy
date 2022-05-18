use crate::{
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};
use encase::{
    internal::WriteInto, DynamicUniformBuffer as DynamicUniformBufferWrapper, ShaderType,
    UniformBuffer as UniformBufferWrapper,
};
use wgpu::{util::BufferInitDescriptor, BindingResource, BufferBinding, BufferUsages};

pub struct UniformBuffer<T: ShaderType> {
    value: T,
    scratch: UniformBufferWrapper<Vec<u8>>,
    buffer: Option<Buffer>,
}

impl<T: ShaderType> From<T> for UniformBuffer<T> {
    fn from(value: T) -> Self {
        Self {
            value,
            scratch: UniformBufferWrapper::new(Vec::new()),
            buffer: None,
        }
    }
}

impl<T: ShaderType + Default> Default for UniformBuffer<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            scratch: UniformBufferWrapper::new(Vec::new()),
            buffer: None,
        }
    }
}

impl<T: ShaderType + WriteInto> UniformBuffer<T> {
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

        match &self.buffer {
            Some(buffer) => queue.write_buffer(buffer, 0, self.scratch.as_ref()),
            None => {
                self.buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
                    label: None,
                    usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                    contents: self.scratch.as_ref(),
                }));
            }
        }
    }
}

pub struct DynamicUniformBuffer<T: ShaderType> {
    values: Vec<T>,
    scratch: DynamicUniformBufferWrapper<Vec<u8>>,
    buffer: Option<Buffer>,
    capacity: usize,
}

impl<T: ShaderType> Default for DynamicUniformBuffer<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            scratch: DynamicUniformBufferWrapper::new(Vec::new()),
            buffer: None,
            capacity: 0,
        }
    }
}

impl<T: ShaderType + WriteInto> DynamicUniformBuffer<T> {
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
                usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
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
