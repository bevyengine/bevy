use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryItem;
use bevy_render::render_graph::ViewNode;

use bevy_render::{
    camera::ExtractedCamera,
    prelude::Color,
    render_graph::{NodeRunError, RenderGraphContext},
    render_phase::RenderPhase,
    render_resource::{
        LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
        RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::ViewDepthTexture,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

use crate::core_3d::{Camera3d, Camera3dDepthLoadOp};
use crate::prepass::{MotionVectorPrepass, NormalPrepass, ViewPrepassTextures};

use super::{AlphaMask3dDeferred, Opaque3dDeferred};

/// Render node used by the prepass.
///
/// By default, inserted before the main pass in the render graph.
#[derive(Default)]
pub struct DeferredGBufferPrepassNode;

impl ViewNode for DeferredGBufferPrepassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static RenderPhase<Opaque3dDeferred>,
        &'static RenderPhase<AlphaMask3dDeferred>,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
        &'static Camera3d,
        Option<&'static NormalPrepass>,
        Option<&'static MotionVectorPrepass>,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            opaque_deferred_phase,
            alpha_mask_deferred_phase,
            view_depth_texture,
            view_prepass_textures,
            camera_3d,
            normal_prepass,
            motion_vector_prepass,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();

        let mut color_attachments = vec![];
        color_attachments.push(
            view_prepass_textures
                .normal
                .as_ref()
                .map(|view_normals_texture| RenderPassColorAttachment {
                    view: &view_normals_texture.default_view,
                    resolve_target: None,
                    ops: Operations {
                        load: if normal_prepass.is_some() {
                            // Load if the normal_prepass has already run.
                            // The prepass will have already cleared this for the current frame.
                            LoadOp::Load
                        } else {
                            LoadOp::Clear(Color::BLACK.into())
                        },
                        store: true,
                    },
                }),
        );
        color_attachments.push(view_prepass_textures.motion_vectors.as_ref().map(
            |view_motion_vectors_texture| RenderPassColorAttachment {
                view: &view_motion_vectors_texture.default_view,
                resolve_target: None,
                ops: Operations {
                    load: if motion_vector_prepass.is_some() {
                        // Load if the motion_vector_prepass has already run.
                        // The prepass will have already cleared this for the current frame.
                        LoadOp::Load
                    } else {
                        LoadOp::Clear(Color::BLACK.into())
                    },
                    store: true,
                },
            },
        ));

        // If we clear the deferred texture with LoadOp::Clear(Default::default()) we get these errors:
        // Chrome: GL_INVALID_OPERATION: No defined conversion between clear value and attachment format.
        // Firefox: WebGL warning: clearBufferu?[fi]v: This attachment is of type FLOAT, but this function is of type UINT.
        // Appears to be unsupported: https://registry.khronos.org/webgl/specs/latest/2.0/#3.7.9
        // For webgl2 we fallback to manually clearing
        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        if let Some(deferred_texture) = &view_prepass_textures.deferred {
            render_context.command_encoder().clear_texture(
                &deferred_texture.texture,
                &bevy_render::render_resource::ImageSubresourceRange::default(),
            );
        }

        color_attachments.push(
            view_prepass_textures
                .deferred
                .as_ref()
                .map(|deferred_texture| RenderPassColorAttachment {
                    view: &deferred_texture.default_view,
                    resolve_target: None,
                    ops: Operations {
                        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
                        load: LoadOp::Load,
                        #[cfg(not(all(feature = "webgl", target_arch = "wasm32")))]
                        load: LoadOp::Clear(Default::default()),
                        store: true,
                    },
                }),
        );

        color_attachments.push(
            view_prepass_textures
                .deferred_lighting_pass_id
                .as_ref()
                .map(|deferred_lighting_pass_id| RenderPassColorAttachment {
                    view: &deferred_lighting_pass_id.default_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Default::default()),
                        store: true,
                    },
                }),
        );

        if color_attachments.iter().all(Option::is_none) {
            // All attachments are none: clear the attachment list so that no fragment shader is required.
            color_attachments.clear();
        }

        {
            // Set up the pass descriptor with the depth attachment and optional color attachments.
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("deferred"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &view_depth_texture.view,
                    depth_ops: Some(Operations {
                        load: if view_depth_texture.is_first_write() {
                            // NOTE: 0.0 is the far plane due to bevy's use of reverse-z projections.
                            camera_3d.depth_load_op.clone()
                        } else {
                            Camera3dDepthLoadOp::Load
                        }
                        .into(),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            // Always run deferred pass to ensure the deferred gbuffer and deferred_lighting_pass_id are cleared.
            {
                // Run the prepass, sorted front-to-back.
                #[cfg(feature = "trace")]
                let _opaque_prepass_span = info_span!("opaque_deferred").entered();
                opaque_deferred_phase.render(&mut render_pass, world, view_entity);
            }

            if !alpha_mask_deferred_phase.items.is_empty() {
                // Run the deferred, sorted front-to-back.
                #[cfg(feature = "trace")]
                let _alpha_mask_deferred_span = info_span!("alpha_mask_deferred").entered();
                alpha_mask_deferred_phase.render(&mut render_pass, world, view_entity);
            }
        }

        if let Some(prepass_depth_texture) = &view_prepass_textures.depth {
            // Copy depth buffer to texture.
            render_context.command_encoder().copy_texture_to_texture(
                view_depth_texture.texture.as_image_copy(),
                prepass_depth_texture.texture.as_image_copy(),
                view_prepass_textures.size,
            );
        }

        Ok(())
    }
}
