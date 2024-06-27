use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryItem;
use bevy_render::render_graph::ViewNode;

use bevy_render::render_phase::{TrackedRenderPass, ViewBinnedRenderPhases};
use bevy_render::render_resource::{CommandEncoderDescriptor, StoreOp};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext},
    render_resource::RenderPassDescriptor,
    renderer::RenderContext,
    view::ViewDepthTexture,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

use crate::prepass::ViewPrepassTextures;

use super::{AlphaMask3dDeferred, Opaque3dDeferred};

/// Render node used by the prepass.
///
/// By default, inserted before the main pass in the render graph.
#[derive(Default)]
pub struct DeferredGBufferPrepassNode;

impl ViewNode for DeferredGBufferPrepassNode {
    type ViewQuery = (
        Entity,
        &'static ExtractedCamera,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view, camera, view_depth_texture, view_prepass_textures): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let (Some(opaque_deferred_phases), Some(alpha_mask_deferred_phases)) = (
            world.get_resource::<ViewBinnedRenderPhases<Opaque3dDeferred>>(),
            world.get_resource::<ViewBinnedRenderPhases<AlphaMask3dDeferred>>(),
        ) else {
            return Ok(());
        };

        let (Some(opaque_deferred_phase), Some(alpha_mask_deferred_phase)) = (
            opaque_deferred_phases.get(&view),
            alpha_mask_deferred_phases.get(&view),
        ) else {
            return Ok(());
        };

        let mut color_attachments = vec![];
        color_attachments.push(
            view_prepass_textures
                .normal
                .as_ref()
                .map(|normals_texture| normals_texture.get_attachment()),
        );
        color_attachments.push(
            view_prepass_textures
                .motion_vectors
                .as_ref()
                .map(|motion_vectors_texture| motion_vectors_texture.get_attachment()),
        );

        // If we clear the deferred texture with LoadOp::Clear(Default::default()) we get these errors:
        // Chrome: GL_INVALID_OPERATION: No defined conversion between clear value and attachment format.
        // Firefox: WebGL warning: clearBufferu?[fi]v: This attachment is of type FLOAT, but this function is of type UINT.
        // Appears to be unsupported: https://registry.khronos.org/webgl/specs/latest/2.0/#3.7.9
        // For webgl2 we fallback to manually clearing
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        if let Some(deferred_texture) = &view_prepass_textures.deferred {
            render_context.command_encoder().clear_texture(
                &deferred_texture.texture.texture,
                &bevy_render::render_resource::ImageSubresourceRange::default(),
            );
        }

        color_attachments.push(
            view_prepass_textures
                .deferred
                .as_ref()
                .map(|deferred_texture| {
                    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                    {
                        bevy_render::render_resource::RenderPassColorAttachment {
                            view: &deferred_texture.texture.default_view,
                            resolve_target: None,
                            ops: bevy_render::render_resource::Operations {
                                load: bevy_render::render_resource::LoadOp::Load,
                                store: StoreOp::Store,
                            },
                        }
                    }
                    #[cfg(any(
                        not(feature = "webgl"),
                        not(target_arch = "wasm32"),
                        feature = "webgpu"
                    ))]
                    deferred_texture.get_attachment()
                }),
        );

        color_attachments.push(
            view_prepass_textures
                .deferred_lighting_pass_id
                .as_ref()
                .map(|deferred_lighting_pass_id| deferred_lighting_pass_id.get_attachment()),
        );

        // If all color attachments are none: clear the color attachment list so that no fragment shader is required
        if color_attachments.iter().all(Option::is_none) {
            color_attachments.clear();
        }

        let depth_stencil_attachment = Some(view_depth_texture.get_attachment(StoreOp::Store));

        let view_entity = graph.view_entity();
        render_context.add_command_buffer_generation_task(move |render_device| {
            #[cfg(feature = "trace")]
            let _deferred_span = info_span!("deferred_prepass").entered();

            // Command encoder setup
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("deferred_prepass_command_encoder"),
                });

            // Render pass setup
            let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("deferred_prepass"),
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
            if !opaque_deferred_phase.batchable_mesh_keys.is_empty()
                || !opaque_deferred_phase.unbatchable_mesh_keys.is_empty()
            {
                #[cfg(feature = "trace")]
                let _opaque_prepass_span = info_span!("opaque_deferred_prepass").entered();
                opaque_deferred_phase.render(&mut render_pass, world, view_entity);
            }

            // Alpha masked draws
            if !alpha_mask_deferred_phase.is_empty() {
                #[cfg(feature = "trace")]
                let _alpha_mask_deferred_span = info_span!("alpha_mask_deferred_prepass").entered();
                alpha_mask_deferred_phase.render(&mut render_pass, world, view_entity);
            }

            drop(render_pass);

            // After rendering to the view depth texture, copy it to the prepass depth texture
            if let Some(prepass_depth_texture) = &view_prepass_textures.depth {
                command_encoder.copy_texture_to_texture(
                    view_depth_texture.texture.as_image_copy(),
                    prepass_depth_texture.texture.texture.as_image_copy(),
                    view_prepass_textures.size,
                );
            }

            command_encoder.finish()
        });

        Ok(())
    }
}
