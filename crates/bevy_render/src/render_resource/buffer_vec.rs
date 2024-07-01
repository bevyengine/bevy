use std::{iter, marker::PhantomData};

use crate::{
    render_resource::Buffer,
    renderer::{RenderDevice, RenderQueue},
};
use bytemuck::{must_cast_slice, NoUninit};
use encase::{
    internal::{WriteInto, Writer},
    ShaderType,
};
use wgpu::{BindingResource, BufferAddress, BufferUsages};

use super::GpuArrayBufferable;

/// A structure for storing raw bytes that have already been properly formatted
/// for use by the GPU.
///
/// "Properly formatted" means that item data already meets the alignment and padding
/// requirements for how it will be used on the GPU. The item type must implement [`NoUninit`]
/// for its data representation to be directly copyable.
///
/// Index, vertex, and instance-rate vertex buffers have no alignment nor padding requirements and
/// so this helper type is a good choice for them.
///
/// The contained data is stored in system RAM. Calling [`reserve`](RawBufferVec::reserve)
/// allocates VRAM from the [`RenderDevice`].
/// [`write_buffer`](RawBufferVec::write_buffer) queues copying of the data
/// from system RAM to VRAM.
///
/// Other options for storing GPU-accessible data are:
/// * [`StorageBuffer`](crate::render_resource::StorageBuffer)
/// * [`DynamicStorageBuffer`](crate::render_resource::DynamicStorageBuffer)
/// * [`UniformBuffer`](crate::render_resource::UniformBuffer)
/// * [`DynamicUniformBuffer`](crate::render_resource::DynamicUniformBuffer)
/// * [`GpuArrayBuffer`](crate::render_resource::GpuArrayBuffer)
/// * [`BufferVec`]
/// * [`Texture`](crate::render_resource::Texture)
pub struct RawBufferVec<T: NoUninit> {
    values: Vec<T>,
    buffer: Option<Buffer>,
    capacity: usize,
    item_size: usize,
    buffer_usage: BufferUsages,
    label: Option<String>,
    changed: bool,
}

impl<T: NoUninit> RawBufferVec<T> {
    /// Creates a new [`RawBufferVec`] with the given [`BufferUsages`].
    pub const fn new(buffer_usage: BufferUsages) -> Self {
        Self {
            values: Vec::new(),
            buffer: None,
            capacity: 0,
            item_size: std::mem::size_of::<T>(),
            buffer_usage,
            label: None,
            changed: false,
        }
    }

    /// Returns a handle to the buffer, if the data has been uploaded.
    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    /// Returns the binding for the buffer if the data has been uploaded.
    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(
            self.buffer()?.as_entire_buffer_binding(),
        ))
    }

    /// Returns the amount of space that the GPU will use before reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of items that have been pushed to this buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Returns true if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Adds a new value and returns its index.
    pub fn push(&mut self, value: T) -> usize {
        let index = self.values.len();
        self.values.push(value);
        index
    }

    pub fn append(&mut self, other: &mut RawBufferVec<T>) {
        self.values.append(&mut other.values);
    }

    /// Changes the debugging label of the buffer.
    ///
    /// The next time the buffer is updated (via [`reserve`]), Bevy will inform
    /// the driver of the new label.
    pub fn set_label(&mut self, label: Option<&str>) {
        let label = label.map(str::to_string);

        if label != self.label {
            self.changed = true;
        }

        self.label = label;
    }

    /// Returns the label
    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Creates a [`Buffer`] on the [`RenderDevice`] with size
    /// at least `std::mem::size_of::<T>() * capacity`, unless a such a buffer already exists.
    ///
    /// If a [`Buffer`] exists, but is too small, references to it will be discarded,
    /// and a new [`Buffer`] will be created. Any previously created [`Buffer`]s
    /// that are no longer referenced will be deleted by the [`RenderDevice`]
    /// once it is done using them (typically 1-2 frames).
    ///
    /// In addition to any [`BufferUsages`] provided when
    /// the `RawBufferVec` was created, the buffer on the [`RenderDevice`]
    /// is marked as [`BufferUsages::COPY_DST`](BufferUsages).
    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        let size = self.item_size * capacity;
        if capacity > self.capacity || (self.changed && size > 0) {
            self.capacity = capacity;
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: self.label.as_deref(),
                size: size as BufferAddress,
                usage: BufferUsages::COPY_DST | self.buffer_usage,
                mapped_at_creation: false,
            }));
            self.changed = false;
        }
    }

    /// Queues writing of data from system RAM to VRAM using the [`RenderDevice`]
    /// and the provided [`RenderQueue`].
    ///
    /// Before queuing the write, a [`reserve`](RawBufferVec::reserve) operation
    /// is executed.
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.values.is_empty() {
            return;
        }
        self.reserve(self.values.len(), device);
        if let Some(buffer) = &self.buffer {
            let range = 0..self.item_size * self.values.len();
            let bytes: &[u8] = must_cast_slice(&self.values);
            queue.write_buffer(buffer, 0, &bytes[range]);
        }
    }

    /// Reduces the length of the buffer.
    pub fn truncate(&mut self, len: usize) {
        self.values.truncate(len);
    }

    /// Removes all elements from the buffer.
    pub fn clear(&mut self) {
        self.values.clear();
    }

    pub fn values(&self) -> &Vec<T> {
        &self.values
    }

    pub fn values_mut(&mut self) -> &mut Vec<T> {
        &mut self.values
    }
}

impl<T: NoUninit> Extend<T> for RawBufferVec<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.values.extend(iter);
    }
}

/// Like [`RawBufferVec`], but doesn't require that the data type `T` be
/// [`NoUninit`].
///
/// This is a high-performance data structure that you should use whenever
/// possible if your data is more complex than is suitable for [`RawBufferVec`].
/// The [`ShaderType`] trait from the `encase` library is used to ensure that
/// the data is correctly aligned for use by the GPU.
///
/// For performance reasons, unlike [`RawBufferVec`], this type doesn't allow
/// CPU access to the data after it's been added via [`BufferVec::push`]. If you
/// need CPU access to the data, consider another type, such as
/// [`StorageBuffer`].
pub struct BufferVec<T>
where
    T: ShaderType + WriteInto,
{
    data: Vec<u8>,
    buffer: Option<Buffer>,
    capacity: usize,
    buffer_usage: BufferUsages,
    label: Option<String>,
    label_changed: bool,
    phantom: PhantomData<T>,
}

impl<T> BufferVec<T>
where
    T: ShaderType + WriteInto,
{
    /// Creates a new [`BufferVec`] with the given [`BufferUsages`].
    pub const fn new(buffer_usage: BufferUsages) -> Self {
        Self {
            data: vec![],
            buffer: None,
            capacity: 0,
            buffer_usage,
            label: None,
            label_changed: false,
            phantom: PhantomData,
        }
    }

    /// Returns a handle to the buffer, if the data has been uploaded.
    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    /// Returns the binding for the buffer if the data has been uploaded.
    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(
            self.buffer()?.as_entire_buffer_binding(),
        ))
    }

    /// Returns the amount of space that the GPU will use before reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the number of items that have been pushed to this buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len() / u64::from(T::min_size()) as usize
    }

    /// Returns true if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Adds a new value and returns its index.
    pub fn push(&mut self, value: T) -> usize {
        let element_size = u64::from(T::min_size()) as usize;
        let offset = self.data.len();

        // TODO: Consider using unsafe code to push uninitialized, to prevent
        // the zeroing. It shows up in profiles.
        self.data.extend(iter::repeat(0).take(element_size));

        // Take a slice of the new data for `write_into` to use. This is
        // important: it hoists the bounds check up here so that the compiler
        // can eliminate all the bounds checks that `write_into` will emit.
        let mut dest = &mut self.data[offset..(offset + element_size)];
        value.write_into(&mut Writer::new(&value, &mut dest, 0).unwrap());

        offset / u64::from(T::min_size()) as usize
    }

    /// Changes the debugging label of the buffer.
    ///
    /// The next time the buffer is updated (via [`reserve`]), Bevy will inform
    /// the driver of the new label.
    pub fn set_label(&mut self, label: Option<&str>) {
        let label = label.map(str::to_string);

        if label != self.label {
            self.label_changed = true;
        }

        self.label = label;
    }

    /// Returns the label
    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Creates a [`Buffer`] on the [`RenderDevice`] with size
    /// at least `std::mem::size_of::<T>() * capacity`, unless such a buffer already exists.
    ///
    /// If a [`Buffer`] exists, but is too small, references to it will be discarded,
    /// and a new [`Buffer`] will be created. Any previously created [`Buffer`]s
    /// that are no longer referenced will be deleted by the [`RenderDevice`]
    /// once it is done using them (typically 1-2 frames).
    ///
    /// In addition to any [`BufferUsages`] provided when
    /// the `BufferVec` was created, the buffer on the [`RenderDevice`]
    /// is marked as [`BufferUsages::COPY_DST`](BufferUsages).
    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        if capacity <= self.capacity && !self.label_changed {
            return;
        }

        self.capacity = capacity;
        let size = u64::from(T::min_size()) as usize * capacity;
        self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: size as BufferAddress,
            usage: BufferUsages::COPY_DST | self.buffer_usage,
            mapped_at_creation: false,
        }));
        self.label_changed = false;
    }

    /// Queues writing of data from system RAM to VRAM using the [`RenderDevice`]
    /// and the provided [`RenderQueue`].
    ///
    /// Before queuing the write, a [`reserve`](BufferVec::reserve) operation is
    /// executed.
    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if self.data.is_empty() {
            return;
        }

        self.reserve(self.data.len() / u64::from(T::min_size()) as usize, device);

        let Some(buffer) = &self.buffer else { return };
        queue.write_buffer(buffer, 0, &self.data);
    }

    /// Reduces the length of the buffer.
    pub fn truncate(&mut self, len: usize) {
        self.data.truncate(u64::from(T::min_size()) as usize * len);
    }

    /// Removes all elements from the buffer.
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

/// Like a [`BufferVec`], but only reserves space on the GPU for elements
/// instead of initializing them CPU-side.
///
/// This type is useful when you're accumulating "output slots" for a GPU
/// compute shader to write into.
///
/// The type `T` need not be [`NoUninit`], unlike [`RawBufferVec`]; it only has to
/// be [`GpuArrayBufferable`].
pub struct UninitBufferVec<T>
where
    T: GpuArrayBufferable,
{
    buffer: Option<Buffer>,
    len: usize,
    capacity: usize,
    item_size: usize,
    buffer_usage: BufferUsages,
    label: Option<String>,
    label_changed: bool,
    phantom: PhantomData<T>,
}

impl<T> UninitBufferVec<T>
where
    T: GpuArrayBufferable,
{
    /// Creates a new [`UninitBufferVec`] with the given [`BufferUsages`].
    pub const fn new(buffer_usage: BufferUsages) -> Self {
        Self {
            len: 0,
            buffer: None,
            capacity: 0,
            item_size: std::mem::size_of::<T>(),
            buffer_usage,
            label: None,
            label_changed: false,
            phantom: PhantomData,
        }
    }

    /// Returns the buffer, if allocated.
    #[inline]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref()
    }

    /// Returns the binding for the buffer if the data has been uploaded.
    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        Some(BindingResource::Buffer(
            self.buffer()?.as_entire_buffer_binding(),
        ))
    }

    /// Reserves space for one more element in the buffer and returns its index.
    pub fn add(&mut self) -> usize {
        let index = self.len;
        self.len += 1;
        index
    }

    /// Returns true if no elements have been added to this [`UninitBufferVec`].
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Removes all elements from the buffer.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns the length of the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Materializes the buffer on the GPU with space for `capacity` elements.
    ///
    /// If the buffer is already big enough, this function doesn't reallocate
    /// the buffer.
    pub fn reserve(&mut self, capacity: usize, device: &RenderDevice) {
        if capacity <= self.capacity && !self.label_changed {
            return;
        }

        self.capacity = capacity;
        let size = self.item_size * capacity;
        self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: size as wgpu::BufferAddress,
            usage: BufferUsages::COPY_DST | self.buffer_usage,
            mapped_at_creation: false,
        }));

        self.label_changed = false;
    }

    /// Materializes the buffer on the GPU, with an appropriate size for the
    /// elements that have been pushed so far.
    pub fn write_buffer(&mut self, device: &RenderDevice) {
        if !self.is_empty() {
            self.reserve(self.len, device);
        }
    }
}
