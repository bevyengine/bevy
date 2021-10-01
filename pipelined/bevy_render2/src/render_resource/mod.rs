mod bind_group;
mod buffer;
mod buffer_vec;
mod pipeline;
mod render_resource_id;
mod texture;
mod uniform_vec;

pub use bind_group::*;
pub use buffer::*;
pub use buffer_vec::*;
pub use pipeline::*;
pub use render_resource_id::*;
pub use texture::*;
pub use uniform_vec::*;

use std::sync::atomic::{AtomicU64, Ordering};
// TODO: decide where re-exports should go
pub use wgpu::{
    util::BufferInitDescriptor, AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent,
    BlendFactor, BlendOperation, BlendState, BufferAddress, BufferBindingType, BufferSize,
    BufferUsage, ColorTargetState, ColorWrite, CompareFunction, ComputePassDescriptor,
    ComputePipelineDescriptor, DepthBiasState, DepthStencilState, Extent3d, Face, FilterMode,
    FragmentState, FrontFace, IndexFormat, InputStepMode, LoadOp, MultisampleState, Operations,
    PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, SamplerDescriptor, ShaderFlags, ShaderModule, ShaderModuleDescriptor,
    ShaderSource, ShaderStage, StencilFaceState, StencilOperation, StencilState,
    StorageTextureAccess, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsage, TextureViewDescriptor, TextureViewDimension, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState,
};

// Change this to AtomicU32, if AtomicU64 is not supported on your platform.
type Id = u64;
type Counter = AtomicU64;

/// Increments the `counter` and returns the previous id.
/// Returns [`None`] if the supply of unique ids has been exhausted.
#[inline]
fn next_id(counter: &Counter) -> Option<Id> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |val| {
            val.checked_add(1)
        })
        .ok()
}
