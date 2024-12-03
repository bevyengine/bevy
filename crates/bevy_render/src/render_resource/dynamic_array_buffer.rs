use core::num::NonZero;

use encase::ShaderType;
use wgpu::{BindingResource, Limits};

use crate::{
    render_resource::DynamicUniformBuffer,
    renderer::{RenderDevice, RenderQueue},
};

use super::{batched_uniform_buffer::MaxCapacityArray, Buffer, GpuArrayBufferable};

/// Similar to [`DynamicUniformBuffer`] except designed for multiple runtime sized arrays, allowing manually batched arrays to be accessed as
/// `array<T, N>` in WGSL where N is the size of the largest array. Arrays smaller than N have the rest of the array uninitialized.
pub struct DynamicArrayUniformBuffer<T: GpuArrayBufferable> {
    uniforms: DynamicUniformBuffer<MaxCapacityArray<Vec<T>>>,
    temp: Vec<MaxCapacityArray<Vec<T>>>,
    offsets: Vec<u32>,
    is_queuing_finished: bool,
}

impl<T: GpuArrayBufferable> DynamicArrayUniformBuffer<T> {
    pub fn new(limits: &Limits) -> Self {
        let alignment = limits.min_uniform_buffer_offset_alignment;

        Self {
            uniforms: DynamicUniformBuffer::new_with_alignment(alignment as u64),
            temp: vec![],
            offsets: vec![],
            is_queuing_finished: false,
        }
    }

    pub fn clear(&mut self) {
        self.uniforms.clear();
        self.offsets.clear();
        self.is_queuing_finished = false;
        self.temp.clear();
    }

    pub fn is_queuing_finished(&self) -> bool {
        self.is_queuing_finished
    }

    /// Returns the stored array that currently has the largest length.
    /// Please note that unless [`is_queuing_finished`](Self::is_queuing_finished) returns true,
    /// then this value is subject to change as more elements are added.
    pub fn current_max_capacity(&self) -> usize {
        self.temp
            .iter()
            .fold(0usize, |size, array| size.max(array.0.len()))
    }

    /// Returns the current size of the arrays.
    /// Please note that unless [`is_queuing_finished`](Self::is_queuing_finished) returns true,
    /// then this value is subject to change as more elements are added.
    pub fn current_size(&self) -> NonZero<u64> {
        Vec::<T>::METADATA
            .stride()
            .mul(self.current_max_capacity().max(1) as u64)
            .0
    }

    /// Creates a new array and returns its index. Panics if [`is_queuing_finished`](Self::is_queuing_finished) returns true.
    pub fn new_array(&mut self) -> DynamicArrayIndex {
        if self.is_queuing_finished() {
            panic!("Cannot create new arrays when the queuing has already been completed. Please clear the array first before attempting to add more elements.")
        }

        self.temp.push(MaxCapacityArray(vec![], 0));
        DynamicArrayIndex(self.temp.len() - 1)
    }

    /// Allows you to push an entire array at once. Panics if [`is_queuing_finished`](Self::is_queuing_finished) returns true.
    pub fn push_array(&mut self, array: Vec<T>) -> DynamicArrayIndex {
        if self.is_queuing_finished() {
            panic!("Cannot create new arrays when the queuing has already been completed. Please clear the array first before attempting to add more elements.")
        }

        self.temp.push(MaxCapacityArray(array, 0));
        DynamicArrayIndex(self.temp.len() - 1)
    }

    /// Pushes an element to the specified array. Panics if [`is_queuing_finished`](Self::is_queuing_finished) returns true.
    pub fn push_element(&mut self, array: DynamicArrayIndex, element: T) {
        if self.is_queuing_finished() {
            panic!("Cannot add elements to array when the queuing has already been completed. Please clear the array first before attempting to add more elements.")
        }

        self.temp[array.0].0.push(element);
    }

    /// Finishes queuing and gets data ready to be written to a buffer. Should be called once all arrays have been created and all elements have been pushed.
    pub fn finish_queuing(&mut self) {
        if !self.is_queuing_finished() {
            let capacity = self.current_max_capacity();
            for array in &mut self.temp {
                array.1 = capacity;
                self.offsets.push(self.uniforms.push(&*array));
            }
            self.is_queuing_finished = true;
        }
    }

    pub fn get_array_offset(&self, index: DynamicArrayIndex) -> u32 {
        self.offsets[index.0]
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.uniforms.write_buffer(device, queue);
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        self.uniforms.buffer()
    }

    pub fn binding(&self) -> Option<BindingResource> {
        if !self.is_queuing_finished() {
            return None;
        }
        let mut binding = self.uniforms.binding();
        if let Some(BindingResource::Buffer(binding)) = &mut binding {
            // MaxCapacityArray is runtime-sized so can't use T::min_size()
            binding.size = Some(self.current_size());
        }
        binding
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DynamicArrayIndex(usize);
