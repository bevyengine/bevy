use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryItem;
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{BinnedRenderPhase, TrackedRenderPass},
    render_resource::{CommandEncoderDescriptor, RenderPassDescriptor, StoreOp},
    renderer::RenderContext,
    view::ViewDepthTexture,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

use super::{AlphaMask3dPrepass, DeferredPrepass, Opaque3dPrepass, ViewPrepassTextures};

/// Render node used by the prepass.
///
/// By default, inserted before the main pass in the render graph.
#[derive(Default)]
pub struct PrepassNode;

impl ViewNode for PrepassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static BinnedRenderPhase<Opaque3dPrepass>,
        &'static BinnedRenderPhase<AlphaMask3dPrepass>,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
        Option<&'static DeferredPrepass>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            camera,
            opaque_prepass_phase,
            alpha_mask_prepass_phase,
            view_depth_texture,
            view_prepass_textures,
            deferred_prepass,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
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
