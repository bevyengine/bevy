use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    extract_component::ComponentUniforms,
    globals::GlobalsBuffer,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        BindGroupDescriptor, BindGroupEntry, BindingResource, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::{Msaa, ViewTarget},
};

use crate::prepass::ViewPrepassTextures;

use super::{
    pipeline::{MotionBlurPipeline, MotionBlurPipelineId},
    MotionBlur,
};

#[derive(Default)]
pub struct MotionBlurNode;

impl ViewNode for MotionBlurNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static MotionBlurPipelineId,
        &'static ViewPrepassTextures,
    );
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, pipeline_id, prepass_textures): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let motion_blur_pipeline = world.resource::<MotionBlurPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let settings_uniforms = world.resource::<ComponentUniforms<MotionBlur>>();
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

        let post_process = view_target.post_process_write();

        let msaa = world.resource::<Msaa>();
        let layout = if msaa.samples() == 1 {
            &motion_blur_pipeline.layout
        } else {
            &motion_blur_pipeline.layout_msaa
        };

        let bind_group = render_context
            .render_device()
            .create_bind_group(&BindGroupDescriptor {
                label: Some("motion_blur_bind_group"),
                layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(post_process.source),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(
                            &prepass_motion_vectors_texture.default_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(&prepass_depth_texture.default_view),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::Sampler(&motion_blur_pipeline.sampler),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: settings_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: globals_uniforms.clone(),
                    },
                ],
            });

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("motion_blur_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}
