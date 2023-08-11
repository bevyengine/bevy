use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryItem;
use bevy_render::render_graph::ViewNode;
use bevy_render::view::Msaa;
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
use crate::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass, ViewPrepassTextures};

use super::{AlphaMask3dDeferred, Opaque3dDeferred};

/// Render node used by the prepass.
///
/// By default, inserted before the main pass in the render graph.
#[derive(Default)]
pub struct DeferredNode;

impl ViewNode for DeferredNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static RenderPhase<Opaque3dDeferred>,
        &'static RenderPhase<AlphaMask3dDeferred>,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
        &'static Camera3d,
        Option<&'static DepthPrepass>,
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
            depth_prepass,
            normal_prepass,
            motion_vector_prepass,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();

        if let Some(msaa) = world.get_resource::<Msaa>() {
            match msaa {
                Msaa::Off => (),
                _ => panic!("MSAA not supported when using deferred rendering."),
            }
        }

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
                            LoadOp::Load // load if the normal_prepass has run
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
                        LoadOp::Load // load if the motion_vector_prepass has run
                    } else {
                        LoadOp::Clear(Color::BLACK.into())
                    },
                    store: true,
                },
            },
        ));
        color_attachments.push(
            view_prepass_textures
                .deferred
                .as_ref()
                .map(|deferred_texture| RenderPassColorAttachment {
                    view: &deferred_texture.default_view,
                    resolve_target: None,
                    ops: Operations {
                        // If we clear with LoadOp::Clear(Default::default()) we get these errors:
                        // Chrome: GL_INVALID_OPERATION: No defined conversion between clear value and attachment format.
                        // Firefox: WebGL warning: clearBufferu?[fi]v: This attachment is of type FLOAT, but this function is of type UINT.
                        // It should by ok to not clear since we are using the stencil buffer to identify which
                        // pixels the deferred lighting passes should run on. And the depth buffer is cleared.
                        load: LoadOp::Load,
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
                        load: if depth_prepass.is_some()
                            || normal_prepass.is_some()
                            || motion_vector_prepass.is_some()
                        {
                            // If any prepass runs, it will generate a depth buffer so we should use it.
                            Camera3dDepthLoadOp::Load
                        } else {
                            // NOTE: 0.0 is the far plane due to bevy's use of reverse-z projections.
                            camera_3d.depth_load_op.clone()
                        }
                        .into(),
                        store: true,
                    }),
                    stencil_ops: Some(Operations {
                        load: if depth_prepass.is_some() {
                            LoadOp::Load // Load if the depth_prepass has run.
                        } else {
                            LoadOp::Clear(0)
                        },
                        store: true,
                    }),
                }),
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            // Always run opaque pass to ensure screen is cleared.
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
            // Warnings in WebGL:
            // Chrome: [.WebGL-0000002C0AD19500] GL_INVALID_FRAMEBUFFER_OPERATION: Framebuffer is incomplete: Depth stencil texture in color attachment.
            // Firefox: WebGL warning: copyTexSubImage: Framebuffer not complete. (status: 0x8cd6) COLOR_ATTACHMENT0: Attachment's format is missing required color/depth/stencil bits.

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
