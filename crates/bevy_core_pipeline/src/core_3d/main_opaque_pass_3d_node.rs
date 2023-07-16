use crate::{
    clear_color::{ClearColor, ClearColorConfig},
    core_3d::{Camera3d, Opaque3d},
    prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass},
    skybox::{SkyboxBindGroup, SkyboxPipelineId},
};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_phase::{RenderPhase, TrackedRenderPass},
    render_resource::{
        CommandEncoderDescriptor, LoadOp, Operations, PipelineCache,
        RenderPassDepthStencilAttachment, RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::{ViewDepthTexture, ViewTarget, ViewUniformOffset},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

use super::{AlphaMask3d, Camera3dDepthLoadOp};

/// A [`bevy_render::render_graph::Node`] that runs the [`Opaque3d`] and [`AlphaMask3d`] [`RenderPhase`].
#[derive(Default)]
pub struct MainOpaquePass3dNode;
impl ViewNode for MainOpaquePass3dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static RenderPhase<Opaque3d>,
        &'static RenderPhase<AlphaMask3d>,
        &'static Camera3d,
        &'static ViewTarget,
        &'static ViewDepthTexture,
        Option<&'static DepthPrepass>,
        Option<&'static NormalPrepass>,
        Option<&'static MotionVectorPrepass>,
        Option<&'static SkyboxPipelineId>,
        Option<&'static SkyboxBindGroup>,
        &'static ViewUniformOffset,
    );

    fn run<'w, 'rc: 'w, 'qi: 'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'rc>,
        (
            camera,
            opaque_phase,
            alpha_mask_phase,
            camera_3d,
            target,
            depth,
            depth_prepass,
            normal_prepass,
            motion_vector_prepass,
            skybox_pipeline,
            skybox_bind_group,
            view_uniform_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Run the opaque pass, sorted front-to-back
        #[cfg(feature = "trace")]
        let main_opaque_pass_3d_span = info_span!("main_opaque_pass_3d").entered();

        let view_entity = graph.view_entity();
        let render_device = render_context.render_device().clone();

        let command_buffer_generation_task = move || {
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("main_opaque_pass_3d_command_encoder"),
                });

            // Setup render pass
            let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("main_opaque_pass_3d"),
                // NOTE: The opaque pass loads the color
                // buffer as well as writing to it.
                color_attachments: &[Some(target.get_color_attachment(Operations {
                    load: match camera_3d.clear_color {
                        ClearColorConfig::Default => {
                            LoadOp::Clear(world.resource::<ClearColor>().0.into())
                        }
                        ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                        ClearColorConfig::None => LoadOp::Load,
                    },
                    store: true,
                }))],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    // NOTE: The opaque main pass loads the depth buffer and possibly overwrites it
                    depth_ops: Some(Operations {
                        load: if depth_prepass.is_some()
                            || normal_prepass.is_some()
                            || motion_vector_prepass.is_some()
                        {
                            // if any prepass runs, it will generate a depth buffer so we should use it,
                            // even if only the normal_prepass is used.
                            Camera3dDepthLoadOp::Load
                        } else {
                            // NOTE: 0.0 is the far plane due to bevy's use of reverse-z projections.
                            camera_3d.depth_load_op.clone()
                        }
                        .into(),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            // Opaque draws
            opaque_phase.render(&mut render_pass, world, view_entity);

            // Alpha draws
            if !alpha_mask_phase.items.is_empty() {
                alpha_mask_phase.render(&mut render_pass, world, view_entity);
            }

            // Draw the skybox using a fullscreen triangle
            if let (Some(skybox_pipeline), Some(skybox_bind_group)) =
                (skybox_pipeline, skybox_bind_group)
            {
                let pipeline_cache = world.resource::<PipelineCache>();
                if let Some(pipeline) = pipeline_cache.get_render_pipeline(skybox_pipeline.0) {
                    render_pass.set_render_pipeline(pipeline);
                    render_pass.set_bind_group(
                        0,
                        &skybox_bind_group.0,
                        &[view_uniform_offset.offset],
                    );
                    render_pass.draw(0..3, 0..1);
                }
            }

            // Finish the command encoder
            drop(render_pass);
            let command_buffer = command_encoder.finish();

            #[cfg(feature = "trace")]
            drop(main_opaque_pass_3d_span);

            command_buffer
        };

        render_context.add_command_buffer_generation_task(command_buffer_generation_task);

        Ok(())
    }
}
