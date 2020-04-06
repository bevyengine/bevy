mod wgpu_render_pass;
mod wgpu_renderer;
mod wgpu_resources;
mod wgpu_type_converter;

pub use wgpu_render_pass::*;
pub use wgpu_renderer::*;
pub use wgpu_resources::*;

use bevy_app::{AppPlugin, AppBuilder, Events};
use bevy_render::{renderer::Renderer, RENDER_STAGE};
use bevy_window::{WindowCreated, WindowResized};
use legion::prelude::*;

#[derive(Default)]
pub struct WgpuRendererPlugin;

impl AppPlugin for WgpuRendererPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let render_system = wgpu_render_system(app.resources());
        app.add_thread_local_fn_to_stage(RENDER_STAGE, render_system);
    }
}

pub fn wgpu_render_system(resources: &Resources) -> impl FnMut(&mut World, &mut Resources) {
    let window_resized_event = resources.get::<Events<WindowResized>>().unwrap();
    let window_created_event = resources.get::<Events<WindowCreated>>().unwrap();
    let mut wgpu_renderer = futures::executor::block_on(WgpuRenderer::new(
        window_resized_event.get_reader(),
        window_created_event.get_reader(),
    ));
    move |world, resources| {
        wgpu_renderer.update(world, resources);
    }
}
