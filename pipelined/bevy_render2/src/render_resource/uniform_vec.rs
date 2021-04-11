use crate::{
    render_resource::{BufferId, BufferInfo, BufferMapMode, BufferUsage, RenderResourceBinding},
    renderer::{RenderContext, RenderResources},
};
use crevice::std140::{self, AsStd140, DynamicUniform, Std140};

pub struct UniformVec<T: AsStd140> {
    values: Vec<T>,
    staging_buffer: Option<BufferId>,
    uniform_buffer: Option<BufferId>,
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
    pub fn staging_buffer(&self) -> Option<BufferId> {
        self.staging_buffer
    }

    #[inline]
    pub fn uniform_buffer(&self) -> Option<BufferId> {
        self.uniform_buffer
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn push(&mut self, value: T) -> RenderResourceBinding {
        if self.values.len() < self.capacity {
            let binding = RenderResourceBinding::Buffer {
                buffer: self.uniform_buffer.unwrap(),
                dynamic_index: Some((self.values.len() * self.item_size) as u32),
                range: 0..self.item_size as u64,
            };
            self.values.push(value);
            binding
        } else {
            panic!(
                "Cannot push value because capacity of {} has been reached",
                self.capacity
            );
        }
    }

    pub fn reserve(&mut self, capacity: usize, render_resources: &RenderResources) {
        if capacity > self.capacity {
            self.capacity = capacity;
            if let Some(staging_buffer) = self.staging_buffer.take() {
                render_resources.remove_buffer(staging_buffer);
            }

            if let Some(uniform_buffer) = self.uniform_buffer.take() {
                render_resources.remove_buffer(uniform_buffer);
            }

            let size = self.item_size * capacity;
            self.staging_buffer = Some(render_resources.create_buffer(BufferInfo {
                size,
                buffer_usage: BufferUsage::COPY_SRC | BufferUsage::MAP_WRITE,
                mapped_at_creation: false,
            }));
            self.uniform_buffer = Some(render_resources.create_buffer(BufferInfo {
                size,
                buffer_usage: BufferUsage::COPY_DST | BufferUsage::UNIFORM,
                mapped_at_creation: false,
            }));
        }
    }

    pub fn reserve_and_clear(&mut self, capacity: usize, render_resources: &RenderResources) {
        self.clear();
        self.reserve(capacity, render_resources);
    }

    pub fn write_to_staging_buffer(&self, render_resources: &RenderResources) {
        if let Some(staging_buffer) = self.staging_buffer {
            let size = self.values.len() * self.item_size;
            render_resources.map_buffer(staging_buffer, BufferMapMode::Write);
            render_resources.write_mapped_buffer(
                staging_buffer,
                0..size as u64,
                &mut |data, _renderer| {
                    let mut writer = std140::Writer::new(data);
                    writer.write(self.values.as_slice()).unwrap();
                },
            );
            render_resources.unmap_buffer(staging_buffer);
        }
    }
    pub fn write_to_uniform_buffer(&self, render_context: &mut dyn RenderContext) {
        if let (Some(staging_buffer), Some(uniform_buffer)) =
            (self.staging_buffer, self.uniform_buffer)
        {
            render_context.copy_buffer_to_buffer(
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
    pub fn staging_buffer(&self) -> Option<BufferId> {
        self.uniform_vec.staging_buffer()
    }

    #[inline]
    pub fn uniform_buffer(&self) -> Option<BufferId> {
        self.uniform_vec.uniform_buffer()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.uniform_vec.capacity()
    }

    #[inline]
    pub fn push(&mut self, value: T) -> RenderResourceBinding {
        self.uniform_vec.push(DynamicUniform(value))
    }

    #[inline]
    pub fn reserve(&mut self, capacity: usize, render_resources: &RenderResources) {
        self.uniform_vec.reserve(capacity, render_resources);
    }

    #[inline]
    pub fn reserve_and_clear(&mut self, capacity: usize, render_resources: &RenderResources) {
        self.uniform_vec
            .reserve_and_clear(capacity, render_resources);
    }

    #[inline]
    pub fn write_to_staging_buffer(&self, render_resources: &RenderResources) {
        self.uniform_vec.write_to_staging_buffer(render_resources);
    }

    #[inline]
    pub fn write_to_uniform_buffer(&self, render_context: &mut dyn RenderContext) {
        self.uniform_vec.write_to_uniform_buffer(render_context);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.uniform_vec.clear();
    }
}
