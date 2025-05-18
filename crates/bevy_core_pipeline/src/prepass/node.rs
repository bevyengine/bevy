use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    experimental::occlusion_culling::OcclusionCulling,
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewBinnedRenderPhases},
    render_resource::{PipelineCache, StoreOp},
    renderer::RenderDevice,
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
        frame_graph: &mut FrameGraph,
        view_query: QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        run_prepass(graph, frame_graph, view_query, world, "early prepass")
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
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static ViewDepthTexture,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
        Option<&'static DeferredPrepass>,
        Option<&'static RenderSkyboxPrepassPipeline>,
        Option<&'static SkyboxPrepassBindGroup>,
        Option<&'static PreviousViewUniformOffset>,
        Has<OcclusionCulling>,
        Has<NoIndirectDrawing>,
        Has<DeferredPrepass>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        query: QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // We only need a late prepass if we have occlusion culling and indirect
        // drawing.
        let (_, _, _, _, _, _, _, _, _, occlusion_culling, no_indirect_drawing, _) = query;
        if !occlusion_culling || no_indirect_drawing {
            return Ok(());
        }

        run_prepass(graph, frame_graph, query, world, "late prepass")
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
    frame_graph: &mut FrameGraph,
    (
        camera,
        extracted_view,
        view_depth_texture,
        view_prepass_textures,
        view_uniform_offset,
        deferred_prepass,
        skybox_prepass_pipeline,
        skybox_prepass_bind_group,
        view_prev_uniform_offset,
        _,
        _,
        has_deferred,
    ): QueryItem<'w, <LatePrepassNode as ViewNode>::ViewQuery>,
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

    let mut pass_builder = frame_graph.create_pass_builder(label);

    let mut color_attachments = vec![
        view_prepass_textures
            .normal
            .as_ref()
            .map(|normals_texture| normals_texture.get_color_attachment(&mut pass_builder)),
        view_prepass_textures
            .motion_vectors
            .as_ref()
            .map(|motion_vectors_texture| {
                motion_vectors_texture.get_color_attachment(&mut pass_builder)
            }),
        // Use None in place of deferred attachments
        None,
        None,
    ];

    // If all color attachments are none: clear the color attachment list so that no fragment shader is required
    if color_attachments.iter().all(Option::is_none) {
        color_attachments.clear();
    }

    let depth_stencil_attachment =
        view_depth_texture.get_depth_stencil_attachment(&mut pass_builder, StoreOp::Store);

    let view_entity = graph.view_entity();

    {
        let mut render_pass_builder = pass_builder.create_render_pass_builder();

        render_pass_builder
            .set_pass_name(label)
            .add_color_attachments(color_attachments)
            .set_depth_stencil_attachment(depth_stencil_attachment)
            .set_camera_viewport(camera.viewport.clone());

        let render_device = world.resource::<RenderDevice>();

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
            if let Some(_) = pipeline_cache.get_render_pipeline(skybox_prepass_pipeline.0) {
                render_pass_builder
                    .set_render_pipeline(skybox_prepass_pipeline.0)
                    .set_raw_bind_group(
                        0,
                        Some(&skybox_prepass_bind_group.0),
                        &[view_uniform_offset.offset, view_prev_uniform_offset.offset],
                    )
                    .draw(0..3, 0..1);
            }
        }

        let mut tracked_render_pass = TrackedRenderPass::new(&render_device, render_pass_builder);

        // Opaque draws
        if !opaque_prepass_phase.is_empty() {
            if let Err(err) =
                opaque_prepass_phase.render(&mut tracked_render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the opaque prepass phase {err:?}");
            }
        }

        // Alpha masked draws
        if !alpha_mask_prepass_phase.is_empty() {
            if let Err(err) =
                alpha_mask_prepass_phase.render(&mut tracked_render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the alpha mask prepass phase {err:?}");
            }
        }
    }

    // After rendering to the view depth texture, copy it to the prepass depth texture if deferred isn't going to
    if deferred_prepass.is_none() {
        if let Some(prepass_depth_texture) = &view_prepass_textures.depth {
            let source = view_depth_texture
                .texture
                .get_image_copy_read(&mut pass_builder);
            let destination = prepass_depth_texture
                .texture
                .get_image_copy_write(&mut pass_builder);

            pass_builder
                .create_encoder_pass_builder()
                .copy_texture_to_texture(source, destination, view_prepass_textures.size);
        }
    }

    Ok(())
}
