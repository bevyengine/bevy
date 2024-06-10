use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryItem;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewBinnedRenderPhases},
    render_resource::{CommandEncoderDescriptor, PipelineCache, RenderPassDescriptor, StoreOp},
    renderer::RenderContext,
    view::{ViewDepthTexture, ViewUniformOffset},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

use crate::skybox::prepass::{RenderSkyboxPrepassPipeline, SkyboxPrepassBindGroup};

use super::{
    AlphaMask3dPrepass, DeferredPrepass, Opaque3dPrepass, PreviousViewUniformOffset,
    ViewPrepassTextures,
};

/// Render node used by the prepass.
///
/// By default, inserted before the main pass in the render graph.
#[derive(Default)]
pub struct PrepassNode;

impl ViewNode for PrepassNode {
    type ViewQuery = (
        Entity,
        &'static ExtractedCamera,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
        Option<&'static DeferredPrepass>,
        Option<&'static RenderSkyboxPrepassPipeline>,
        Option<&'static SkyboxPrepassBindGroup>,
        Option<&'static PreviousViewUniformOffset>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            view,
            camera,
            view_depth_texture,
            view_prepass_textures,
            view_uniform_offset,
            deferred_prepass,
            skybox_prepass_pipeline,
            skybox_prepass_bind_group,
            view_prev_uniform_offset,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let (Some(opaque_prepass_phases), Some(alpha_mask_prepass_phases)) = (
            world.get_resource::<ViewBinnedRenderPhases<Opaque3dPrepass>>(),
            world.get_resource::<ViewBinnedRenderPhases<AlphaMask3dPrepass>>(),
        ) else {
            return Ok(());
        };

        let (Some(opaque_prepass_phase), Some(alpha_mask_prepass_phase)) = (
            opaque_prepass_phases.get(&view),
            alpha_mask_prepass_phases.get(&view),
        ) else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let mut color_attachments = vec![
            view_prepass_textures
                .normal
                .as_ref()
                .map(|normals_texture| normals_texture.get_attachment()),
            view_prepass_textures
                .motion_vectors
                .as_ref()
                .map(|motion_vectors_texture| motion_vectors_texture.get_attachment()),
            // Use None in place of deferred attachments
            None,
            None,
        ];

        // If all color attachments are none: clear the color attachment list so that no fragment shader is required
        if color_attachments.iter().all(Option::is_none) {
            color_attachments.clear();
        }

        let depth_stencil_attachment = Some(view_depth_texture.get_attachment(StoreOp::Store));

        let view_entity = graph.view_entity();
        render_context.add_command_buffer_generation_task(move |render_device| {
            #[cfg(feature = "trace")]
            let _prepass_span = info_span!("prepass").entered();

            // Command encoder setup
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("prepass_command_encoder"),
                });

            // Render pass setup
            let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("prepass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);
            let pass_span = diagnostics.pass_span(&mut render_pass, "prepass");

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            // Opaque draws
            if !opaque_prepass_phase.batchable_keys.is_empty()
                || !opaque_prepass_phase.unbatchable_keys.is_empty()
            {
                #[cfg(feature = "trace")]
                let _opaque_prepass_span = info_span!("opaque_prepass").entered();
                opaque_prepass_phase.render(&mut render_pass, world, view_entity);
            }

            // Alpha masked draws
            if !alpha_mask_prepass_phase.is_empty() {
                #[cfg(feature = "trace")]
                let _alpha_mask_prepass_span = info_span!("alpha_mask_prepass").entered();
                alpha_mask_prepass_phase.render(&mut render_pass, world, view_entity);
            }

            // Skybox draw using a fullscreen triangle
            if let (
                Some(skybox_prepass_pipeline),
                Some(skybox_prepass_bind_group),
                Some(view_prev_uniform_offset),
            ) = (
                skybox_prepass_pipeline,
                skybox_prepass_bind_group,
                view_prev_uniform_offset,
            ) {
                let pipeline_cache = world.resource::<PipelineCache>();
                if let Some(pipeline) =
                    pipeline_cache.get_render_pipeline(skybox_prepass_pipeline.0)
                {
                    render_pass.set_render_pipeline(pipeline);
                    render_pass.set_bind_group(
                        0,
                        &skybox_prepass_bind_group.0,
                        &[view_uniform_offset.offset, view_prev_uniform_offset.offset],
                    );
                    render_pass.draw(0..3, 0..1);
                }
            }

            pass_span.end(&mut render_pass);
            drop(render_pass);

            // Copy prepass depth to the main depth texture if deferred isn't going to
            if deferred_prepass.is_none() {
                if let Some(prepass_depth_texture) = &view_prepass_textures.depth {
                    command_encoder.copy_texture_to_texture(
                        view_depth_texture.texture.as_image_copy(),
                        prepass_depth_texture.texture.texture.as_image_copy(),
                        view_prepass_textures.size,
                    );
                }
            }

            command_encoder.finish()
        });

        Ok(())
    }
}
