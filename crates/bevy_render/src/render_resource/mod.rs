mod bind_group;
mod bind_group_layout;
mod buffer;
mod buffer_vec;
mod pipeline;
mod pipeline_cache;
mod pipeline_specializer;
pub mod resource_macros;
mod shader;
mod storage_buffer;
mod texture;
mod uniform_buffer;

pub use bind_group::*;
pub use bind_group_layout::*;
pub use buffer::*;
pub use buffer_vec::*;
pub use pipeline::*;
pub use pipeline_cache::*;
pub use pipeline_specializer::*;
pub use shader::*;
pub use storage_buffer::*;
pub use texture::*;
pub use uniform_buffer::*;

// TODO: decide where re-exports should go
pub use wgpu::{
    util::BufferInitDescriptor, AddressMode, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendComponent,
    BlendFactor, BlendOperation, BlendState, BufferAddress, BufferBinding, BufferBindingType,
    BufferDescriptor, BufferSize, BufferUsages, ColorTargetState, ColorWrites, CommandEncoder,
    CommandEncoderDescriptor, CompareFunction, ComputePass, ComputePassDescriptor, DepthBiasState,
    DepthStencilState, Extent3d, Face, Features, FilterMode, FrontFace, ImageCopyBuffer,
    ImageCopyBufferBase, ImageCopyTexture, ImageCopyTextureBase, ImageDataLayout,
    ImageSubresourceRange, IndexFormat, Limits, LoadOp, MapMode, MultisampleState, Operations,
    Origin3d, PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PrimitiveState,
    PrimitiveTopology, RenderPass, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, SamplerBindingType, SamplerDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilFaceState, StencilOperation,
    StencilState, StorageTextureAccess, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension,
    VertexAttribute, VertexFormat, VertexStepMode,
};

pub use wgpu::{
    ComputePipelineDescriptor as RawComputePipelineDescriptor, FragmentState as RawFragmentState,
    RenderPipelineDescriptor as RawRenderPipelineDescriptor,
    VertexBufferLayout as RawVertexBufferLayout, VertexState as RawVertexState,
};

pub mod encase {
    pub use bevy_encase_derive::ShaderType;
    pub use encase::*;
}

pub use self::encase::{ShaderSize, ShaderType};

pub use naga::ShaderStage;
