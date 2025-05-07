use super::{Camera3d, Transmissive3d, ViewTransmissionTexture};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::{
        command_encoder::CommandEncoderPass, command_encoder_context::CommandEncoderCommandBuilder,
        FrameGraph, RenderPassBuilder,
    },
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{TrackedRenderPass, ViewSortedRenderPhases},
    render_resource::{Extent3d, StoreOp},
    renderer::RenderDevice,
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
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (camera, view, camera_3d, target, transmission, depth): QueryItem<Self::ViewQuery>,
        world: &World,
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

        let physical_target_size = camera.physical_target_size.unwrap();

        #[cfg(feature = "trace")]
        let _main_transmissive_pass_3d_span = info_span!("main_transmissive_pass_3d").entered();

        if !transmissive_phase.items.is_empty() {
            let render_device = world.resource::<RenderDevice>();

            let screen_space_specular_transmission_steps =
                camera_3d.screen_space_specular_transmission_steps;
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

                    {
                        let mut pass_node_builder = frame_graph
                            .create_pass_node_bulder("main_transmissive_command_encoder_3d");

                        let source = target.get_main_texture_image_copy(&mut pass_node_builder)?;
                        let destination = transmission.get_image_copy(&mut pass_node_builder)?;

                        let mut pass = CommandEncoderPass::default();

                        pass.copy_texture_to_texture(
                            source,
                            destination,
                            Extent3d {
                                width: physical_target_size.x,
                                height: physical_target_size.y,
                                depth_or_array_layers: 1,
                            },
                        );

                        pass_node_builder.set_pass(pass);
                    }

                    let mut pass_node_builder =
                        frame_graph.create_pass_node_bulder("main_transmissive_pass_3d");

                    let color_attachment = target.get_color_attachment(&mut pass_node_builder)?;
                    let depth_stencil_attachment = depth
                        .get_depth_stencil_attachment(&mut pass_node_builder, StoreOp::Store)?;

                    let mut builder = RenderPassBuilder::new(pass_node_builder);

                    builder
                        .add_color_attachment(color_attachment)
                        .set_depth_stencil_attachment(depth_stencil_attachment)
                        .set_camera_viewport(camera.viewport.clone());

                    let mut tracked_render_pass = TrackedRenderPass::new(&render_device, builder);

                    // render items in range
                    if let Err(err) = transmissive_phase.render_range(
                        &mut tracked_render_pass,
                        world,
                        view_entity,
                        range,
                    ) {
                        error!("Error encountered while rendering the transmissive phase {err:?}");
                    }
                }
            } else {
                let mut pass_node_builder =
                    frame_graph.create_pass_node_bulder("main_transmissive_pass_3d");

                let color_attachment = target.get_color_attachment(&mut pass_node_builder)?;
                let depth_stencil_attachment =
                    depth.get_depth_stencil_attachment(&mut pass_node_builder, StoreOp::Store)?;

                let mut builder = RenderPassBuilder::new(pass_node_builder);

                builder
                    .add_color_attachment(color_attachment)
                    .set_depth_stencil_attachment(depth_stencil_attachment)
                    .set_camera_viewport(camera.viewport.clone());

                let mut tracked_render_pass = TrackedRenderPass::new(&render_device, builder);

                if let Err(err) =
                    transmissive_phase.render(&mut tracked_render_pass, world, view_entity)
                {
                    error!("Error encountered while rendering the transmissive phase {err:?}");
                }
            }
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
