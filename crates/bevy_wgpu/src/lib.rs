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

#[derive(Clone, Copy)]
pub enum DeviceFeatures {
    DepthClamping,
    TextureCompressionBC,
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
}

impl From<DeviceFeatures> for wgpu::Features {
    fn from(value: DeviceFeatures) -> Self {
        match value {
            DeviceFeatures::DepthClamping => wgpu::Features::DEPTH_CLAMPING,
            DeviceFeatures::TextureCompressionBC => wgpu::Features::TEXTURE_COMPRESSION_BC,
            DeviceFeatures::MappablePrimaryBuffers => wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
            DeviceFeatures::SampledTextureBindingArray => {
                wgpu::Features::SAMPLED_TEXTURE_BINDING_ARRAY
            }
            DeviceFeatures::SampledTextureArrayDynamicIndexing => {
                wgpu::Features::SAMPLED_TEXTURE_ARRAY_DYNAMIC_INDEXING
            }
            DeviceFeatures::SampledTextureArrayNonUniformIndexing => {
                wgpu::Features::SAMPLED_TEXTURE_ARRAY_NON_UNIFORM_INDEXING
            }
            DeviceFeatures::UnsizedBindingArray => wgpu::Features::UNSIZED_BINDING_ARRAY,
            DeviceFeatures::MultiDrawIndirect => wgpu::Features::MULTI_DRAW_INDIRECT,
            DeviceFeatures::MultiDrawIndirectCount => wgpu::Features::MULTI_DRAW_INDIRECT_COUNT,
            DeviceFeatures::PushConstants => wgpu::Features::PUSH_CONSTANTS,
            DeviceFeatures::AddressModeClampToBorder => {
                wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
            }
            DeviceFeatures::NonFillPolygonMode => wgpu::Features::NON_FILL_POLYGON_MODE,
        }
    }
}

#[derive(Default)]
pub struct WgpuDeviceFeatures {
    pub features: Vec<DeviceFeatures>,
}

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
    let mut wgpu_renderer = future::block_on(WgpuRenderer::new());

    let mut wgpu_features = wgpu::Features::empty();
    if let Some(device_features_res) = resources.get::<WgpuDeviceFeatures>() {
        wgpu_features = device_features_res.features.iter().fold(
            wgpu::Features::empty(),
            |wgpu_features, feature| {
                let feature: wgpu::Features = (*feature).into();
                let wgpu_features = wgpu_features | feature;
                wgpu_features
            },
        );
    }

      let options = resources
        .get_cloned::<WgpuOptions>()
        .unwrap_or_else(WgpuOptions::default);
    let mut wgpu_renderer = future::block_on(WgpuRenderer::new(options, wgpu_features));

    let resource_context = WgpuRenderResourceContext::new(wgpu_renderer.device.clone());
    resources.insert::<Box<dyn RenderResourceContext>>(Box::new(resource_context));
    resources.insert(SharedBuffers::new(4096));
    move |world, resources| {
        wgpu_renderer.update(world, resources);
    }
}

#[derive(Default, Clone)]
pub struct WgpuOptions {
    power_pref: WgpuPowerOptions,
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
