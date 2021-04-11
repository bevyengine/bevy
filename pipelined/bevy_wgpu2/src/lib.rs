pub mod diagnostic;

mod render_context;
mod render_graph_executor;
mod render_pass;
mod render_resource_context;
mod renderer;
mod resources;
mod type_converter;

pub use render_context::*;
pub use render_pass::*;
pub use render_resource_context::*;
pub use renderer::*;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_render2::{
    render_graph::{ExtractedWindow, ExtractedWindows, RawWindowHandleWrapper},
    renderer::RenderResources,
    RenderStage,
};
use bevy_window::Windows;
use bevy_winit::WinitWindows;
use futures_lite::future;
use raw_window_handle::HasRawWindowHandle;
use std::borrow::Cow;

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
        let options = app
            .world
            .get_resource::<WgpuOptions>()
            .cloned()
            .unwrap_or_else(WgpuOptions::default);
        let wgpu_renderer = future::block_on(WgpuRenderer::new(options));
        let resource_context = WgpuRenderResourceContext::new(wgpu_renderer.device.clone());
        app.world
            .insert_resource(RenderResources::new(Box::new(resource_context.clone())));
        let render_app = app.sub_app_mut(0);
        render_app.insert_resource(RenderResources::new(Box::new(resource_context)));

        let render_system = get_wgpu_render_system(wgpu_renderer);
        render_app
            .add_system_to_stage(RenderStage::Extract, extract_windows.system())
            .add_system_to_stage(RenderStage::Render, render_system.exclusive_system());
    }
}

pub fn get_wgpu_render_system(mut wgpu_renderer: WgpuRenderer) -> impl FnMut(&mut World) {
    move |world| {
        wgpu_renderer.update(world);
    }
}

fn extract_windows(
    mut commands: Commands,
    winit_windows: Res<WinitWindows>,
    windows: Res<Windows>,
) {
    let mut extracted_windows = ExtractedWindows::default();
    for window in windows.iter() {
        let winit_window = winit_windows.get_window(window.id()).unwrap();
        extracted_windows.insert(
            window.id(),
            ExtractedWindow {
                id: window.id(),
                handle: RawWindowHandleWrapper(winit_window.raw_window_handle()),
                physical_width: window.physical_width(),
                physical_height: window.physical_height(),
                vsync: window.vsync(),
            },
        );
    }

    commands.insert_resource(extracted_windows);
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
