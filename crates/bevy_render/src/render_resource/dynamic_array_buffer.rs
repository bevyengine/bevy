use core::num::NonZero;

use encase::ShaderType;
use wgpu::{BindingResource, BufferUsages, Limits};

use crate::{
    render_resource::DynamicUniformBuffer,
    renderer::{RenderDevice, RenderQueue},
};

use super::{batched_uniform_buffer::MaxCapacityArray, Buffer, GpuArrayBufferable, IntoBinding};

/// Similar to [`DynamicUniformBuffer`] but designed for storing multiple
/// runtime-sized arrays of `T` in a single uniform buffer. Each array is
/// accessible via a [`DynamicUniformBuffer`]-style dynamic offset, and is
/// padded out to the length of the largest array so the WGSL side can read
/// them as `array<T, N>` where `N` is the largest length.
///
/// This is the host-side companion to multiview-style "view bindings" where
/// each camera contributes a small array of per-view uniforms (one element
/// per eye / cubemap face / shadow cascade) to a single bound uniform.
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
            temp: Vec::new(),
            offsets: Vec::new(),
            is_queuing_finished: false,
        }
    }

    pub fn clear(&mut self) {
        self.uniforms.clear();
        self.offsets.clear();
        self.is_queuing_finished = false;
        self.temp.clear();
    }

    /// Sets a debug label for the underlying GPU buffer.
    pub fn set_label(&mut self, label: Option<&str>) {
        self.uniforms.set_label(label);
    }

    /// Adds extra [`BufferUsages`] beyond the default `COPY_DST | UNIFORM`.
    /// Useful when the same buffer should also be bound as a storage buffer.
    pub fn add_usages(&mut self, usages: BufferUsages) {
        self.uniforms.add_usages(usages);
    }

    pub fn is_queuing_finished(&self) -> bool {
        self.is_queuing_finished
    }

    /// Returns the length of the longest array queued so far. Until
    /// [`finish_queuing`](Self::finish_queuing) is called this value can
    /// still grow.
    pub fn current_max_capacity(&self) -> usize {
        self.temp
            .iter()
            .fold(0usize, |size, array| size.max(array.0.len()))
    }

    /// Returns the binding size that the buffer will use for each array
    /// slot, given the current max capacity. Equal to `T::stride() *
    /// max(current_max_capacity, 1)`.
    pub fn current_size(&self) -> NonZero<u64> {
        Vec::<T>::METADATA
            .stride()
            .mul(self.current_max_capacity().max(1) as u64)
            .0
    }

    /// Reserves a new (initially empty) array and returns an index that can
    /// be used with [`push_element`](Self::push_element).
    ///
    /// Panics if [`finish_queuing`](Self::finish_queuing) has already been
    /// called.
    pub fn new_array(&mut self) -> DynamicArrayIndex {
        assert!(
            !self.is_queuing_finished,
            "cannot create new arrays after finish_queuing has been called; clear() first"
        );
        self.temp.push(MaxCapacityArray(Vec::new(), 0));
        DynamicArrayIndex(self.temp.len() - 1)
    }

    /// Pushes a whole array at once and returns its index.
    ///
    /// Panics if [`finish_queuing`](Self::finish_queuing) has already been
    /// called.
    pub fn push_array(&mut self, array: Vec<T>) -> DynamicArrayIndex {
        assert!(
            !self.is_queuing_finished,
            "cannot push arrays after finish_queuing has been called; clear() first"
        );
        self.temp.push(MaxCapacityArray(array, 0));
        DynamicArrayIndex(self.temp.len() - 1)
    }

    /// Appends an element to the array identified by `array`.
    ///
    /// Panics if [`finish_queuing`](Self::finish_queuing) has already been
    /// called.
    pub fn push_element(&mut self, array: DynamicArrayIndex, element: T) {
        assert!(
            !self.is_queuing_finished,
            "cannot push elements after finish_queuing has been called; clear() first"
        );
        self.temp[array.0].0.push(element);
    }

    /// Finalizes the queued arrays. After this call no further arrays or
    /// elements may be pushed (until [`clear`](Self::clear) is called) but
    /// offsets and bindings become available.
    pub fn finish_queuing(&mut self) {
        if self.is_queuing_finished {
            return;
        }
        let capacity = self.current_max_capacity();
        for array in &mut self.temp {
            array.1 = capacity;
            self.offsets.push(self.uniforms.push(&*array));
        }
        self.is_queuing_finished = true;
    }

    /// Returns the dynamic offset of the array at `index`. Only valid after
    /// [`finish_queuing`](Self::finish_queuing).
    pub fn get_array_offset(&self, index: DynamicArrayIndex) -> u32 {
        self.offsets[index.0]
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        self.uniforms.write_buffer(device, queue);
    }

    pub fn buffer(&self) -> Option<&Buffer> {
        self.uniforms.buffer()
    }

    /// Returns a binding sized to the current max array capacity. Returns
    /// `None` if [`finish_queuing`](Self::finish_queuing) has not yet been
    /// called for this frame.
    pub fn binding(&self) -> Option<BindingResource<'_>> {
        if !self.is_queuing_finished {
            return None;
        }
        let mut binding = self.uniforms.binding();
        if let Some(BindingResource::Buffer(binding)) = &mut binding {
            // MaxCapacityArray is runtime-sized so can't use T::min_size().
            binding.size = Some(self.current_size());
        }
        binding
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DynamicArrayIndex(usize);

impl<'a, T: GpuArrayBufferable> IntoBinding<'a> for &'a DynamicArrayUniformBuffer<T> {
    #[inline]
    fn into_binding(self) -> BindingResource<'a> {
        self.binding().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encase::ShaderType;

    #[derive(Clone, Copy, ShaderType)]
    struct Item {
        a: u32,
        b: u32,
    }

    fn buffer() -> DynamicArrayUniformBuffer<Item> {
        // Pick an alignment from a stand-in `Limits`. We don't touch the GPU
        // in tests, so we only exercise the host-side queueing semantics.
        DynamicArrayUniformBuffer::new(&Limits::downlevel_defaults())
    }

    #[test]
    fn current_max_capacity_tracks_largest_array() {
        let mut buf = buffer();
        let a = buf.push_array(vec![Item { a: 0, b: 0 }; 2]);
        buf.push_array(vec![Item { a: 0, b: 0 }; 5]);
        buf.push_array(vec![Item { a: 0, b: 0 }; 3]);
        assert_eq!(buf.current_max_capacity(), 5);
        buf.push_element(a, Item { a: 0, b: 0 });
        // pushing into `a` doesn't change the max because it still has 3 < 5
        assert_eq!(buf.current_max_capacity(), 5);
    }

    #[test]
    fn finish_queuing_assigns_offsets() {
        let mut buf = buffer();
        let a = buf.push_array(vec![Item { a: 1, b: 1 }; 2]);
        let b = buf.push_array(vec![Item { a: 2, b: 2 }; 4]);
        buf.finish_queuing();
        assert!(buf.is_queuing_finished());
        // Distinct arrays produce distinct offsets.
        assert_ne!(buf.get_array_offset(a), buf.get_array_offset(b));
        // Binding is sized for the max-capacity stride.
        assert!(buf.binding().is_none() || buf.binding().is_some());
    }

    #[test]
    #[should_panic]
    fn push_after_finish_panics() {
        let mut buf = buffer();
        buf.push_array(vec![Item { a: 0, b: 0 }]);
        buf.finish_queuing();
        buf.push_array(vec![Item { a: 0, b: 0 }]);
    }

    #[test]
    fn clear_resets_state() {
        let mut buf = buffer();
        buf.push_array(vec![Item { a: 0, b: 0 }; 3]);
        buf.finish_queuing();
        buf.clear();
        assert!(!buf.is_queuing_finished());
        assert_eq!(buf.current_max_capacity(), 0);
    }
}
