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

// TODO: decide where re-exports should go
pub use wgpu::util::BufferInitDescriptor;
pub use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent, BlendFactor,
    BlendOperation, BlendState, BufferAddress, BufferBindingType, BufferSize, BufferUsage,
    ColorTargetState, ColorWrite, CompareFunction, ComputePassDescriptor,
    ComputePipelineDescriptor, DepthBiasState, DepthStencilState, Extent3d, Face, FilterMode,
    FragmentState, FrontFace, IndexFormat, InputStepMode, LoadOp, MultisampleState, Operations,
    PipelineLayoutDescriptor, PolygonMode, PrimitiveState, PrimitiveTopology,
    RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    RenderPipelineDescriptor, SamplerDescriptor, ShaderFlags, ShaderModule, ShaderModuleDescriptor,
    ShaderSource, ShaderStage, StencilFaceState, StencilState, StorageTextureAccess, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsage,
    TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState,
};
