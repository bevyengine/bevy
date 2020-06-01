pub mod diagnostic;
pub mod renderer;
mod wgpu_render_pass;
mod wgpu_renderer;
mod wgpu_resources;
mod wgpu_type_converter;

pub use wgpu_render_pass::*;
pub use wgpu_renderer::*;
pub use wgpu_resources::*;

use bevy_app::{AppBuilder, AppPlugin, Events};
use bevy_render::{renderer::RenderResources};
use bevy_window::{WindowCreated, WindowResized};
use legion::prelude::*;
use renderer::WgpuRenderResourceContext;

#[derive(Default)]
pub struct WgpuPlugin;

impl AppPlugin for WgpuPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let render_system = wgpu_render_system(app.resources_mut());
        app.add_system_to_stage(bevy_render::stage::RENDER, render_system);
    }
}

pub fn wgpu_render_system(resources: &mut Resources) -> impl FnMut(&mut World, &mut Resources) {
    let mut wgpu_renderer = {
        let window_resized_event = resources.get::<Events<WindowResized>>().unwrap();
        let window_created_event = resources.get::<Events<WindowCreated>>().unwrap();
        futures::executor::block_on(WgpuRenderer::new(
            window_resized_event.get_reader(),
            window_created_event.get_reader(),
        ))
    };
    resources.insert(RenderResources::new(WgpuRenderResourceContext::new(
        wgpu_renderer.device.clone(),
    )));
    move |world, resources| {
        wgpu_renderer.update(world, resources);
    }
}
