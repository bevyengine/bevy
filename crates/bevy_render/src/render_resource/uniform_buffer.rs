use std::{marker::PhantomData, num::NonZeroU64};

use crate::{
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};
use encase::{
    internal::{AlignmentValue, BufferMut, WriteInto},
    DynamicUniformBuffer as DynamicUniformBufferWrapper, ShaderType,
    UniformBuffer as UniformBufferWrapper,
};
use wgpu::{
    util::BufferInitDescriptor, BindingResource, BufferBinding, BufferDescriptor, BufferUsages,
};

use super::IntoBinding;

/// Stores data to be transferred to the GPU and made accessible to shaders as a uniform buffer.
///
/// Uniform buffers are available to shaders on a read-only basis. Uniform buffers are commonly used to make available to shaders
/// parameters that are constant during shader execution, and are best used for data that is relatively small in size as they are
/// only guaranteed to support up to 16kB per binding.
///
/// The contained data is stored in system RAM. [`write_buffer`](UniformBuffer::write_buffer) queues
/// copying of the data from system RAM to VRAM. Data in uniform buffers must follow [std140 alignment/padding requirements],
/// which is automatically enforced by this structure. Per the WGPU spec, uniform buffers cannot store runtime-sized array
/// (vectors), or structures with fields that are vectors.
///
/// Other options for storing GPU-accessible data are:
/// * [`StorageBuffer`](crate::render_resource::StorageBuffer)
/// * [`DynamicStorageBuffer`](crate::render_resource::DynamicStorageBuffer)
/// * [`DynamicUniformBuffer`]
/// * [`GpuArrayBuffer`](crate::render_resource::GpuArrayBuffer)
/// * [`RawBufferVec`](crate::render_resource::RawBufferVec)
/// * [`BufferVec`](crate::render_resource::BufferVec)
/// * [`Texture`](crate::render_resource::Texture)
///
/// [std140 alignment/padding requirements]: https://www.w3.org/TR/WGSL/#address-spaces-uniform
pub struct UniformBuffer<T: ShaderType> {
    value: T,
    scratch: UniformBufferWrapper<Vec<u8>>,
    buffer: Option<Buffer>,
    label: Option<String>,
    changed: bool,
    buffer_usage: BufferUsages,
}

impl<T: ShaderType> From<T> for UniformBuffer<T> {
    fn from(value: T) -> Self {
        Self {
            value,
            scratch: UniformBufferWrapper::new(Vec::new()),
            buffer: None,
            label: None,
            changed: false,
            buffer_usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
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
            changed: false,
            buffer_usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
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
            self.changed = true;
        }

        self.label = label;
    }

    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Add more [`BufferUsages`] to the buffer.
    ///
    /// This method only allows addition of flags to the default usage flags.
    ///
    /// The default values for buffer usage are `BufferUsages::COPY_DST` and `BufferUsages::UNIFORM`.
    pub fn add_usages(&mut self, usage: BufferUsages) {
        self.buffer_usage |= usage;
        self.changed = true;
    }

    /// Queues writing of data from system RAM to VRAM using the [`RenderDevice`]
    /// and the provided [`RenderQueue`], if a GPU-side backing buffer already exists.
    ///
    /// If a GPU-side buffer does not already exist for this data, such a buffer is initialized with currently
    /// available data.
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.scratch.write(&self.value).unwrap();

        if self.changed || self.buffer.is_none() {
            self.buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
                label: self.label.as_deref(),
                usage: self.buffer_usage,
                contents: self.scratch.as_ref(),
            }));
            self.changed = false;
        } else if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, self.scratch.as_ref());
        }
    }
}

impl<'a, T: ShaderType + WriteInto> IntoBinding<'a> for &'a UniformBuffer<T> {
    #[inline]
    fn into_binding(self) -> BindingResource<'a> {
        self.buffer()
            .expect("Failed to get buffer")
            .as_entire_buffer_binding()
            .into_binding()
    }
}

/// Stores data to be transferred to the GPU and made accessible to shaders as a dynamic uniform buffer.
///
/// Dynamic uniform buffers are available to shaders on a read-only basis. Dynamic uniform buffers are commonly used to make
/// available to shaders runtime-sized arrays of parameters that are otherwise constant during shader execution, and are best
/// suited to data that is relatively small in size as they are only guaranteed to support up to 16kB per binding.
///
/// The contained data is stored in system RAM. [`write_buffer`](DynamicUniformBuffer::write_buffer) queues
/// copying of the data from system RAM to VRAM. Data in uniform buffers must follow [std140 alignment/padding requirements],
/// which is automatically enforced by this structure. Per the WGPU spec, uniform buffers cannot store runtime-sized array
/// (vectors), or structures with fields that are vectors.
///
/// Other options for storing GPU-accessible data are:
/// * [`StorageBuffer`](crate::render_resource::StorageBuffer)
/// * [`DynamicStorageBuffer`](crate::render_resource::DynamicStorageBuffer)
/// * [`UniformBuffer`]
/// * [`DynamicUniformBuffer`]
/// * [`GpuArrayBuffer`](crate::render_resource::GpuArrayBuffer)
/// * [`RawBufferVec`](crate::render_resource::RawBufferVec)
/// * [`BufferVec`](crate::render_resource::BufferVec)
/// * [`Texture`](crate::render_resource::Texture)
///
/// [std140 alignment/padding requirements]: https://www.w3.org/TR/WGSL/#address-spaces-uniform
pub struct DynamicUniformBuffer<T: ShaderType> {
    scratch: DynamicUniformBufferWrapper<Vec<u8>>,
    buffer: Option<Buffer>,
    label: Option<String>,
    changed: bool,
    buffer_usage: BufferUsages,
    _marker: PhantomData<fn() -> T>,
}

impl<T: ShaderType> Default for DynamicUniformBuffer<T> {
    fn default() -> Self {
        Self {
            scratch: DynamicUniformBufferWrapper::new(Vec::new()),
            buffer: None,
            label: None,
            changed: false,
            buffer_usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            _marker: PhantomData,
        }
    }
}

impl<T: ShaderType + WriteInto> DynamicUniformBuffer<T> {
    pub fn new_with_alignment(alignment: u64) -> Self {
        Self {
            scratch: DynamicUniformBufferWrapper::new_with_alignment(Vec::new(), alignment),
            buffer: None,
            label: None,
            changed: false,
            buffer_usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            _marker: PhantomData,
        }
    }

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
    pub fn is_empty(&self) -> bool {
        self.scratch.as_ref().is_empty()
    }

    /// Push data into the `DynamicUniformBuffer`'s internal vector (residing on system RAM).
    #[inline]
    pub fn push(&mut self, value: &T) -> u32 {
        self.scratch.write(value).unwrap() as u32
    }

    pub fn set_label(&mut self, label: Option<&str>) {
        let label = label.map(str::to_string);

        if label != self.label {
            self.changed = true;
        }

        self.label = label;
    }

    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Add more [`BufferUsages`] to the buffer.
    ///
    /// This method only allows addition of flags to the default usage flags.
    ///
    /// The default values for buffer usage are `BufferUsages::COPY_DST` and `BufferUsages::UNIFORM`.
    pub fn add_usages(&mut self, usage: BufferUsages) {
        self.buffer_usage |= usage;
        self.changed = true;
    }

    /// Creates a writer that can be used to directly write elements into the target buffer.
    ///
    /// This method uses less memory and performs fewer memory copies using over [`push`] and [`write_buffer`].
    ///
    /// `max_count` *must* be greater than or equal to the number of elements that are to be written to the buffer, or
    /// the writer will panic while writing.  Dropping the writer will schedule the buffer write into the provided
    /// [`RenderQueue`].
    ///
    /// If there is no GPU-side buffer allocated to hold the data currently stored, or if a GPU-side buffer previously
    /// allocated does not have enough capacity to hold `max_count` elements, a new GPU-side buffer is created.
    ///
    /// Returns `None` if there is no allocated GPU-side buffer, and `max_count` is 0.
    ///
    /// [`push`]: Self::push
    /// [`write_buffer`]: Self::write_buffer
    #[inline]
    pub fn get_writer<'a>(
        &'a mut self,
        max_count: usize,
        device: &RenderDevice,
        queue: &'a RenderQueue,
    ) -> Option<DynamicUniformBufferWriter<'a, T>> {
        let alignment = if cfg!(feature = "ios_simulator") {
            // On iOS simulator on silicon macs, metal validation check that the host OS alignment
            // is respected, but the device reports the correct value for iOS, which is smaller.
            // Use the larger value.
            // See https://github.com/bevyengine/bevy/pull/10178 - remove if it's not needed anymore.
            AlignmentValue::new(256)
        } else {
            AlignmentValue::new(device.limits().min_uniform_buffer_offset_alignment as u64)
        };

        let mut capacity = self.buffer.as_deref().map(wgpu::Buffer::size).unwrap_or(0);
        let size = alignment
            .round_up(T::min_size().get())
            .checked_mul(max_count as u64)
            .unwrap();

        if capacity < size || (self.changed && size > 0) {
            let buffer = device.create_buffer(&BufferDescriptor {
                label: self.label.as_deref(),
                usage: self.buffer_usage,
                size,
                mapped_at_creation: false,
            });
            capacity = buffer.size();
            self.buffer = Some(buffer);
            self.changed = false;
        }

        if let Some(buffer) = self.buffer.as_deref() {
            let buffer_view = queue
                .write_buffer_with(buffer, 0, NonZeroU64::new(buffer.size())?)
                .unwrap();
            Some(DynamicUniformBufferWriter {
                buffer: encase::DynamicUniformBuffer::new_with_alignment(
                    QueueWriteBufferViewWrapper {
                        capacity: capacity as usize,
                        buffer_view,
                    },
                    alignment.get(),
                ),
                _marker: PhantomData,
            })
        } else {
            None
        }
    }

    /// Queues writing of data from system RAM to VRAM using the [`RenderDevice`]
    /// and the provided [`RenderQueue`].
    ///
    /// If there is no GPU-side buffer allocated to hold the data currently stored, or if a GPU-side buffer previously
    /// allocated does not have enough capacity, a new GPU-side buffer is created.
    #[inline]
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        let capacity = self.buffer.as_deref().map(wgpu::Buffer::size).unwrap_or(0);
        let size = self.scratch.as_ref().len() as u64;

        if capacity < size || (self.changed && size > 0) {
            self.buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
                label: self.label.as_deref(),
                usage: self.buffer_usage,
                contents: self.scratch.as_ref(),
            }));
            self.changed = false;
        } else if let Some(buffer) = &self.buffer {
            queue.write_buffer(buffer, 0, self.scratch.as_ref());
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.scratch.as_mut().clear();
        self.scratch.set_offset(0);
    }
}

/// A writer that can be used to directly write elements into the target buffer.
///
/// For more information, see [`DynamicUniformBuffer::get_writer`].
pub struct DynamicUniformBufferWriter<'a, T> {
    buffer: encase::DynamicUniformBuffer<QueueWriteBufferViewWrapper<'a>>,
    _marker: PhantomData<fn() -> T>,
}

impl<'a, T: ShaderType + WriteInto> DynamicUniformBufferWriter<'a, T> {
    pub fn write(&mut self, value: &T) -> u32 {
        self.buffer.write(value).unwrap() as u32
    }
}

/// A wrapper to work around the orphan rule so that [`wgpu::QueueWriteBufferView`] can  implement
/// [`BufferMut`].
struct QueueWriteBufferViewWrapper<'a> {
    buffer_view: wgpu::QueueWriteBufferView<'a>,
    // Must be kept separately and cannot be retrieved from buffer_view, as the read-only access will
    // invoke a panic.
    capacity: usize,
}

impl<'a> BufferMut for QueueWriteBufferViewWrapper<'a> {
    #[inline]
    fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    fn write<const N: usize>(&mut self, offset: usize, val: &[u8; N]) {
        self.buffer_view.write(offset, val);
    }

    #[inline]
    fn write_slice(&mut self, offset: usize, val: &[u8]) {
        self.buffer_view.write_slice(offset, val);
    }
}

impl<'a, T: ShaderType + WriteInto> IntoBinding<'a> for &'a DynamicUniformBuffer<T> {
    #[inline]
    fn into_binding(self) -> BindingResource<'a> {
        self.binding().unwrap()
    }
}
