pub mod diagnostic;
pub mod renderer;
mod wgpu_render_pass;
mod wgpu_renderer;
mod wgpu_resources;
mod wgpu_type_converter;

use futures_lite::future;
pub use wgpu_render_pass::*;
pub use wgpu_renderer::*;
pub use wgpu_resources::*;

use bevy_app::prelude::*;
use bevy_ecs::{IntoSystem, Resources, World};
use bevy_render::renderer::{shared_buffers_update_system, RenderResourceContext, SharedBuffers};
use renderer::WgpuRenderResourceContext;

#[derive(Default)]
pub struct WgpuPlugin;

impl Plugin for WgpuPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let render_system = get_wgpu_render_system(app.resources_mut());
        app.add_system_to_stage(bevy_render::stage::RENDER, render_system.system())
            .add_system_to_stage(
                bevy_render::stage::POST_RENDER,
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
pub struct WgpuOptions<'a> {
    pub name: Option<&'a str>,
    pub backend: WgpuBackend,
    pub power_pref: WgpuPowerOptions,
    pub features: WgpuFeatures,
    pub limits: WgpuLimits,
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

bitflags::bitflags! {
    pub struct WgpuFeatures: u64 {
        const DEPTH_CLAMPING = 0x0000_0000_0000_0001;
        const TEXTURE_COMPRESSION_BC = 0x0000_0000_0000_0002;
        const TIMESTAMP_QUERY = 0x0000_0000_0000_0004;
        const PIPELINE_STATISTICS_QUERY = 0x0000_0000_0000_0008;
        const MAPPABLE_PRIMARY_BUFFERS = 0x0000_0000_0001_0000;
        const SAMPLED_TEXTURE_BINDING_ARRAY = 0x0000_0000_0002_0000;
        const SAMPLED_TEXTURE_ARRAY_DYNAMIC_INDEXING = 0x0000_0000_0004_0000;
        const SAMPLED_TEXTURE_ARRAY_NON_UNIFORM_INDEXING = 0x0000_0000_0008_0000;
        const UNSIZED_BINDING_ARRAY = 0x0000_0000_0010_0000;
        const MULTI_DRAW_INDIRECT = 0x0000_0000_0020_0000;
        const MULTI_DRAW_INDIRECT_COUNT = 0x0000_0000_0040_0000;
        const PUSH_CONSTANTS = 0x0000_0000_0080_0000;
        const ADDRESS_MODE_CLAMP_TO_BORDER = 0x0000_0000_0100_0000;
        const NON_FILL_POLYGON_MODE = 0x0000_0000_0200_0000;
        const TEXTURE_COMPRESSION_ETC2 = 0x0000_0000_0400_0000;
        const TEXTURE_COMPRESSION_ASTC_LDR = 0x0000_0000_0800_0000;
        const TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES = 0x0000_0000_1000_0000;
        const SHADER_FLOAT64 = 0x0000_0000_2000_0000;
        const VERTEX_ATTRIBUTE_64BIT = 0x0000_0000_4000_0000;
        const ALL_WEBGPU = 0x0000_0000_0000_FFFF;
        const ALL_NATIVE = 0xFFFF_FFFF_FFFF_0000;
    }
}

impl Default for WgpuFeatures {
    fn default() -> Self {
        WgpuFeatures::empty()
    }
}

#[derive(Debug, Clone)]
pub struct WgpuLimits {
    pub max_bind_groups: u32,
    pub max_dynamic_uniform_buffers_per_pipeline_layout: u32,
    pub max_dynamic_storage_buffers_per_pipeline_layout: u32,
    pub max_sampled_textures_per_shader_stage: u32,
    pub max_samplers_per_shader_stage: u32,
    pub max_storage_buffers_per_shader_stage: u32,
    pub max_storage_textures_per_shader_stage: u32,
    pub max_uniform_buffers_per_shader_stage: u32,
    pub max_uniform_buffer_binding_size: u32,
    pub max_push_constant_size: u32,
}

impl Default for WgpuLimits {
    fn default() -> Self {
        let default = wgpu::Limits::default();
        WgpuLimits {
            max_bind_groups: default.max_bind_groups,
            max_dynamic_uniform_buffers_per_pipeline_layout: default
                .max_dynamic_uniform_buffers_per_pipeline_layout,
            max_dynamic_storage_buffers_per_pipeline_layout: default
                .max_dynamic_storage_buffers_per_pipeline_layout,
            max_sampled_textures_per_shader_stage: default.max_sampled_textures_per_shader_stage,
            max_samplers_per_shader_stage: default.max_samplers_per_shader_stage,
            max_storage_buffers_per_shader_stage: default.max_storage_buffers_per_shader_stage,
            max_storage_textures_per_shader_stage: default.max_storage_textures_per_shader_stage,
            max_uniform_buffers_per_shader_stage: default.max_uniform_buffers_per_shader_stage,
            max_uniform_buffer_binding_size: default.max_uniform_buffer_binding_size,
            max_push_constant_size: default.max_push_constant_size,
        }
    }
}
