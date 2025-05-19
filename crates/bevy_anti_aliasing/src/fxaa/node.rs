use crate::fxaa::{CameraFxaaPipeline, Fxaa};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    frame_graph::{ColorAttachment, FrameGraph, TextureView, TextureViewInfo},
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{Operations, PipelineCache},
    view::ViewTarget,
};

use super::FxaaPipeline;

#[derive(Default)]
pub struct FxaaNode;

impl ViewNode for FxaaNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static CameraFxaaPipeline,
        &'static Fxaa,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (target, pipeline, fxaa): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let fxaa_pipeline = world.resource::<FxaaPipeline>();

        if !fxaa.enabled {
            return Ok(());
        };

        let Some(_) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
            return Ok(());
        };

        let post_process = target.post_process_write();

        let bind_group_handle = frame_graph
            .create_bind_group_handle_builder(None, &fxaa_pipeline.texture_bind_group)
            .add_helper(0, post_process.source)
            .add_handle(1, &fxaa_pipeline.sampler)
            .build();

        let mut pass_builder = frame_graph.create_pass_builder("fxaa_node");

        let destination = pass_builder.write_material(post_process.destination);

        pass_builder
            .create_render_pass_builder("fxaa_pass")
            .add_color_attachment(ColorAttachment {
                view: TextureView {
                    texture: destination,
                    desc: TextureViewInfo::default(),
                },
                resolve_target: None,
                ops: Operations::default(),
            })
            .set_render_pipeline(pipeline.pipeline_id)
            .set_bind_group_handle(0, &bind_group_handle, &[])
            .draw(0..3, 0..1);

        Ok(())
    }
}
