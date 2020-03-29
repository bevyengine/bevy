mod wgpu_render_pass;
mod wgpu_renderer;
mod wgpu_resources;
mod wgpu_type_converter;

pub use wgpu_render_pass::*;
pub use wgpu_renderer::*;
pub use wgpu_resources::*;

use crate::{app::AppBuilder, plugin::AppPlugin};

pub struct WgpuRendererPlugin;

impl AppPlugin for WgpuRendererPlugin {
    fn build(&self, app: AppBuilder) -> AppBuilder {
        // let render_context = app.resources.get_mut::<RenderContext>().unwrap();
        // render_context.renderer = Some(Box::new(WgpuRenderer::new()));
        app
    }
    fn name(&self) -> &'static str {
        "WgpuRenderer"
    }
}
