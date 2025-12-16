use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::prelude::*;
use bevy_render::experimental::occlusion_culling::OcclusionCulling;

use bevy_render::view::{ExtractedView, NoIndirectDrawing};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_phase::ViewBinnedRenderPhases,
    render_resource::{RenderPassDescriptor, StoreOp},
    renderer::{RenderContext, ViewQuery},
    view::ViewDepthTexture,
};
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

use crate::prepass::ViewPrepassTextures;

use super::{AlphaMask3dDeferred, Opaque3dDeferred};

/// Type alias for the deferred prepass view query.
type DeferredPrepassViewQueryData = (
    &'static ExtractedCamera,
    &'static ExtractedView,
    &'static ViewDepthTexture,
    &'static ViewPrepassTextures,
    Option<&'static MainPassResolutionOverride>,
    Has<OcclusionCulling>,
    Has<NoIndirectDrawing>,
);

pub(crate) fn early_deferred_prepass(
    world: &World,
    view: ViewQuery<DeferredPrepassViewQueryData>,
    opaque_deferred_phases: Res<ViewBinnedRenderPhases<Opaque3dDeferred>>,
    alpha_mask_deferred_phases: Res<ViewBinnedRenderPhases<AlphaMask3dDeferred>>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();
    let (
        camera,
        extracted_view,
        view_depth_texture,
        view_prepass_textures,
        resolution_override,
        _,
        _,
    ) = view.into_inner();

    run_deferred_prepass_system(
        world,
        view_entity,
        camera,
        extracted_view,
        view_depth_texture,
        view_prepass_textures,
        resolution_override,
        false,
        &opaque_deferred_phases,
        &alpha_mask_deferred_phases,
        &mut ctx,
        "early deferred prepass",
    );
}

pub(crate) fn late_deferred_prepass(
    world: &World,
    view: ViewQuery<DeferredPrepassViewQueryData>,
    opaque_deferred_phases: Res<ViewBinnedRenderPhases<Opaque3dDeferred>>,
    alpha_mask_deferred_phases: Res<ViewBinnedRenderPhases<AlphaMask3dDeferred>>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();
    let (
        camera,
        extracted_view,
        view_depth_texture,
        view_prepass_textures,
        resolution_override,
        occlusion_culling,
        no_indirect_drawing,
    ) = view.into_inner();

    if !occlusion_culling || no_indirect_drawing {
        return;
    }

    run_deferred_prepass_system(
        world,
        view_entity,
        camera,
        extracted_view,
        view_depth_texture,
        view_prepass_textures,
        resolution_override,
        true,
        &opaque_deferred_phases,
        &alpha_mask_deferred_phases,
        &mut ctx,
        "late deferred prepass",
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "render system with many view components"
)]
fn run_deferred_prepass_system(
    world: &World,
    view_entity: Entity,
    camera: &ExtractedCamera,
    extracted_view: &ExtractedView,
    view_depth_texture: &ViewDepthTexture,
    view_prepass_textures: &ViewPrepassTextures,
    resolution_override: Option<&MainPassResolutionOverride>,
    is_late: bool,
    opaque_deferred_phases: &ViewBinnedRenderPhases<Opaque3dDeferred>,
    alpha_mask_deferred_phases: &ViewBinnedRenderPhases<AlphaMask3dDeferred>,
    ctx: &mut RenderContext,
    label: &'static str,
) {
    let (Some(opaque_deferred_phase), Some(alpha_mask_deferred_phase)) = (
        opaque_deferred_phases.get(&extracted_view.retained_view_entity),
        alpha_mask_deferred_phases.get(&extracted_view.retained_view_entity),
    ) else {
        return;
    };

    #[cfg(feature = "trace")]
    let _deferred_span = info_span!("deferred_prepass").entered();

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

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
            ctx.command_encoder().clear_texture(
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

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some(label),
        color_attachments: &color_attachments,
        depth_stencil_attachment,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_pass, label);

    if let Some(viewport) =
        Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
    {
        render_pass.set_camera_viewport(&viewport);
    }

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

    if !alpha_mask_deferred_phase.is_empty() {
        #[cfg(feature = "trace")]
        let _alpha_mask_deferred_span = info_span!("alpha_mask_deferred_prepass").entered();
        if let Err(err) = alpha_mask_deferred_phase.render(&mut render_pass, world, view_entity) {
            error!("Error encountered while rendering the alpha mask deferred phase {err:?}");
        }
    }

    pass_span.end(&mut render_pass);
    drop(render_pass);

    // After rendering to the view depth texture, copy it to the prepass depth texture
    if let Some(prepass_depth_texture) = &view_prepass_textures.depth {
        ctx.command_encoder().copy_texture_to_texture(
            view_depth_texture.texture.as_image_copy(),
            prepass_depth_texture.texture.texture.as_image_copy(),
            view_prepass_textures.size,
        );
    }
}
