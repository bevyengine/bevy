pub mod camera;
pub mod color;
pub mod main_pass;
pub mod mesh;
pub mod pass;
pub mod pipeline;
pub mod render_command;
pub mod render_graph;
pub mod render_resource;
pub mod renderer;
pub mod shader;
pub mod texture;

pub use once_cell;

use crate::{
    render_command::RenderCommandPlugin, render_graph::RenderGraph, renderer::RenderResources,
    texture::TexturePlugin,
};
use bevy_app::{App, Plugin, StartupStage};
use bevy_ecs::prelude::*;
use bevy_utils::tracing::warn;

#[derive(Default)]
pub struct RenderPlugin;

/// The names of the default App stages
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum RenderStage {
    /// Extract data from "app world" and insert it into "render world". This step should be kept
    /// as short as possible to increase the "pipelining potential" for running the next frame
    /// while rendering the current frame.
    Extract,

    /// Prepare render resources from extracted data.
    Prepare,

    /// Create Bind Groups that depend on Prepare data and queue up draw calls to run during the Render stage.
    Queue,

    /// Actual rendering happens here. In most cases, only the render backend should insert resources here
    Render,
}

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system_to_stage(
            StartupStage::PreStartup,
            check_for_render_resource_context.system(),
        );
        let mut render_app = App::empty();
        let mut extract_stage = SystemStage::parallel();
        // don't apply buffers when the stage finishes running
        // extract stage runs on the app world, but the buffers are applied to the render world
        extract_stage.set_apply_buffers(false);
        render_app
            .add_stage(RenderStage::Extract, extract_stage)
            .add_stage(RenderStage::Prepare, SystemStage::parallel())
            .add_stage(RenderStage::Queue, SystemStage::parallel())
            .add_stage(RenderStage::Render, SystemStage::parallel());
        render_app.insert_resource(RenderGraph::default());
        app.add_sub_app(render_app, |app_world, render_app| {
            // extract
            extract(app_world, render_app);

            // prepare
            let prepare = render_app
                .schedule
                .get_stage_mut::<SystemStage>(&RenderStage::Prepare)
                .unwrap();
            prepare.run(&mut render_app.world);

            // queue
            let queue = render_app
                .schedule
                .get_stage_mut::<SystemStage>(&RenderStage::Queue)
                .unwrap();
            queue.run(&mut render_app.world);

            // render
            let render = render_app
                .schedule
                .get_stage_mut::<SystemStage>(&RenderStage::Render)
                .unwrap();
            render.run(&mut render_app.world);

            render_app.world.clear_entities();
        });

        app.add_plugin(RenderCommandPlugin)
            .add_plugin(TexturePlugin);
    }
}

fn extract(app_world: &mut World, render_app: &mut App) {
    let extract = render_app
        .schedule
        .get_stage_mut::<SystemStage>(&RenderStage::Extract)
        .unwrap();
    extract.run(app_world);
    extract.apply_buffers(&mut render_app.world);
}

fn check_for_render_resource_context(context: Option<Res<RenderResources>>) {
    if context.is_none() {
        warn!(
            "bevy_render couldn't find a render backend. Perhaps try adding the bevy_wgpu feature/plugin!"
        );
    }
}
