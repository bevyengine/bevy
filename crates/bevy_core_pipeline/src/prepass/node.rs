use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    experimental::occlusion_culling::OcclusionCulling,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewBinnedRenderPhases},
    render_resource::{CommandEncoderDescriptor, PipelineCache, RenderPassDescriptor, StoreOp},
    renderer::RenderContext,
    view::{ExtractedView, NoIndirectDrawing, ViewDepthTexture, ViewUniformOffset},
};
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

use crate::skybox::prepass::{RenderSkyboxPrepassPipeline, SkyboxPrepassBindGroup};

use super::{
    AlphaMask3dPrepass, DeferredPrepass, Opaque3dPrepass, PreviousViewUniformOffset,
    ViewPrepassTextures,
};

/// The phase of the prepass that draws meshes that were visible last frame.
///
/// If occlusion culling isn't in use, this prepass simply draws all meshes.
///
/// Like all prepass nodes, this is inserted before the main pass in the render
/// graph.
#[derive(Default)]
pub struct EarlyPrepassNode;

impl ViewNode for EarlyPrepassNode {
    type ViewQuery = <LatePrepassNode as ViewNode>::ViewQuery;

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        view_query: QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        run_prepass(graph, render_context, view_query, world, "early prepass")
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
pub struct LatePrepassNode;

impl ViewNode for LatePrepassNode {
    type ViewQuery = (
        (
            &'static ExtractedCamera,
            &'static ExtractedView,
            &'static ViewDepthTexture,
            &'static ViewPrepassTextures,
            &'static ViewUniformOffset,
        ),
        (
            Option<&'static DeferredPrepass>,
            Option<&'static RenderSkyboxPrepassPipeline>,
            Option<&'static SkyboxPrepassBindGroup>,
            Option<&'static PreviousViewUniformOffset>,
            Option<&'static MainPassResolutionOverride>,
        ),
        (
            Has<OcclusionCulling>,
            Has<NoIndirectDrawing>,
            Has<DeferredPrepass>,
        ),
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        query: QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // We only need a late prepass if we have occlusion culling and indirect
        // drawing.
        let (_, _, (occlusion_culling, no_indirect_drawing, _)) = query;
        if !occlusion_culling || no_indirect_drawing {
            return Ok(());
        }

        run_prepass(graph, render_context, query, world, "late prepass")
    }
}

/// Runs a prepass that draws all meshes to the depth buffer, and possibly
/// normal and motion vector buffers as well.
///
/// If occlusion culling isn't in use, and a prepass is enabled, then there's
/// only one prepass. If occlusion culling is in use, then any prepass is split
/// into two: an *early* prepass and a *late* prepass. The early prepass draws
/// what was visible last frame, and the last prepass performs occlusion culling
/// against a conservative hierarchical Z buffer before drawing unoccluded
/// meshes.
fn run_prepass<'w>(
    graph: &mut RenderGraphContext,
    render_context: &mut RenderContext<'w>,
    (
        (camera, extracted_view, view_depth_texture, view_prepass_textures, view_uniform_offset),
        (
            deferred_prepass,
            skybox_prepass_pipeline,
            skybox_prepass_bind_group,
            view_prev_uniform_offset,
            resolution_override,
        ),
        (_, _, has_deferred),
    ): QueryItem<'w, '_, <LatePrepassNode as ViewNode>::ViewQuery>,
    world: &'w World,
    label: &'static str,
) -> Result<(), NodeRunError> {
    // If we're using deferred rendering, there will be a deferred prepass
    // instead of this one. Just bail out so we don't have to bother looking at
    // the empty bins.
    if has_deferred {
        return Ok(());
    }

    let (Some(opaque_prepass_phases), Some(alpha_mask_prepass_phases)) = (
        world.get_resource::<ViewBinnedRenderPhases<Opaque3dPrepass>>(),
        world.get_resource::<ViewBinnedRenderPhases<AlphaMask3dPrepass>>(),
    ) else {
        return Ok(());
    };

    let (Some(opaque_prepass_phase), Some(alpha_mask_prepass_phase)) = (
        opaque_prepass_phases.get(&extracted_view.retained_view_entity),
        alpha_mask_prepass_phases.get(&extracted_view.retained_view_entity),
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
        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("prepass_command_encoder"),
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
        let pass_span = diagnostics.pass_span(&mut render_pass, label);

        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_pass.set_camera_viewport(&viewport);
        }

        // Opaque draws
        if !opaque_prepass_phase.is_empty() {
            #[cfg(feature = "trace")]
            let _opaque_prepass_span = info_span!("opaque_prepass").entered();
            if let Err(err) = opaque_prepass_phase.render(&mut render_pass, world, view_entity) {
                error!("Error encountered while rendering the opaque prepass phase {err:?}");
            }
        }

        // Alpha masked draws
        if !alpha_mask_prepass_phase.is_empty() {
            #[cfg(feature = "trace")]
            let _alpha_mask_prepass_span = info_span!("alpha_mask_prepass").entered();
            if let Err(err) = alpha_mask_prepass_phase.render(&mut render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the alpha mask prepass phase {err:?}");
            }
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
            if let Some(pipeline) = pipeline_cache.get_render_pipeline(skybox_prepass_pipeline.0) {
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

        // After rendering to the view depth texture, copy it to the prepass depth texture if deferred isn't going to
        if deferred_prepass.is_none()
            && let Some(prepass_depth_texture) = &view_prepass_textures.depth
        {
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
