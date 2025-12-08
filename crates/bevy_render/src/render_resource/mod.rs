mod batched_uniform_buffer;
mod bind_group;
mod bind_group_entries;
mod bind_group_layout;
mod bindless;
mod buffer;
mod buffer_vec;
mod gpu_array_buffer;
mod pipeline;
mod pipeline_cache;
mod pipeline_specializer;
pub mod resource_macros;
mod specializer;
mod storage_buffer;
mod texture;
mod uniform_buffer;

pub use bind_group::*;
pub use bind_group_entries::*;
pub use bind_group_layout::*;
pub use bindless::*;
pub use buffer::*;
pub use buffer_vec::*;
pub use gpu_array_buffer::*;
pub use pipeline::*;
pub use pipeline_cache::*;
pub use pipeline_specializer::*;
pub use specializer::*;
pub use storage_buffer::*;
pub use texture::*;
pub use uniform_buffer::*;

// TODO: decide where re-exports should go
pub use bevy_material::render_resource::*;
pub use wgpu::{
    util::{
        BufferInitDescriptor, DispatchIndirectArgs, DrawIndexedIndirectArgs, DrawIndirectArgs,
        TextureDataOrder,
    },
    BindGroupDescriptor, BindGroupEntry, BindingResource, Blas, BlasBuildEntry, BlasGeometries,
    BlasTriangleGeometry, BufferAsyncError, BufferBinding, CommandEncoder, ComputePass,
    ComputePassDescriptor, ComputePipelineDescriptor as RawComputePipelineDescriptor,
    FragmentState as RawFragmentState, MapMode, PipelineCompilationOptions, PipelineLayout,
    PipelineLayoutDescriptor, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    RenderPassDescriptor, RenderPipelineDescriptor as RawRenderPipelineDescriptor,
    Sampler as WgpuSampler, ShaderModule, ShaderModuleDescriptor, ShaderSource, TextureDescriptor,
    TextureView as WgpuTextureView, Tlas, TlasInstance,
    VertexBufferLayout as RawVertexBufferLayout, VertexState as RawVertexState,
};

pub mod encase {
    pub use bevy_encase_derive::ShaderType;
    pub use encase::*;
}

pub use self::encase::{ShaderSize, ShaderType};

pub use naga::ShaderStage;
