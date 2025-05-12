use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::experimental::occlusion_culling::OcclusionCulling;
use bevy_render::frame_graph::FrameGraph;
use bevy_render::render_graph::ViewNode;

use bevy_render::render_phase::{TrackedRenderPass, ViewBinnedRenderPhases};
use bevy_render::renderer::RenderDevice;
use bevy_render::view::{ExtractedView, NoIndirectDrawing};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext},
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
        frame_graph: &mut FrameGraph,
        view_query: QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        run_deferred_prepass(
            graph,
            frame_graph,
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
        Has<OcclusionCulling>,
        Has<NoIndirectDrawing>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        view_query: QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let (_, _, _, _, occlusion_culling, no_indirect_drawing) = view_query;
        if !occlusion_culling || no_indirect_drawing {
            return Ok(());
        }

        run_deferred_prepass(
            graph,
            frame_graph,
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
    frame_graph: &mut FrameGraph,
    (camera, extracted_view, view_depth_texture, view_prepass_textures, _, _): QueryItem<
        'w,
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

    // If we clear the deferred texture with LoadOp::Clear(Default::default()) we get these errors:
    // Chrome: GL_INVALID_OPERATION: No defined conversion between clear value and attachment format.
    // Firefox: WebGL warning: clearBufferu?[fi]v: This attachment is of type FLOAT, but this function is of type UINT.
    // Appears to be unsupported: https://registry.khronos.org/webgl/specs/latest/2.0/#3.7.9
    // For webgl2 we fallback to manually clearing
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    if !is_late {
        if let Some(deferred_texture) = &view_prepass_textures.deferred {
            let mut pass_builder = frame_graph.create_pass_builder("clear_texture");

            let deferred_texture = pass_builder.write_material(&deferred_texture.texture);

            pass_builder.create_encoder_pass_builder().clear_texture(
                &deferred_texture,
                bevy_render::render_resource::ImageSubresourceRange::default(),
            );
        }
    }

    let mut pass_builder = frame_graph.create_pass_builder(label);

    let mut color_attachments = vec![];
    color_attachments.push(
        view_prepass_textures
            .normal
            .as_ref()
            .map(|normals_texture| normals_texture.get_color_attachment(&mut pass_builder)),
    );

    color_attachments.push(view_prepass_textures.motion_vectors.as_ref().map(
        |motion_vectors_texture| motion_vectors_texture.get_color_attachment(&mut pass_builder),
    ));

    color_attachments.push(
        view_prepass_textures
            .deferred
            .as_ref()
            .map(|deferred_texture| {
                if is_late {
                    deferred_texture.get_color_attachment(&mut pass_builder)
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
                        }
                    }
                    #[cfg(any(
                        not(feature = "webgl"),
                        not(target_arch = "wasm32"),
                        feature = "webgpu"
                    ))]
                    deferred_texture.get_color_attachment(&mut pass_builder)
                }
            }),
    );

    color_attachments.push(
        view_prepass_textures
            .deferred_lighting_pass_id
            .as_ref()
            .map(|deferred_lighting_pass_id| {
                deferred_lighting_pass_id.get_color_attachment(&mut pass_builder)
            }),
    );

    if color_attachments.iter().all(Option::is_none) {
        color_attachments.clear();
    }

    let view_entity = graph.view_entity();

    {
        let mut render_pass_builder = pass_builder.create_render_pass_builder();

        render_pass_builder
            .set_pass_name(label)
            .add_color_attachments(color_attachments)
            .set_camera_viewport(camera.viewport.clone());

        let render_device = world.resource::<RenderDevice>();

        let mut tracked_render_pass = TrackedRenderPass::new(&render_device, render_pass_builder);

        // Opaque draws
        if !opaque_deferred_phase.multidrawable_meshes.is_empty()
            || !opaque_deferred_phase.batchable_meshes.is_empty()
            || !opaque_deferred_phase.unbatchable_meshes.is_empty()
        {
            if let Err(err) =
                opaque_deferred_phase.render(&mut tracked_render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the opaque deferred phase {err:?}");
            }
        }

        // Alpha masked draws
        if !alpha_mask_deferred_phase.is_empty() {
            #[cfg(feature = "trace")]
            let _alpha_mask_deferred_span = info_span!("alpha_mask_deferred_prepass").entered();
            if let Err(err) =
                alpha_mask_deferred_phase.render(&mut tracked_render_pass, world, view_entity)
            {
                error!("Error encountered while rendering the alpha mask deferred phase {err:?}");
            }
        }
    }

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

    Ok(())
}
