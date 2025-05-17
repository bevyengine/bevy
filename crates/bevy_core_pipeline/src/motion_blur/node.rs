use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    extract_component::ComponentUniforms,
    frame_graph::{ColorAttachment, FrameGraph, TextureView, TextureViewInfo},
    globals::GlobalsBuffer,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{Operations, PipelineCache},
    view::{Msaa, ViewTarget},
};

use crate::prepass::ViewPrepassTextures;

use super::{
    pipeline::{MotionBlurPipeline, MotionBlurPipelineId},
    MotionBlurUniform,
};

#[derive(Default)]
pub struct MotionBlurNode;

impl ViewNode for MotionBlurNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static MotionBlurPipelineId,
        &'static ViewPrepassTextures,
        &'static MotionBlurUniform,
        &'static Msaa,
    );
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (view_target, pipeline_id, prepass_textures, motion_blur, msaa): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if motion_blur.samples == 0 || motion_blur.shutter_angle <= 0.0 {
            return Ok(()); // We can skip running motion blur in these cases.
        }

        let motion_blur_pipeline = world.resource::<MotionBlurPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let settings_uniforms = world.resource::<ComponentUniforms<MotionBlurUniform>>();
        let Some(_) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
            return Ok(());
        };

        let Some(settings_handle) = settings_uniforms
            .uniforms()
            .make_binding_resource_handle(frame_graph)
        else {
            return Ok(());
        };
        let (Some(prepass_motion_vectors_texture), Some(prepass_depth_texture)) =
            (&prepass_textures.motion_vectors, &prepass_textures.depth)
        else {
            return Ok(());
        };
        let Some(globals_handle) = world
            .resource::<GlobalsBuffer>()
            .buffer
            .make_binding_resource_handle(frame_graph)
        else {
            return Ok(());
        };

        let post_process = view_target.post_process_write();

        let layout = if msaa.samples() == 1 {
            &motion_blur_pipeline.layout
        } else {
            &motion_blur_pipeline.layout_msaa
        };

        let bind_group_handle = frame_graph
            .create_bind_group_handle_builder(Some("motion_blur_bind_group".into()), layout)
            .add_helper(0, post_process.source)
            .add_helper(1, &prepass_motion_vectors_texture.texture)
            .add_helper(2, &prepass_depth_texture.texture)
            .add_handle(3, &motion_blur_pipeline.sampler)
            .add_handle(4, &settings_handle)
            .add_handle(4, &globals_handle)
            .build();

        let mut pass_builder = frame_graph.create_pass_builder("motion_blur_pass");

        let destination = pass_builder.write_material(post_process.destination);

        pass_builder
            .create_render_pass_builder()
            .set_pass_name("motion_blur_pass")
            .add_color_attachment(ColorAttachment {
                view: TextureView {
                    texture: destination,
                    desc: TextureViewInfo::default(),
                },
                resolve_target: None,
                ops: Operations::default(),
            })
            .set_render_pipeline(pipeline_id.0)
            .set_bind_group_handle(0, &bind_group_handle, &[])
            .draw(0..3, 0..1);

        Ok(())
    }
}
