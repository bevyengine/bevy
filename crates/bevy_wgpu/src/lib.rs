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
pub struct WgpuOptions {
    pub backend: WgpuBackend,
    pub power_pref: WgpuPowerOptions,
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
