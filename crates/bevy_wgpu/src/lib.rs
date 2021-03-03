pub mod diagnostic;
pub mod renderer;
mod wgpu_render_pass;
mod wgpu_renderer;
mod wgpu_resources;
mod wgpu_type_converter;

pub use wgpu_render_pass::*;
pub use wgpu_renderer::*;
pub use wgpu_resources::*;

use bevy_app::prelude::*;
use bevy_ecs::{IntoExclusiveSystem, IntoSystem, Resources, World};
use bevy_render::{
    renderer::{shared_buffers_update_system, RenderResourceContext, SharedBuffers},
    RenderStage,
};
use futures_lite::future;
use renderer::WgpuRenderResourceContext;

#[derive(Clone, Copy)]
pub enum WgpuFeature {
    DepthClamping,
    TextureCompressionBc,
    TimestampQuery,
    PipelineStatisticsQuery,
    MappablePrimaryBuffers,
    SampledTextureBindingArray,
    SampledTextureArrayDynamicIndexing,
    SampledTextureArrayNonUniformIndexing,
    UnsizedBindingArray,
    MultiDrawIndirect,
    MultiDrawIndirectCount,
    PushConstants,
    AddressModeClampToBorder,
    NonFillPolygonMode,
    TextureCompressionEtc2,
    TextureCompressionAstcLdr,
    TextureAdapterSpecificFormatFeatures,
    ShaderFloat64,
    VertexAttribute64Bit,
}

impl From<WgpuFeature> for wgpu::Features {
    fn from(value: WgpuFeature) -> Self {
        match value {
            WgpuFeature::DepthClamping => wgpu::Features::DEPTH_CLAMPING,
            WgpuFeature::TextureCompressionBc => wgpu::Features::TEXTURE_COMPRESSION_BC,
            WgpuFeature::TimestampQuery => wgpu::Features::TIMESTAMP_QUERY,
            WgpuFeature::PipelineStatisticsQuery => wgpu::Features::PIPELINE_STATISTICS_QUERY,
            WgpuFeature::MappablePrimaryBuffers => wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
            WgpuFeature::SampledTextureBindingArray => {
                wgpu::Features::SAMPLED_TEXTURE_BINDING_ARRAY
            }
            WgpuFeature::SampledTextureArrayDynamicIndexing => {
                wgpu::Features::SAMPLED_TEXTURE_ARRAY_DYNAMIC_INDEXING
            }
            WgpuFeature::SampledTextureArrayNonUniformIndexing => {
                wgpu::Features::SAMPLED_TEXTURE_ARRAY_NON_UNIFORM_INDEXING
            }
            WgpuFeature::UnsizedBindingArray => wgpu::Features::UNSIZED_BINDING_ARRAY,
            WgpuFeature::MultiDrawIndirect => wgpu::Features::MULTI_DRAW_INDIRECT,
            WgpuFeature::MultiDrawIndirectCount => wgpu::Features::MULTI_DRAW_INDIRECT_COUNT,
            WgpuFeature::PushConstants => wgpu::Features::PUSH_CONSTANTS,
            WgpuFeature::AddressModeClampToBorder => wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER,
            WgpuFeature::NonFillPolygonMode => wgpu::Features::NON_FILL_POLYGON_MODE,
            WgpuFeature::TextureCompressionEtc2 => wgpu::Features::TEXTURE_COMPRESSION_ETC2,
            WgpuFeature::TextureCompressionAstcLdr => wgpu::Features::TEXTURE_COMPRESSION_ASTC_LDR,
            WgpuFeature::TextureAdapterSpecificFormatFeatures => {
                wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            }
            WgpuFeature::ShaderFloat64 => wgpu::Features::SHADER_FLOAT64,
            WgpuFeature::VertexAttribute64Bit => wgpu::Features::VERTEX_ATTRIBUTE_64BIT,
        }
    }
}

impl From<WgpuFeatures> for wgpu::Features {
    fn from(features: WgpuFeatures) -> Self {
        features
            .features
            .iter()
            .fold(wgpu::Features::empty(), |wgpu_features, feature| {
                wgpu_features | (*feature).into()
            })
    }
}

#[derive(Default, Clone)]
pub struct WgpuFeatures {
    pub features: Vec<WgpuFeature>,
}

#[derive(Default)]
pub struct WgpuPlugin;

impl Plugin for WgpuPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let render_system = get_wgpu_render_system(app.resources_mut());
        app.add_system_to_stage(RenderStage::Render, render_system.exclusive_system())
            .add_system_to_stage(
                RenderStage::PostRender,
                shared_buffers_update_system.system(),
            );
    }
}
pub fn get_wgpu_render_system(resources: &mut Resources) -> impl FnMut(&mut World, &mut Resources) {
    let options = resources
        .get_cloned::<WgpuOptions>()
        .unwrap_or_else(WgpuOptions::default);
    let mut wgpu_renderer = future::block_on(WgpuRenderer::new(options));

    let resource_context = WgpuRenderResourceContext::new(wgpu_renderer.device.clone());
    resources.insert::<Box<dyn RenderResourceContext>>(Box::new(resource_context));
    resources.insert(SharedBuffers::new(4096));
    move |world, resources| {
        wgpu_renderer.update(world, resources);
    }
}

#[derive(Default, Clone)]
pub struct WgpuOptions {
    pub backend: WgpuBackend,
    pub power_pref: WgpuPowerOptions,
    pub features: WgpuFeatures,
}

#[derive(Clone)]
pub enum WgpuBackend {
    Auto,
    Vulkan,
    Metal,
    Dx12,
    Dx11,
    GL,
    BrowserWgpu,
}

impl WgpuBackend {
    fn from_env() -> Self {
        if let Ok(backend) = std::env::var("BEVY_WGPU_BACKEND") {
            match backend.to_lowercase().as_str() {
                "vulkan" => WgpuBackend::Vulkan,
                "metal" => WgpuBackend::Metal,
                "dx12" => WgpuBackend::Dx12,
                "dx11" => WgpuBackend::Dx11,
                "gl" => WgpuBackend::GL,
                "webgpu" => WgpuBackend::BrowserWgpu,
                other => panic!("Unknown backend: {}", other),
            }
        } else {
            WgpuBackend::Auto
        }
    }
}

impl Default for WgpuBackend {
    fn default() -> Self {
        Self::from_env()
    }
}

#[derive(Clone)]
pub enum WgpuPowerOptions {
    HighPerformance,
    Adaptive,
    LowPower,
}

impl Default for WgpuPowerOptions {
    fn default() -> Self {
        WgpuPowerOptions::HighPerformance
    }
}
