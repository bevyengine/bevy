use super::ViewTransmissionTexture;
use crate::core_3d::Transmissive3d;
use bevy_camera::{Camera3d, MainPassResolutionOverride, Viewport};
use bevy_ecs::prelude::*;
use bevy_image::ToExtents;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_phase::ViewSortedRenderPhases,
    render_resource::{RenderPassDescriptor, StoreOp},
    renderer::{RenderContext, ViewQuery},
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
use core::ops::Range;
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

/// Splits a [`Range`] into at most `max_num_splits` sub-ranges without overlaps
///
/// Properly takes into account remainders of inexact divisions (by adding extra
/// elements to the initial sub-ranges as needed)
fn split_range(range: Range<usize>, max_num_splits: usize) -> impl Iterator<Item = Range<usize>> {
    let len = range.end - range.start;
    assert!(len > 0, "to be split, a range must not be empty");
    assert!(max_num_splits > 0, "max_num_splits must be at least 1");
    let num_splits = max_num_splits.min(len);
    let step = len / num_splits;
    let mut rem = len % num_splits;
    let mut start = range.start;

    (0..num_splits).map(move |_| {
        let extra = if rem > 0 {
            rem -= 1;
            1
        } else {
            0
        };
        let end = (start + step + extra).min(range.end);
        let result = start..end;
        start = end;
        result
    })
}

pub fn main_transmissive_pass_3d(
    world: &World,
    view: ViewQuery<(
        &ExtractedCamera,
        &ExtractedView,
        &Camera3d,
        &ViewTarget,
        Option<&ViewTransmissionTexture>,
        &ViewDepthTexture,
        Option<&MainPassResolutionOverride>,
    )>,
    transmissive_phases: Res<ViewSortedRenderPhases<Transmissive3d>>,
    mut ctx: RenderContext,
) {
    let view_entity = view.entity();

    let (camera, extracted_view, camera_3d, target, transmission, depth, resolution_override) =
        view.into_inner();

    let Some(transmissive_phase) = transmissive_phases.get(&extracted_view.retained_view_entity)
    else {
        return;
    };

    let Some(physical_target_size) = camera.physical_target_size else {
        return;
    };

    #[cfg(feature = "trace")]
    let _main_transmissive_pass_3d_span = info_span!("main_transmissive_pass_3d").entered();

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let render_pass_descriptor = RenderPassDescriptor {
        label: Some("main_transmissive_pass_3d"),
        color_attachments: &[Some(target.get_color_attachment())],
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
    };

    if !transmissive_phase.items.is_empty() {
        let screen_space_specular_transmission_steps =
            camera_3d.screen_space_specular_transmission_steps;
        if screen_space_specular_transmission_steps > 0 {
            let transmission =
                transmission.expect("`ViewTransmissionTexture` should exist at this point");

            // `transmissive_phase.items` are depth sorted, so we split them into N = `screen_space_specular_transmission_steps`
            // ranges, rendering them back-to-front in multiple steps, allowing multiple levels of transparency.
            for range in split_range(
                0..transmissive_phase.items.len(),
                screen_space_specular_transmission_steps,
            ) {
                // Copy the main texture to the transmission texture
                ctx.command_encoder().copy_texture_to_texture(
                    target.main_texture().as_image_copy(),
                    transmission.texture.as_image_copy(),
                    physical_target_size.to_extents(),
                );

                let mut render_pass = ctx.begin_tracked_render_pass(render_pass_descriptor.clone());
                let pass_span =
                    diagnostics.pass_span(&mut render_pass, "main_transmissive_pass_3d");

                if let Some(viewport) = camera.viewport.as_ref() {
                    render_pass.set_camera_viewport(viewport);
                }

                if let Err(err) =
                    transmissive_phase.render_range(&mut render_pass, world, view_entity, range)
                {
                    error!("Error encountered while rendering the transmissive phase {err:?}");
                }

                pass_span.end(&mut render_pass);
            }
        } else {
            let mut render_pass = ctx.begin_tracked_render_pass(render_pass_descriptor);
            let pass_span = diagnostics.pass_span(&mut render_pass, "main_transmissive_pass_3d");

            if let Some(viewport) =
                Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
            {
                render_pass.set_camera_viewport(&viewport);
            }

            if let Err(err) = transmissive_phase.render(&mut render_pass, world, view_entity) {
                error!("Error encountered while rendering the transmissive phase {err:?}");
            }

            pass_span.end(&mut render_pass);
        }
    }
}
