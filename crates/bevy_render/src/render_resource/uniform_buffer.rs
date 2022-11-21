use crate::{
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};
use encase::{
    internal::WriteInto, DynamicUniformBuffer as DynamicUniformBufferWrapper, ShaderType,
    UniformBuffer as UniformBufferWrapper,
};
use wgpu::{util::BufferInitDescriptor, BindingResource, BufferBinding, BufferUsages};

/// Stores data to be transferred to the GPU and made accessible to shaders as a uniform buffer.
///
/// Uniform buffers are available to shaders on a read-only basis. Uniform buffers are commonly used to make available to shaders
/// parameters that are constant during shader execution, and are best used for data that is relatively small in size as they are
/// only guaranteed to support up to 16kB per binding.
///
/// The contained data is stored in system RAM. [`write_buffer`](crate::render_resource::UniformBuffer::write_buffer) queues
/// copying of the data from system RAM to VRAM. Data in uniform buffers must follow [std140 alignment/padding requirements],
/// which is automatically enforced by this structure. Per the WGPU spec, uniform buffers cannot store runtime-sized array
/// (vectors), or structures with fields that are vectors.
///
/// Other options for storing GPU-accessible data are:
/// * [`DynamicUniformBuffer`](crate::render_resource::DynamicUniformBuffer)
/// * [`StorageBuffer`](crate::render_resource::StorageBuffer)
/// * [`DynamicStorageBuffer`](crate::render_resource::DynamicStorageBuffer)
/// * [`BufferVec`](crate::render_resource::BufferVec)
/// * [`Texture`](crate::render_resource::Texture)
///
/// [std140 alignment/padding requirements]: https://www.w3.org/TR/WGSL/#address-spaces-uniform
pub struct UniformBuffer<T: ShaderType> {
    value: T,
    scratch: UniformBufferWrapper<Vec<u8>>,
    buffer: Option<Buffer>,
    label: Option<String>,
    label_changed: bool,
}

impl<T: ShaderType> From<T> for UniformBuffer<T> {
    fn from(value: T) -> Self {
        Self {
            value,
            scratch: UniformBufferWrapper::new(Vec::new()),
            buffer: None,
            label: None,
            label_changed: false,
        }
    }
}

impl<T: ShaderType + Default> Default for UniformBuffer<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            scratch: UniformBufferWrapper::new(Vec::new()),
            buffer: None,
            label: None,
            label_changed: false,
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

    /// Set the data the buffer stores.
    pub fn set(&mut self, value: T) {
        self.value = value;
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    pub fn set_label(&mut self, label: Option<&str>) {
        let label = label.map(str::to_string);

        if label != self.label {
            self.label_changed = true;
        }

        self.label = label;
    }

    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Queues writing of data from system RAM to VRAM using the [`RenderDevice`](crate::renderer::RenderDevice)
    /// and the provided [`RenderQueue`](crate::renderer::RenderQueue), if a GPU-side backing buffer already exists.
    ///
    /// If a GPU-side buffer does not already exist for this data, such a buffer is initialized with currently
    /// available data.
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.scratch.write(&self.value).unwrap();

        if self.label_changed || self.buffer.is_none() {
            self.buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
                label: self.label.as_deref(),
                usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                contents: self.scratch.as_ref(),
            }));
            self.label_changed = false;
        } else if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, self.scratch.as_ref());
        }
    }
}

/// Stores data to be transferred to the GPU and made accessible to shaders as a dynamic uniform buffer.
///
/// Dynamic uniform buffers are available to shaders on a read-only basis. Dynamic uniform buffers are commonly used to make
/// available to shaders runtime-sized arrays of parameters that are otherwise constant during shader execution, and are best
/// suited to data that is relatively small in size as they are only guaranteed to support up to 16kB per binding.
///
/// The contained data is stored in system RAM. [`write_buffer`](crate::render_resource::DynamicUniformBuffer::write_buffer) queues
/// copying of the data from system RAM to VRAM. Data in uniform buffers must follow [std140 alignment/padding requirements],
/// which is automatically enforced by this structure. Per the WGPU spec, uniform buffers cannot store runtime-sized array
/// (vectors), or structures with fields that are vectors.
///
/// Other options for storing GPU-accessible data are:
/// * [`StorageBuffer`](crate::render_resource::StorageBuffer)
/// * [`DynamicStorageBuffer`](crate::render_resource::DynamicStorageBuffer)
/// * [`UniformBuffer`](crate::render_resource::UniformBuffer)
/// * [`DynamicUniformBuffer`](crate::render_resource::DynamicUniformBuffer)
/// * [`Texture`](crate::render_resource::Texture)
///
/// [std140 alignment/padding requirements]: https://www.w3.org/TR/WGSL/#address-spaces-uniform
pub struct DynamicUniformBuffer<T: ShaderType> {
    values: Vec<T>,
    scratch: DynamicUniformBufferWrapper<Vec<u8>>,
    buffer: Option<Buffer>,
    capacity: usize,
    label: Option<String>,
    label_changed: bool,
}

impl<T: ShaderType> Default for DynamicUniformBuffer<T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            scratch: DynamicUniformBufferWrapper::new(Vec::new()),
            buffer: None,
            capacity: 0,
            label: None,
            label_changed: false,
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

    /// Push data into the `DynamicUniformBuffer`'s internal vector (residing on system RAM).
    #[inline]
    pub fn push(&mut self, value: T) -> u32 {
        let offset = self.scratch.write(&value).unwrap() as u32;
        self.values.push(value);
        offset
    }

    pub fn set_label(&mut self, label: Option<&str>) {
        let label = label.map(str::to_string);

        if label != self.label {
            self.label_changed = true;
        }

        self.label = label;
    }

    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Queues writing of data from system RAM to VRAM using the [`RenderDevice`](crate::renderer::RenderDevice)
    /// and the provided [`RenderQueue`](crate::renderer::RenderQueue).
    ///
    /// If there is no GPU-side buffer allocated to hold the data currently stored, or if a GPU-side buffer previously
    /// allocated does not have enough capacity, a new GPU-side buffer is created.
    #[inline]
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        let size = self.scratch.as_ref().len();

        if self.capacity < size || self.label_changed {
            self.buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
                label: self.label.as_deref(),
                usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                contents: self.scratch.as_ref(),
            }));
            self.capacity = size;
            self.label_changed = false;
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
