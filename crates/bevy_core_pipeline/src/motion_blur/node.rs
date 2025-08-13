use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    diagnostic::RecordDiagnostics,
    extract_component::ComponentUniforms,
    globals::GlobalsBuffer,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        BindGroupEntries, Operations, PipelineCache, RenderPassColorAttachment,
        RenderPassDescriptor,
    },
    renderer::RenderContext,
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
        render_context: &mut RenderContext,
        (view_target, pipeline_id, prepass_textures, motion_blur, msaa): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        if motion_blur.samples == 0 || motion_blur.shutter_angle <= 0.0 {
            return Ok(()); // We can skip running motion blur in these cases.
        }

        let motion_blur_pipeline = world.resource::<MotionBlurPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let settings_uniforms = world.resource::<ComponentUniforms<MotionBlurUniform>>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
            return Ok(());
        };

        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };
        let (Some(prepass_motion_vectors_texture), Some(prepass_depth_texture)) =
            (&prepass_textures.motion_vectors, &prepass_textures.depth)
        else {
            return Ok(());
        };
        let Some(globals_uniforms) = world.resource::<GlobalsBuffer>().buffer.binding() else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let post_process = view_target.post_process_write();

        let layout = if msaa.samples() == 1 {
            &motion_blur_pipeline.layout
        } else {
            &motion_blur_pipeline.layout_msaa
        };

        let bind_group = render_context.render_device().create_bind_group(
            Some("motion_blur_bind_group"),
            layout,
            &BindGroupEntries::sequential((
                post_process.source,
                &prepass_motion_vectors_texture.texture.default_view,
                &prepass_depth_texture.texture.default_view,
                &motion_blur_pipeline.sampler,
                settings_binding.clone(),
                globals_uniforms.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("motion_blur"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let pass_span = diagnostics.pass_span(&mut render_pass, "motion_blur");

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        pass_span.end(&mut render_pass);

        Ok(())
    }
}
