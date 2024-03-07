use super::{GpuArrayBufferIndex, GpuArrayBufferable};
use crate::{
    render_resource::DynamicUniformBuffer,
    renderer::{RenderDevice, RenderQueue},
};
use encase::{
    private::{ArrayMetadata, BufferMut, Metadata, RuntimeSizedArray, WriteInto, Writer},
    ShaderType,
};
use nonmax::NonMaxU32;
use std::{marker::PhantomData, num::NonZeroU64};
use wgpu::{BindingResource, Limits};

// 1MB else we will make really large arrays on macOS which reports very large
// `max_uniform_buffer_binding_size`. On macOS this ends up being the minimum
// size of the uniform buffer as well as the size of each chunk of data at a
// dynamic offset.
#[cfg(any(
    not(feature = "webgl"),
    not(target_arch = "wasm32"),
    feature = "webgpu"
))]
const MAX_REASONABLE_UNIFORM_BUFFER_BINDING_SIZE: u32 = 1 << 20;

// WebGL2 quirk: using uniform buffers larger than 4KB will cause extremely
// long shader compilation times, so the limit needs to be lower on WebGL2.
// This is due to older shader compilers/GPUs that don't support dynamically
// indexing uniform buffers, and instead emulate it with large switch statements
// over buffer indices that take a long time to compile.
#[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
const MAX_REASONABLE_UNIFORM_BUFFER_BINDING_SIZE: u32 = 1 << 12;

/// Similar to [`DynamicUniformBuffer`], except every N elements (depending on size)
/// are grouped into a batch as an `array<T, N>` in WGSL.
///
/// This reduces the number of rebindings required due to having to pass dynamic
/// offsets to bind group commands, and if indices into the array can be passed
/// in via other means, it enables batching of draw commands.
pub struct BatchedUniformBuffer<T: GpuArrayBufferable> {
    // Batches of fixed-size arrays of T are written to this buffer so that
    // each batch in a fixed-size array can be bound at a dynamic offset.
    uniforms: DynamicUniformBuffer<MaxCapacityArray<Vec<T>>>,
    // A batch of T are gathered into this `MaxCapacityArray` until it is full,
    // then it is written into the `DynamicUniformBuffer`, cleared, and new T
    // are gathered here, and so on for each batch.
    temp: MaxCapacityArray<Vec<T>>,
    current_offset: u32,
    dynamic_offset_alignment: u32,
}

impl<T: GpuArrayBufferable> BatchedUniformBuffer<T> {
    pub fn batch_size(limits: &Limits) -> usize {
        (limits
            .max_uniform_buffer_binding_size
            .min(MAX_REASONABLE_UNIFORM_BUFFER_BINDING_SIZE) as u64
            / T::min_size().get()) as usize
    }

    pub fn new(limits: &Limits) -> Self {
        let capacity = Self::batch_size(limits);
        let alignment = limits.min_uniform_buffer_offset_alignment;

        Self {
            uniforms: DynamicUniformBuffer::new_with_alignment(alignment as u64),
            temp: MaxCapacityArray(Vec::with_capacity(capacity), capacity),
            current_offset: 0,
            dynamic_offset_alignment: alignment,
        }
    }

    #[inline]
    pub fn size(&self) -> NonZeroU64 {
        self.temp.size()
    }

    pub fn clear(&mut self) {
        self.uniforms.clear();
        self.current_offset = 0;
        self.temp.0.clear();
    }

    pub fn push(&mut self, component: T) -> GpuArrayBufferIndex<T> {
        let result = GpuArrayBufferIndex {
            index: self.temp.0.len() as u32,
            dynamic_offset: NonMaxU32::new(self.current_offset),
            element_type: PhantomData,
        };
        self.temp.0.push(component);
        if self.temp.0.len() == self.temp.1 {
            self.flush();
        }
        result
    }

    pub fn flush(&mut self) {
        self.uniforms.push(&self.temp);

        self.current_offset +=
            align_to_next(self.temp.size().get(), self.dynamic_offset_alignment as u64) as u32;

        self.temp.0.clear();
    }

    pub fn write_buffer(&mut self, device: &RenderDevice, queue: &RenderQueue) {
        if !self.temp.0.is_empty() {
            self.flush();
        }
        self.uniforms.write_buffer(device, queue);
    }

    #[inline]
    pub fn binding(&self) -> Option<BindingResource> {
        let mut binding = self.uniforms.binding();
        if let Some(BindingResource::Buffer(binding)) = &mut binding {
            // MaxCapacityArray is runtime-sized so can't use T::min_size()
            binding.size = Some(self.size());
        }
        binding
    }
}

#[inline]
fn align_to_next(value: u64, alignment: u64) -> u64 {
    debug_assert!(alignment & (alignment - 1) == 0);
    ((value - 1) | (alignment - 1)) + 1
}

// ----------------------------------------------------------------------------
// MaxCapacityArray was implemented by Teodor Tanasoaia for encase. It was
// copied here as it was not yet included in an encase release and it is
// unclear if it is the correct long-term solution for encase.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct MaxCapacityArray<T>(T, usize);

impl<T> ShaderType for MaxCapacityArray<T>
where
    T: ShaderType<ExtraMetadata = ArrayMetadata>,
{
    type ExtraMetadata = ArrayMetadata;

    const METADATA: Metadata<Self::ExtraMetadata> = T::METADATA;

    fn size(&self) -> NonZeroU64 {
        Self::METADATA.stride().mul(self.1.max(1) as u64).0
    }
}

impl<T> WriteInto for MaxCapacityArray<T>
where
    T: WriteInto + RuntimeSizedArray,
{
    fn write_into<B: BufferMut>(&self, writer: &mut Writer<B>) {
        debug_assert!(self.0.len() <= self.1);
        self.0.write_into(writer);
    }
}
