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

use super::{AlphaMask3dPrepass, Opaque3dPrepass, ViewPrepassTextures};

/// Render node used by the prepass.
///
/// By default, inserted before the main pass in the render graph.
#[derive(Default)]
pub struct PrepassNode;

impl ViewNode for PrepassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static RenderPhase<Opaque3dPrepass>,
        &'static RenderPhase<AlphaMask3dPrepass>,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            opaque_prepass_phase,
            alpha_mask_prepass_phase,
            view_depth_texture,
            view_prepass_textures,
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
                        load: LoadOp::Clear(Color::BLACK.into()),
                        store: true,
                    },
                }),
        );
        color_attachments.push(view_prepass_textures.motion_vectors.as_ref().map(
            |view_motion_vectors_texture| RenderPassColorAttachment {
                view: &view_motion_vectors_texture.default_view,
                resolve_target: None,
                ops: Operations {
                    // Red and Green channels are X and Y components of the motion vectors
                    // Blue channel doesn't matter, but set to 0.0 for possible faster clear
                    // https://gpuopen.com/performance/#clears
                    load: LoadOp::Clear(Color::rgb_linear(0.0, 0.0, 0.0).into()),
                    store: true,
                },
            },
        ));
        if color_attachments.iter().all(Option::is_none) {
            // all attachments are none: clear the attachment list so that no fragment shader is required
            color_attachments.clear();
        }

        {
            // Set up the pass descriptor with the depth attachment and optional color attachments
            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("prepass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &view_depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            // Always run opaque pass to ensure screen is cleared
            {
                // Run the prepass, sorted front-to-back
                #[cfg(feature = "trace")]
                let _opaque_prepass_span = info_span!("opaque_prepass").entered();
                opaque_prepass_phase.render(&mut render_pass, world, view_entity);
            }

            if !alpha_mask_prepass_phase.items.is_empty() {
                // Run the prepass, sorted front-to-back
                #[cfg(feature = "trace")]
                let _alpha_mask_prepass_span = info_span!("alpha_mask_prepass").entered();
                alpha_mask_prepass_phase.render(&mut render_pass, world, view_entity);
            }
        }

        if let Some(prepass_depth_texture) = &view_prepass_textures.depth {
            // Copy depth buffer to texture
            render_context.command_encoder().copy_texture_to_texture(
                view_depth_texture.texture.as_image_copy(),
                prepass_depth_texture.texture.as_image_copy(),
                view_prepass_textures.size,
            );
        }

        Ok(())
    }
}
