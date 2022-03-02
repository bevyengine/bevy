use std::num::NonZeroU64;

use bevy_crevice::std430::{self, AsStd430, Std430};
use bevy_utils::tracing::warn;
use wgpu::{BindingResource, BufferBinding, BufferDescriptor, BufferUsages};

use crate::renderer::{RenderDevice, RenderQueue};

use super::Buffer;

/// A helper for a storage buffer binding with a body, or a variable-sized array, or both.
pub struct StorageBuffer<T: AsStd430, U: AsStd430> {
    body: T,
    values: Vec<U>,
    scratch: Vec<u8>,
    storage_buffer: Option<Buffer>,
    capacity: usize,
    item_size: usize,
}

impl<T: AsStd430 + Default, U: AsStd430> Default for StorageBuffer<T, U> {
    fn default() -> Self {
        Self {
            body: T::default(),
            values: Vec::new(),
            scratch: Vec::new(),
            storage_buffer: None,
            capacity: 0,
            item_size: U::std430_size_static(),
        }
    }
}

impl<T: AsStd430, U: AsStd430> StorageBuffer<T, U> {
    #[inline]
    pub fn storage_buffer(&self) -> Option<&Buffer> {
        self.storage_buffer.as_ref()
    }

    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(BufferBinding {
            buffer: self.storage_buffer()?,
            offset: 0,
            size: Some(NonZeroU64::new((self.size()) as u64).unwrap()),
        }))
    }

    #[inline]
    pub fn set_body(&mut self, body: T) {
        self.body = body;
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

    pub fn push(&mut self, value: U) -> usize {
        let index = self.values.len();
        self.values.push(value);
        index
    }

    pub fn get_mut(&mut self, index: usize) -> &mut U {
        &mut self.values[index]
    }

    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) -> bool {
        if self.storage_buffer.is_none() || capacity > self.capacity {
            self.capacity = capacity;
            let size = self.size();
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
        size += T::std430_size_static();
        if size > 0 {
            // Pad according to the array item type's alignment
            size = (size + <U as AsStd430>::Output::ALIGNMENT - 1)
                & !(<U as AsStd430>::Output::ALIGNMENT - 1);
        }
        // Variable size arrays must have at least 1 element
        size += self.item_size * self.capacity.max(1);
        size
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.reserve(self.values.len(), device);
        if let Some(storage_buffer) = &self.storage_buffer {
            let range = 0..self.size();
            let mut writer = std430::Writer::new(&mut self.scratch[range.clone()]);
            let mut offset = 0;
            // First write the struct body if there is one
            if T::std430_size_static() > 0 {
                if let Ok(new_offset) = writer.write(&self.body).map_err(|e| warn!("{:?}", e)) {
                    offset = new_offset;
                }
            }
            if self.values.is_empty() {
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
            queue.write_buffer(storage_buffer, 0, &self.scratch[range]);
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn values(&self) -> &[U] {
        &self.values
    }
}
