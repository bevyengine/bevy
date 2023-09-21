use super::StorageBuffer;
use crate::{
    render_resource::batched_uniform_buffer::BatchedUniformBuffer,
    renderer::{RenderDevice, RenderQueue},
};
use bevy_ecs::{prelude::Component, system::Resource};
use bevy_utils::nonmax::NonMaxU32;
use encase::{private::WriteInto, ShaderSize, ShaderType};
use std::{marker::PhantomData, mem};
use wgpu::{BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, ShaderStages};

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
/// * [`StorageBuffer`](crate::render_resource::StorageBuffer)
/// * [`DynamicStorageBuffer`](crate::render_resource::DynamicStorageBuffer)
/// * [`UniformBuffer`](crate::render_resource::UniformBuffer)
/// * [`DynamicUniformBuffer`](crate::render_resource::DynamicUniformBuffer)
/// * [`BufferVec`](crate::render_resource::BufferVec)
/// * [`Texture`](crate::render_resource::Texture)
#[derive(Resource)]
pub enum GpuArrayBuffer<T: GpuArrayBufferable> {
    Uniform(BatchedUniformBuffer<T>),
    Storage((StorageBuffer<Vec<T>>, Vec<T>)),
}

impl<T: GpuArrayBufferable> GpuArrayBuffer<T> {
    pub fn new(device: &RenderDevice) -> Self {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage == 0 {
            GpuArrayBuffer::Uniform(BatchedUniformBuffer::new(&limits))
        } else {
            GpuArrayBuffer::Storage((StorageBuffer::default(), Vec::new()))
        }
    }

    pub fn clear(&mut self) {
        match self {
            GpuArrayBuffer::Uniform(buffer) => buffer.clear(),
            GpuArrayBuffer::Storage((_, buffer)) => buffer.clear(),
        }
    }

    pub fn push(&mut self, value: T) -> GpuArrayBufferIndex<T> {
        match self {
            GpuArrayBuffer::Uniform(buffer) => buffer.push(value),
            GpuArrayBuffer::Storage((_, buffer)) => {
                let index = NonMaxU32::new(buffer.len() as u32).unwrap();
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
            GpuArrayBuffer::Storage((buffer, vec)) => {
                buffer.set(mem::take(vec));
                buffer.write_buffer(device, queue);
            }
        }
    }

    pub fn binding_layout(
        binding: u32,
        visibility: ShaderStages,
        device: &RenderDevice,
    ) -> BindGroupLayoutEntry {
        BindGroupLayoutEntry {
            binding,
            visibility,
            ty: if device.limits().max_storage_buffers_per_shader_stage == 0 {
                BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    // BatchedUniformBuffer uses a MaxCapacityArray that is runtime-sized, so we use
                    // None here and let wgpu figure out the size.
                    min_binding_size: None,
                }
            } else {
                BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: Some(T::min_size()),
                }
            },
            count: None,
        }
    }

    pub fn binding(&self) -> Option<BindingResource> {
        match self {
            GpuArrayBuffer::Uniform(buffer) => buffer.binding(),
            GpuArrayBuffer::Storage((buffer, _)) => buffer.binding(),
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
    pub index: NonMaxU32,
    /// The dynamic offset to use when setting the bind group in a pass.
    /// Only used on platforms that don't support storage buffers.
    pub dynamic_offset: Option<NonMaxU32>,
    pub element_type: PhantomData<T>,
}
