use super::{
    binding_types::{storage_buffer_read_only, uniform_buffer_sized},
    BindGroupLayoutEntryBuilder, StorageBuffer,
};
use crate::{
    render_resource::{
        batched_uniform_buffer::{
            BatchedUniformBuffer, BatchedUniformBufferPool, BatchedUniformBufferWriter,
        },
        BufferPoolSlice, StorageBufferPool, StorageBufferWriter,
    },
    renderer::{RenderDevice, RenderQueue},
};
use bevy_ecs::{prelude::Component, system::Resource};
use encase::{private::WriteInto, ShaderSize, ShaderType};
use nonmax::NonMaxU32;
use std::{marker::PhantomData, num::NonZeroU64};
use wgpu::BindingResource;

/// Trait for types able to go in a [`GpuArrayBuffer`].
pub trait GpuArrayBufferable: ShaderType + ShaderSize + WriteInto + Clone {}
impl<T: ShaderType + ShaderSize + WriteInto + Clone> GpuArrayBufferable for T {}

/// Stores an array of elements to be transferred to the GPU and made accessible to shaders as a read-only array.
///
/// On platforms that support storage buffers, this is equivalent to [`StorageBuffer<Vec<T>>`].
/// Otherwise, this falls back to a dynamic offset uniform buffer with the largest
/// array of T that fits within a uniform buffer binding (within reasonable limits).
///
/// Other options for storing GPU-accessible data are:
/// * [`StorageBuffer`]
/// * [`DynamicStorageBuffer`](crate::render_resource::DynamicStorageBuffer)
/// * [`UniformBuffer`](crate::render_resource::UniformBuffer)
/// * [`DynamicUniformBuffer`](crate::render_resource::DynamicUniformBuffer)
/// * [`BufferVec`](crate::render_resource::BufferVec)
/// * [`Texture`](crate::render_resource::Texture)
#[derive(Resource)]
pub enum GpuArrayBuffer<T: GpuArrayBufferable> {
    Uniform(BatchedUniformBuffer<T>),
    Storage(StorageBuffer<Vec<T>>),
}

impl<T: GpuArrayBufferable> GpuArrayBuffer<T> {
    pub fn new(device: &RenderDevice) -> Self {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage == 0 {
            GpuArrayBuffer::Uniform(BatchedUniformBuffer::new(&limits))
        } else {
            GpuArrayBuffer::Storage(StorageBuffer::default())
        }
    }

    pub fn clear(&mut self) {
        match self {
            GpuArrayBuffer::Uniform(buffer) => buffer.clear(),
            GpuArrayBuffer::Storage(buffer) => buffer.get_mut().clear(),
        }
    }

    pub fn push(&mut self, value: T) -> GpuArrayBufferIndex<T> {
        match self {
            GpuArrayBuffer::Uniform(buffer) => buffer.push(value),
            GpuArrayBuffer::Storage(buffer) => {
                let buffer = buffer.get_mut();
                let index = buffer.len() as u32;
                buffer.push(value);
                GpuArrayBufferIndex {
                    index,
                    dynamic_offset: None,
                    element_type: PhantomData,
                }
            }
        }
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        match self {
            GpuArrayBuffer::Uniform(buffer) => buffer.write_buffer(device, queue),
            GpuArrayBuffer::Storage(buffer) => buffer.write_buffer(device, queue),
        }
    }

    pub fn binding_layout(device: &RenderDevice) -> BindGroupLayoutEntryBuilder {
        if device.limits().max_storage_buffers_per_shader_stage == 0 {
            uniform_buffer_sized(
                true,
                // BatchedUniformBuffer uses a MaxCapacityArray that is runtime-sized, so we use
                // None here and let wgpu figure out the size.
                None,
            )
        } else {
            storage_buffer_read_only::<T>(false)
        }
    }

    pub fn binding(&self) -> Option<BindingResource> {
        match self {
            GpuArrayBuffer::Uniform(buffer) => buffer.binding(),
            GpuArrayBuffer::Storage(buffer) => buffer.binding(),
        }
    }

    pub fn batch_size(device: &RenderDevice) -> Option<u32> {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage == 0 {
            Some(BatchedUniformBuffer::<T>::batch_size(&limits) as u32)
        } else {
            None
        }
    }
}

/// An index into a [`GpuArrayBuffer`] for a given element.
#[derive(Component, Clone)]
pub struct GpuArrayBufferIndex<T: GpuArrayBufferable> {
    /// The index to use in a shader into the array.
    pub index: u32,
    /// The dynamic offset to use when setting the bind group in a pass.
    /// Only used on platforms that don't support storage buffers.
    pub dynamic_offset: Option<NonMaxU32>,
    pub element_type: PhantomData<T>,
}

#[derive(Resource)]
pub enum GpuArrayBufferPool<T: GpuArrayBufferable> {
    Uniform(BatchedUniformBufferPool<T>),
    Storage(StorageBufferPool<T>),
}

impl<T: GpuArrayBufferable> GpuArrayBufferPool<T> {
    pub fn new(device: &RenderDevice) -> Self {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage == 0 {
            GpuArrayBufferPool::Uniform(BatchedUniformBufferPool::new(&limits))
        } else {
            GpuArrayBufferPool::Storage(StorageBufferPool::default())
        }
    }

    pub fn clear(&mut self) {
        match self {
            GpuArrayBufferPool::Uniform(buffer) => buffer.clear(),
            GpuArrayBufferPool::Storage(buffer) => buffer.clear(),
        }
    }

    pub fn binding_layout(device: &RenderDevice) -> BindGroupLayoutEntryBuilder {
        if device.limits().max_storage_buffers_per_shader_stage == 0 {
            uniform_buffer_sized(
                true,
                // BatchedUniformBuffer uses a MaxCapacityArray that is runtime-sized, so we use
                // None here and let wgpu figure out the size.
                None,
            )
        } else {
            storage_buffer_read_only::<T>(false)
        }
    }

    pub fn binding(&self) -> Option<BindingResource> {
        match self {
            GpuArrayBufferPool::Uniform(buffer) => buffer.binding(),
            GpuArrayBufferPool::Storage(buffer) => buffer.binding(),
        }
    }

    pub fn reserve(&mut self, count: NonZeroU64) -> BufferPoolSlice {
        match self {
            GpuArrayBufferPool::Uniform(buffer) => buffer.reserve(count),
            GpuArrayBufferPool::Storage(buffer) => buffer.reserve(count),
        }
    }

    pub fn allocate(&mut self, device: &RenderDevice) {
        match self {
            GpuArrayBufferPool::Uniform(buffer) => buffer.allocate(device),
            GpuArrayBufferPool::Storage(buffer) => buffer.allocate(device),
        }
    }

    #[inline]
    pub fn get_writer<'a>(
        &'a self,
        slice: BufferPoolSlice,
        queue: &'a RenderQueue,
    ) -> Option<GpuArrayBufferWriter<'a, T>> {
        Some(match self {
            GpuArrayBufferPool::Uniform(buffer) => {
                GpuArrayBufferWriter::Uniform(buffer.get_writer(slice, queue)?)
            }
            GpuArrayBufferPool::Storage(buffer) => {
                GpuArrayBufferWriter::Storage(buffer.get_writer(slice, queue)?)
            }
        })
    }

    pub fn batch_size(device: &RenderDevice) -> Option<u32> {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage == 0 {
            Some(BatchedUniformBuffer::<T>::batch_size(&limits) as u32)
        } else {
            None
        }
    }
}

#[derive(Resource)]
pub enum GpuArrayBufferWriter<'a, T: GpuArrayBufferable> {
    Uniform(BatchedUniformBufferWriter<'a, T>),
    Storage(StorageBufferWriter<'a, T>),
}

impl<'a, T: GpuArrayBufferable> GpuArrayBufferWriter<'a, T> {
    pub fn write(&mut self, value: T) -> GpuArrayBufferIndex<T> {
        match self {
            GpuArrayBufferWriter::Uniform(buffer) => buffer.write(value),
            GpuArrayBufferWriter::Storage(buffer) => {
                let index = buffer.current_index() as u32;
                buffer.write(&value);
                GpuArrayBufferIndex {
                    index,
                    dynamic_offset: None,
                    element_type: PhantomData,
                }
            }
        }
    }
}
