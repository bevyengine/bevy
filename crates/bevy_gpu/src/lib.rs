mod gpu;
mod resource;
mod resource_macros;
mod settings;

pub use gpu::*;
pub use naga::ShaderStage;
pub use resource::*;
pub use resource_macros::*;
pub use settings::*;

pub use wgpu::{
    util::BufferInitDescriptor, AddressMode, AstcBlock, AstcChannel, Backends, BindGroupDescriptor,
    BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
    BlendComponent, BlendFactor, BlendOperation, BlendState, BufferAddress, BufferBinding,
    BufferBindingType, BufferDescriptor, BufferSize, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoder, CommandEncoderDescriptor, CompareFunction, ComputePass, ComputePassDescriptor,
    DepthBiasState, DepthStencilState, Extent3d, Face, Features, FilterMode, FrontFace,
    ImageCopyBuffer, ImageCopyBufferBase, ImageCopyTexture, ImageCopyTextureBase, ImageDataLayout,
    ImageSubresourceRange, IndexFormat, Limits, LoadOp, MapMode, MultisampleState, Operations,
    Origin3d, PipelineLayout, PipelineLayoutDescriptor, PolygonMode, PowerPreference,
    PrimitiveState, PrimitiveTopology, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, SamplerBindingType, SamplerDescriptor,
    ShaderModule, ShaderModuleDescriptor, ShaderSource, ShaderStages, StencilFaceState,
    StencilOperation, StencilState, StorageTextureAccess, Surface, SurfaceConfiguration,
    SurfaceError, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension, VertexAttribute,
    VertexFormat, VertexStepMode,
};
pub use wgpu::{
    Color as RawColor, CompositeAlphaMode as RawCompositeAlphaMode,
    ComputePipelineDescriptor as RawComputePipelineDescriptor, FragmentState as RawFragmentState,
    PresentMode as RawPresentMode, RenderPipelineDescriptor as RawRenderPipelineDescriptor,
    VertexBufferLayout as RawVertexBufferLayout, VertexState as RawVertexState,
};

// TODO: can we reexport only the necessary types?
pub mod encase {
    pub use bevy_encase_derive::ShaderType;
    pub use encase::*;
}

pub use self::encase::{ShaderSize, ShaderType};

use bevy_app::{App, Plugin};
use bevy_asset::AddAsset;
use bevy_utils::tracing::debug;
use bevy_window::Windows;
use wgpu::RequestAdapterOptions;

/// Contains the default Bevy GPU abstraction based on wgpu.
#[derive(Default)]
pub struct GpuPlugin {
    pub settings: Settings,
}

impl Plugin for GpuPlugin {
    /// Initializes the the wgpu backend.
    fn build(&self, app: &mut App) {
        app.add_asset::<Shader>()
            .add_debug_asset::<Shader>()
            .init_asset_loader::<ShaderLoader>()
            .init_debug_asset_loader::<ShaderLoader>();

        if let Some(backends) = self.settings.backends {
            let instance = Instance::new(backends);
            let windows = app.world.resource_mut::<Windows>();

            let surface = windows
                .get_primary()
                .and_then(|window| window.raw_handle())
                .map(|wrapper| unsafe {
                    let handle = wrapper.get_handle();
                    instance.create_surface(&handle)
                });

            let request_adapter_options = RequestAdapterOptions {
                power_preference: self.settings.power_preference,
                compatible_surface: surface.as_ref(),
                ..Default::default()
            };

            let (device, queue, adapter_info, adapter) = futures_lite::future::block_on(
                initialize_gpu(&instance, &self.settings, &request_adapter_options),
            );
            debug!("Configured wgpu adapter Limits: {:#?}", device.limits());
            debug!("Configured wgpu adapter Features: {:#?}", device.features());
            app.insert_resource(instance)
                .insert_resource(device)
                .insert_resource(queue)
                .insert_resource(adapter)
                .insert_resource(adapter_info);
        }
    }
}
