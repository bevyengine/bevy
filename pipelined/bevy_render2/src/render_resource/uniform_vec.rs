use crate::{render_resource::Buffer, renderer::RenderDevice};
use crevice::std140::{self, AsStd140, DynamicUniform, Std140};
use std::{num::NonZeroU64, ops::DerefMut};
use wgpu::{BindingResource, BufferBinding, BufferDescriptor, BufferUsages, CommandEncoder};

pub struct UniformVec<T: AsStd140> {
    values: Vec<T>,
    staging_buffer: Option<Buffer>,
    uniform_buffer: Option<Buffer>,
    capacity: usize,
    item_size: usize,
}

impl<T: AsStd140> Default for UniformVec<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            staging_buffer: None,
            uniform_buffer: None,
            capacity: 0,
            item_size: (T::std140_size_static() + <T as AsStd140>::Std140Type::ALIGNMENT - 1)
                & !(<T as AsStd140>::Std140Type::ALIGNMENT - 1),
        }
    }
}

impl<T: AsStd140> UniformVec<T> {
    #[inline]
    pub fn staging_buffer(&self) -> Option<&Buffer> {
        self.staging_buffer.as_ref()
    }

    #[inline]
    pub fn uniform_buffer(&self) -> Option<&Buffer> {
        self.uniform_buffer.as_ref()
    }

    #[inline]
    pub fn binding(&self) -> BindingResource {
        BindingResource::Buffer(BufferBinding {
            buffer: self.uniform_buffer().expect("uniform buffer should exist"),
            offset: 0,
            size: Some(NonZeroU64::new(self.item_size as u64).unwrap()),
        })
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
        let len = self.values.len();
        if len < self.capacity {
            self.values.push(value);
            len
        } else {
            panic!(
                "Cannot push value because capacity of {} has been reached",
                self.capacity
            );
        }
    }

    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        if capacity > self.capacity {
            self.capacity = capacity;
            let size = (self.item_size * capacity) as wgpu::BufferAddress;
            self.staging_buffer = Some(device.create_buffer(&BufferDescriptor {
                label: None,
                size,
                usage: BufferUsages::COPY_SRC | BufferUsages::MAP_WRITE,
                mapped_at_creation: false,
            }));
            self.uniform_buffer = Some(device.create_buffer(&BufferDescriptor {
                label: None,
                size,
                usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                mapped_at_creation: false,
            }));
        }
    }

    pub fn reserve_and_clear(&mut self, capacity: usize, device: &RenderDevice) {
        self.clear();
        self.reserve(capacity, device);
    }

    pub fn write_to_staging_buffer(&self, device: &RenderDevice) {
        if let Some(staging_buffer) = &self.staging_buffer {
            let slice = staging_buffer.slice(..);
            device.map_buffer(&slice, wgpu::MapMode::Write);
            {
                let mut data = slice.get_mapped_range_mut();
                let mut writer = std140::Writer::new(data.deref_mut());
                writer.write(self.values.as_slice()).unwrap();
            }
            staging_buffer.unmap()
        }
    }
    pub fn write_to_uniform_buffer(&self, command_encoder: &mut CommandEncoder) {
        if let (Some(staging_buffer), Some(uniform_buffer)) =
            (&self.staging_buffer, &self.uniform_buffer)
        {
            command_encoder.copy_buffer_to_buffer(
                staging_buffer,
                0,
                uniform_buffer,
                0,
                (self.values.len() * self.item_size) as u64,
            );
        }
    }

    pub fn clear(&mut self) {
        self.values.clear();
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
    pub fn staging_buffer(&self) -> Option<&Buffer> {
        self.uniform_vec.staging_buffer()
    }

    #[inline]
    pub fn uniform_buffer(&self) -> Option<&Buffer> {
        self.uniform_vec.uniform_buffer()
    }

    #[inline]
    pub fn binding(&self) -> BindingResource {
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
    pub fn reserve_and_clear(&mut self, capacity: usize, device: &RenderDevice) {
        self.uniform_vec.reserve_and_clear(capacity, device);
    }

    #[inline]
    pub fn write_to_staging_buffer(&self, device: &RenderDevice) {
        self.uniform_vec.write_to_staging_buffer(device);
    }

    #[inline]
    pub fn write_to_uniform_buffer(&self, command_encoder: &mut CommandEncoder) {
        self.uniform_vec.write_to_uniform_buffer(command_encoder);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.uniform_vec.clear();
    }
}
