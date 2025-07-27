use super::{Camera3d, ViewTransmissionTexture};
use crate::core_3d::Transmissive3d;
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_image::ToExtents;
use bevy_render::{
    camera::{ExtractedCamera, MainPassResolutionOverride},
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewSortedRenderPhases},
    render_resource::{CommandEncoderDescriptor, RenderPassDescriptor, StoreOp},
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};
use core::ops::Range;
use tracing::error;
#[cfg(feature = "trace")]
use tracing::info_span;

/// A [`bevy_render::render_graph::Node`] that runs the [`Transmissive3d`]
/// [`ViewSortedRenderPhases`].
#[derive(Default)]
pub struct MainTransmissivePass3dNode;

impl ViewNode for MainTransmissivePass3dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ExtractedView,
        &'static Camera3d,
        &'static ViewTarget,
        Option<&'static ViewTransmissionTexture>,
        &'static ViewDepthTexture,
        Option<&'static MainPassResolutionOverride>,
    );

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, view, camera_3d, target, transmission, depth, resolution_override): QueryItem<
            'w,
            '_,
            Self::ViewQuery,
        >,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();

        let Some(transmissive_phases) =
            world.get_resource::<ViewSortedRenderPhases<Transmissive3d>>()
        else {
            return Ok(());
        };

        let Some(transmissive_phase) = transmissive_phases.get(&view.retained_view_entity) else {
            return Ok(());
        };

        if !transmissive_phase.items.is_empty() {
            let physical_target_size = camera.physical_target_size.unwrap();
            let screen_space_specular_transmission_steps =
                camera_3d.screen_space_specular_transmission_steps;

            let color_attachments = [Some(target.get_color_attachment())];
            let depth_stencil_attachment = Some(depth.get_attachment(StoreOp::Store));

            render_context.add_command_buffer_generation_task(move |render_device| {
                // Run the transmissive pass, sorted back-to-front
                #[cfg(feature = "trace")]
                let _main_transmissive_pass_3d_span = info_span!("main_transmissive_pass_3d").entered();

                // Command encoder setup
                let mut command_encoder =
                    render_device.create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("main_transmissive_pass_3d_command_encoder"),
                    });

                if screen_space_specular_transmission_steps > 0 {
                    let transmission =
                        transmission.expect("`ViewTransmissionTexture` should exist at this point");

                    // `transmissive_phase.items` are depth sorted, so we split them into N = `screen_space_specular_transmission_steps`
                    // ranges, rendering them back-to-front in multiple steps, allowing multiple levels of transparency.
                    //
                    // Note: For the sake of simplicity, we currently split items evenly among steps. In the future, we
                    // might want to use a more sophisticated heuristic (e.g. based on view bounds, or with an exponential
                    // falloff so that nearby objects have more levels of transparency available to them)
                    for range in split_range(
                        0..transmissive_phase.items.len(),
                        screen_space_specular_transmission_steps,
                    ) {
                        // Copy the main texture to the transmission texture, allowing to use the color output of the
                        // previous step (or of the `Opaque3d` phase, for the first step) as a transmissive color input
                        command_encoder.copy_texture_to_texture(
                            target.main_texture().as_image_copy(),
                            transmission.texture.as_image_copy(),
                            physical_target_size.to_extents(),
                        );

                        let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                            label: Some("main_transmissive_pass_3d"),
                            color_attachments: &color_attachments,
                            depth_stencil_attachment: depth_stencil_attachment.clone(),
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                        let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);

                        if let Some(viewport) = camera.viewport.as_ref() {
                            render_pass.set_camera_viewport(viewport);
                        }

                        // render items in range
                        if let Err(err) =
                            transmissive_phase.render_range(&mut render_pass, world, view_entity, range)
                        {
                            error!("Error encountered while rendering the transmissive phase {err:?}");
                        }
                    }
                } else {
                    let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("main_transmissive_pass_3d"),
                        color_attachments: &color_attachments,
                        depth_stencil_attachment,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);

                    if let Some(viewport) = camera.viewport.as_ref() {
                        render_pass.set_camera_viewport(&viewport.with_override(resolution_override));
                    }

                    if let Err(err) = transmissive_phase.render(&mut render_pass, world, view_entity) {
                        error!("Error encountered while rendering the transmissive phase {err:?}");
                    }
                }

                command_encoder.finish()
            });
        }

        Ok(())
    }
}

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
