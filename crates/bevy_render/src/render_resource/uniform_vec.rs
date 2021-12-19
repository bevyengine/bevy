use crate::{
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};
use crevice::std140::{self, AsStd140, DynamicUniform, Std140};
use std::num::NonZeroU64;
use wgpu::{BindingResource, BufferBinding, BufferDescriptor, BufferUsages};

pub struct UniformVec<T: AsStd140> {
    values: Vec<T>,
    scratch: Vec<u8>,
    uniform_buffer: Option<Buffer>,
    capacity: usize,
    item_size: usize,
}

impl<T: AsStd140> Default for UniformVec<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            scratch: Vec::new(),
            uniform_buffer: None,
            capacity: 0,
            item_size: (T::std140_size_static() + <T as AsStd140>::Output::ALIGNMENT - 1)
                & !(<T as AsStd140>::Output::ALIGNMENT - 1),
        }
    }
}

impl<T: AsStd140> UniformVec<T> {
    #[inline]
    pub fn uniform_buffer(&self) -> Option<&Buffer> {
        self.uniform_buffer.as_ref()
    }

    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(BufferBinding {
            buffer: self.uniform_buffer()?,
            offset: 0,
            size: Some(NonZeroU64::new(self.item_size as u64).unwrap()),
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
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn push(&mut self, value: T) -> usize {
        let index = self.values.len();
        self.values.push(value);
        index
    }

    pub fn get_mut(&mut self, index: usize) -> &mut T {
        &mut self.values[index]
    }

    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) -> bool {
        if capacity > self.capacity {
            self.capacity = capacity;
            let size = self.item_size * capacity;
            self.scratch.resize(size, 0);
            self.uniform_buffer = Some(device.create_buffer(&BufferDescriptor {
                label: None,
                size: size as wgpu::BufferAddress,
                usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                mapped_at_creation: false,
            }));
            true
        } else {
            false
        }
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.values.is_empty() {
            return;
        }
        self.reserve(self.values.len(), device);
        if let Some(uniform_buffer) = &self.uniform_buffer {
            let range = 0..self.item_size * self.values.len();
            let mut writer = std140::Writer::new(&mut self.scratch[range.clone()]);
            writer.write(self.values.as_slice()).unwrap();
            queue.write_buffer(uniform_buffer, 0, &self.scratch[range]);
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }
}

pub struct DynamicUniformVec<T: AsStd140> {
    uniform_vec: UniformVec<DynamicUniform<T>>,
}

impl<T: AsStd140> Default for DynamicUniformVec<T> {
    fn default() -> Self {
        Self {
            uniform_vec: Default::default(),
        }
    }
}

impl<T: AsStd140> DynamicUniformVec<T> {
    #[inline]
    pub fn uniform_buffer(&self) -> Option<&Buffer> {
        self.uniform_vec.uniform_buffer()
    }

    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        self.uniform_vec.binding()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.uniform_vec.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.uniform_vec.is_empty()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.uniform_vec.capacity()
    }

    #[inline]
    pub fn push(&mut self, value: T) -> u32 {
        (self.uniform_vec.push(DynamicUniform(value)) * self.uniform_vec.item_size) as u32
    }

    #[inline]
    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        self.uniform_vec.reserve(capacity, device);
    }

    #[inline]
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.uniform_vec.write_buffer(device, queue);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.uniform_vec.clear();
    }
}
