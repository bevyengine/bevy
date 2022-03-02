use crate::{
    render_resource::std140::{self, AsStd140, DynamicUniform, Std140},
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};
use std::{
    num::NonZeroU64,
    ops::{Deref, DerefMut},
};
use wgpu::{BindingResource, BufferBinding, BufferDescriptor, BufferUsages};

/// A user-friendly wrapper around a [`Buffer`] that provides a `Vec`-like
/// interface for constructing the buffer.
///
/// Intended strictly for use with uniform buffers. For other use cases,
/// see [`BufferVec`][buffervec] instead.
///
/// [buffervec]: crate::render_resource::BufferVec
pub struct UniformVec<T: AsStd140> {
    values: Vec<T>,
    scratch: Vec<u8>,
    uniform_buffer: Option<Buffer>,
}

impl<T: AsStd140> Default for UniformVec<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            scratch: Vec::new(),
            uniform_buffer: None,
        }
    }
}

impl<T: AsStd140> UniformVec<T> {
    const ITEM_SIZE: usize =
        (std::mem::size_of::<T::Output>() + <T as AsStd140>::Output::ALIGNMENT - 1)
            & !(<T as AsStd140>::Output::ALIGNMENT - 1);

    /// Gets the reference to the underlying buffer, if one has been allocated.
    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.uniform_buffer.as_ref()
    }

    /// Creates a binding for the underlying buffer.
    /// Returns `None` if no buffer has been allocated.
    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(BufferBinding {
            buffer: self.buffer()?,
            offset: 0,
            size: Some(NonZeroU64::new(Self::ITEM_SIZE as u64).unwrap()),
        }))
    }

    /// Gets the capacity of the underlying buffer.
    ///
    /// Will return 0 if no buffer has been allocated yet.
    #[inline]
    pub fn buffer_capacity(&self) -> usize {
        self.values.len()
    }

    pub fn push(&mut self, value: T) -> usize {
        let index = self.values.len();
        self.values.push(value);
        index
    }

    /// Queues up a copy of the contents of the [`UniformVec`] into the underlying
    /// buffer.
    ///
    /// If no buffer has been allocated yet or if the current size of the contents
    /// exceeds the size of the underlying buffer, a new buffer will be allocated.
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.values.is_empty() {
            return;
        }
        self.reserve_buffer(self.values.len(), device);
        if let Some(uniform_buffer) = &self.uniform_buffer {
            let range = 0..Self::ITEM_SIZE * self.values.len();
            let mut writer = std140::Writer::new(&mut self.scratch[range.clone()]);
            writer.write(self.values.as_slice()).unwrap();
            queue.write_buffer(uniform_buffer, 0, &self.scratch[range]);
        }
    }

    fn reserve_buffer(&mut self, capacity: usize, device: &RenderDevice) -> bool {
        if capacity > self.scratch.len() {
            self.capacity = capacity;
            let size = Self::ITEM_SIZE * capacity;
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
}

impl<T: AsStd140> Deref for UniformVec<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl<T: AsStd140> DerefMut for UniformVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
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
    /// Gets the reference to the underlying buffer, if one has been allocated.
    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.uniform_vec.buffer()
    }

    /// Creates a binding for the underlying buffer.
    /// Returns `None` if no buffer has been allocated.
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

    /// Gets the capacity of the underlying buffer, in bytes.
    #[inline]
    pub fn buffer_capacity(&self) -> usize {
        self.uniform_vec.buffer_capacity()
    }

    #[inline]
    pub fn push(&mut self, value: T) -> u32 {
        (self.uniform_vec.push(DynamicUniform(value)) * UniformVec::<DynamicUniform<T>>::ITEM_SIZE)
            as u32
    }

    /// Queues up a copy of the contents of the [`UniformVec`] into the underlying
    /// buffer.
    ///
    /// If no buffer has been allocated yet or if the current size of the contents
    /// exceeds the size of the underlying buffer, a new buffer will be allocated.
    #[inline]
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.uniform_vec.write_buffer(device, queue);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.uniform_vec.clear();
    }
}
