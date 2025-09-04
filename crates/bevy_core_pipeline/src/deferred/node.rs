use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::experimental::occlusion_culling::OcclusionCulling;
use bevy_render::render_graph::ViewNode;

use bevy_render::view::{ExtractedView, NoIndirectDrawing};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext},
    render_phase::{TrackedRenderPass, ViewBinnedRenderPhases},
    render_resource::{CommandEncoderDescriptor, RenderPassDescriptor, StoreOp},
    renderer::RenderContext,
    view::ViewDepthTexture,
};
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

use crate::prepass::ViewPrepassTextures;

use super::{AlphaMask3dDeferred, Opaque3dDeferred};

/// The phase of the deferred prepass that draws meshes that were visible last
/// frame.
///
/// If occlusion culling isn't in use, this prepass simply draws all meshes.
///
/// Like all prepass nodes, this is inserted before the main pass in the render
/// graph.
#[derive(Default)]
pub struct EarlyDeferredGBufferPrepassNode;

impl ViewNode for EarlyDeferredGBufferPrepassNode {
    type ViewQuery = <LateDeferredGBufferPrepassNode as ViewNode>::ViewQuery;

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        view_query: QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        run_deferred_prepass(
            graph,
            render_context,
            view_query,
            false,
            world,
            "early deferred prepass",
        )
    }
}

/// The phase of the prepass that runs after occlusion culling against the
/// meshes that were visible last frame.
///
/// If occlusion culling isn't in use, this is a no-op.
///
/// Like all prepass nodes, this is inserted before the main pass in the render
/// graph.
#[derive(Default)]
pub struct LateDeferredGBufferPrepassNode;

impl ViewNode for LateDeferredGBufferPrepassNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
        Option<&'static MainPassResolutionOverride>,
        Has<OcclusionCulling>,
        Has<NoIndirectDrawing>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        view_query: QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let (.., occlusion_culling, no_indirect_drawing) = view_query;
        if !occlusion_culling || no_indirect_drawing {
            return Ok(());
        }

        run_deferred_prepass(
            graph,
            render_context,
            view_query,
            true,
            world,
            "late deferred prepass",
        )
    }
}

/// Runs the deferred prepass that draws all meshes to the depth buffer and
/// G-buffers.
///
/// If occlusion culling isn't in use, and a prepass is enabled, then there's
/// only one prepass. If occlusion culling is in use, then any prepass is split
/// into two: an *early* prepass and a *late* prepass. The early prepass draws
/// what was visible last frame, and the last prepass performs occlusion culling
/// against a conservative hierarchical Z buffer before drawing unoccluded
/// meshes.
fn run_deferred_prepass<'w>(
    graph: &mut RenderGraphContext,
    render_context: &mut RenderContext<'w>,
    (camera, extracted_view, view_depth_texture, view_prepass_textures, resolution_override, _, _): QueryItem<
        'w,
        '_,
        <LateDeferredGBufferPrepassNode as ViewNode>::ViewQuery,
    >,
    is_late: bool,
    world: &'w World,
    label: &'static str,
) -> Result<(), NodeRunError> {
    let (Some(opaque_deferred_phases), Some(alpha_mask_deferred_phases)) = (
        world.get_resource::<ViewBinnedRenderPhases<Opaque3dDeferred>>(),
        world.get_resource::<ViewBinnedRenderPhases<AlphaMask3dDeferred>>(),
    ) else {
        return Ok(());
    };

    let (Some(opaque_deferred_phase), Some(alpha_mask_deferred_phase)) = (
        opaque_deferred_phases.get(&extracted_view.retained_view_entity),
        alpha_mask_deferred_phases.get(&extracted_view.retained_view_entity),
    ) else {
        return Ok(());
    };

    let diagnostic = render_context.diagnostic_recorder();

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
    if !is_late {
        if let Some(deferred_texture) = &view_prepass_textures.deferred {
            render_context.command_encoder().clear_texture(
                &deferred_texture.texture.texture,
                &bevy_render::render_resource::ImageSubresourceRange::default(),
            );
        }
    }

    color_attachments.push(
        view_prepass_textures
            .deferred
            .as_ref()
            .map(|deferred_texture| {
                if is_late {
                    deferred_texture.get_attachment()
                } else {
                    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                    {
                        bevy_render::render_resource::RenderPassColorAttachment {
                            view: &deferred_texture.texture.default_view,
                            resolve_target: None,
                            ops: bevy_render::render_resource::Operations {
                                load: bevy_render::render_resource::LoadOp::Load,
                                store: StoreOp::Store,
                            },
                            depth_slice: None,
                        }
                    }
                    #[cfg(any(
                        not(feature = "webgl"),
                        not(target_arch = "wasm32"),
                        feature = "webgpu"
                    ))]
                    deferred_texture.get_attachment()
                }
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
        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("deferred_prepass_command_encoder"),
        });

        // Render pass setup
        let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(label),
            color_attachments: &color_attachments,
            depth_stencil_attachment,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);
        let pass_span = diagnostic.pass_span(&mut render_pass, label);
        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_pass.set_camera_viewport(&viewport);
        }

        // Opaque draws
        if !opaque_deferred_phase.multidrawable_meshes.is_empty()
            || !opaque_deferred_phase.batchable_meshes.is_empty()
            || !opaque_deferred_phase.unbatchable_meshes.is_empty()
        {
            #[cfg(feature = "trace")]
            let _opaque_prepass_span = info_span!("opaque_deferred_prepass").entered();
            if let Err(err) = opaque_deferred_phase.render(&mut render_pass, world, view_entity) {
                error!("Error encountered while rendering the opaque deferred phase {err:?}");
            }
        }

        // Alpha masked draws
        if !alpha_mask_deferred_phase.is_empty() {
            #[cfg(feature = "trace")]
            let _alpha_mask_deferred_span = info_span!("alpha_mask_deferred_prepass").entered();
            if let Err(err) = alpha_mask_deferred_phase.render(&mut render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the alpha mask deferred phase {err:?}");
            }
        }

        pass_span.end(&mut render_pass);
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
