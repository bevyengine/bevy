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
use bevy_ecs::{
    system::{IntoExclusiveSystem, IntoSystem},
    world::World,
};
use bevy_render::{
    renderer::{shared_buffers_update_system, RenderResourceContext, SharedBuffers},
    RenderStage,
};
use futures_lite::future;
use renderer::WgpuRenderResourceContext;
use std::borrow::Cow;

#[derive(Clone, Copy)]
pub enum WgpuFeature {
    DepthClamping,
    TextureCompressionBc,
    TimestampQuery,
    PipelineStatisticsQuery,
    MappablePrimaryBuffers,
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

#[derive(Default, Clone)]
pub struct WgpuFeatures {
    pub features: Vec<WgpuFeature>,
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
    pub max_texture_dimension_1d: u32,
    pub max_texture_dimension_2d: u32,
    pub max_texture_dimension_3d: u32,
    pub max_texture_array_layers: u32,
    pub max_storage_buffer_binding_size: u32,
    pub max_vertex_buffers: u32,
    pub max_vertex_attributes: u32,
    pub max_vertex_buffer_array_stride: u32,
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
            max_texture_dimension_1d: default.max_texture_dimension_1d,
            max_texture_dimension_2d: default.max_texture_dimension_2d,
            max_texture_dimension_3d: default.max_texture_dimension_3d,
            max_texture_array_layers: default.max_texture_array_layers,
            max_storage_buffer_binding_size: default.max_storage_buffer_binding_size,
            max_vertex_buffers: default.max_vertex_buffers,
            max_vertex_attributes: default.max_vertex_attributes,
            max_vertex_buffer_array_stride: default.max_vertex_buffer_array_stride,
        }
    }
}

#[derive(Default)]
pub struct WgpuPlugin;

impl Plugin for WgpuPlugin {
    fn build(&self, app: &mut App) {
        let render_system = get_wgpu_render_system(&mut app.world);
        app.add_system_to_stage(RenderStage::Render, render_system.exclusive_system())
            .add_system_to_stage(
                RenderStage::PostRender,
                shared_buffers_update_system.system(),
            );
    }
}

pub fn get_wgpu_render_system(world: &mut World) -> impl FnMut(&mut World) {
    let options = world
        .get_resource::<WgpuOptions>()
        .cloned()
        .unwrap_or_else(WgpuOptions::default);
    let mut wgpu_renderer = future::block_on(WgpuRenderer::new(options));

    let resource_context = WgpuRenderResourceContext::new(wgpu_renderer.device.clone());
    world.insert_resource::<Box<dyn RenderResourceContext>>(Box::new(resource_context));
    world.insert_resource(SharedBuffers::new(4096));
    move |world| {
        wgpu_renderer.update(world);
    }
}

#[derive(Default, Clone)]
pub struct WgpuOptions {
    pub device_label: Option<Cow<'static, str>>,
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
    Gl,
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
                "gl" => WgpuBackend::Gl,
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
