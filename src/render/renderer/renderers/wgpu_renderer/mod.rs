mod wgpu_render_pass;
mod wgpu_renderer;
mod wgpu_resources;
mod wgpu_type_converter;

pub use wgpu_render_pass::*;
pub use wgpu_renderer::*;
pub use wgpu_resources::*;

use crate::{
    app::{plugin::AppPlugin, AppBuilder, system_stage},
    core::{Event, WindowResize},
    render::renderer::Renderer,
};

use legion::prelude::*;

#[derive(Default)]
pub struct WgpuRendererPlugin;

impl AppPlugin for WgpuRendererPlugin {
    fn build(&self, app: AppBuilder) -> AppBuilder {
        let render_system = wgpu_render_system(&app.resources);
        app.add_thread_local_to_stage(system_stage::RENDER, render_system)
    }
    fn name(&self) -> &'static str {
        "WgpuRenderer"
    }
}

pub fn wgpu_render_system(resources: &Resources) -> impl FnMut(&mut World, &mut Resources) {
    let window_resize_event = resources.get::<Event<WindowResize>>().unwrap();
    let mut wgpu_renderer = WgpuRenderer::new(window_resize_event.get_handle());
    move |world, resources| {
        wgpu_renderer.update(world, resources);
    }
}