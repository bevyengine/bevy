use super::{
    gpu_scene::{MeshletViewBindGroups, MeshletViewResources},
    pipelines::MeshletPipelines,
};
use crate::{LightEntity, ShadowView};
use bevy_ecs::{
    query::{AnyOf, QueryItem},
    world::World,
};
use bevy_render::{
    camera::ExtractedCamera,
    color::Color,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        ComputePassDescriptor, IndexFormat, LoadOp, Operations, RenderPassColorAttachment,
        RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp,
    },
    renderer::RenderContext,
    view::{ViewDepthTexture, ViewUniformOffset},
};

pub mod draw_3d_graph {
    pub mod node {
        pub const MESHLET_VISIBILITY_BUFFER_PASS: &str = "meshlet_visibility_buffer_pass";
    }
}

#[derive(Default)]
pub struct MeshletVisibilityBufferPassNode;
impl ViewNode for MeshletVisibilityBufferPassNode {
    type ViewData = (
        Option<&'static ExtractedCamera>,
        AnyOf<(&'static ViewDepthTexture, &'static ShadowView)>,
        Option<&'static LightEntity>,
        &'static ViewUniformOffset,
        &'static MeshletViewBindGroups,
        &'static MeshletViewResources,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            view_depth,
            light_entity,
            view_offset,
            meshlet_view_bind_groups,
            meshlet_view_resources,
        ): QueryItem<Self::ViewData>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Some((
            culling_first_pipeline,
            culling_second_pipeline,
            downsample_depth_pipeline,
            visibility_buffer_pipeline,
            visibility_buffer_with_output_pipeline,
            visibility_buffer_depth_clamp_ortho,
            copy_material_depth_pipeline,
        )) = MeshletPipelines::get(world)
        else {
            return Ok(());
        };

        let is_directional_light = matches!(light_entity, Some(&LightEntity::Directional { .. }));
        let (first_pass_visibility_buffer_pipeline, second_pass_visibility_buffer_pipeline) =
            match view_depth {
                (Some(_), None) => (
                    visibility_buffer_pipeline,
                    visibility_buffer_with_output_pipeline,
                ),
                (None, Some(_)) if is_directional_light => (
                    visibility_buffer_depth_clamp_ortho,
                    visibility_buffer_depth_clamp_ortho,
                ),
                (None, Some(_)) => (visibility_buffer_pipeline, visibility_buffer_pipeline),
                _ => unreachable!(),
            };

        let culling_workgroups = (meshlet_view_resources.scene_meshlet_count + 127) / 128;

        render_context
            .command_encoder()
            .push_debug_group(draw_3d_graph::node::MESHLET_VISIBILITY_BUFFER_PASS);

        {
            let command_encoder = render_context.command_encoder();
            let mut cull_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("meshlet_culling_first_pass"),
                timestamp_writes: None,
            });
            cull_pass.set_bind_group(
                0,
                &meshlet_view_bind_groups.culling_first,
                &[view_offset.offset],
            );
            cull_pass.set_pipeline(culling_first_pipeline);
            cull_pass.dispatch_workgroups(culling_workgroups, 1, 1);
        }

        {
            let mut draw_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("meshlet_visibility_buffer_first_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(match view_depth {
                    (Some(view_depth), None) => view_depth.get_attachment(StoreOp::Store),
                    (None, Some(shadow_view)) => {
                        shadow_view.depth_attachment.get_attachment(StoreOp::Store)
                    }
                    _ => unreachable!(),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(viewport) = camera.and_then(|camera| camera.viewport.as_ref()) {
                draw_pass.set_camera_viewport(&viewport);
            }

            draw_pass.set_index_buffer(
                meshlet_view_resources
                    .visibility_buffer_draw_index_buffer
                    .slice(..),
                0,
                IndexFormat::Uint32,
            );
            draw_pass.set_bind_group(
                0,
                &meshlet_view_bind_groups.visibility_buffer,
                &[view_offset.offset],
            );
            draw_pass.set_render_pipeline(first_pass_visibility_buffer_pipeline);
            draw_pass.draw_indexed_indirect(
                &meshlet_view_resources.visibility_buffer_draw_command_buffer_first,
                0,
            );
        }

        render_context
            .command_encoder()
            .push_debug_group("meshlet_downsample_depth");
        for i in 0..6 {
            let downsample_pass = RenderPassDescriptor {
                label: Some("meshlet_downsample_depth_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &meshlet_view_resources.depth_pyramid_mips[i],
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK.into()),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            };
            let mut downsample_pass = render_context.begin_tracked_render_pass(downsample_pass);
            downsample_pass.set_bind_group(0, &meshlet_view_bind_groups.downsample_depth[i], &[]);
            downsample_pass.set_render_pipeline(downsample_depth_pipeline);
            downsample_pass.draw(0..3, 0..1);
        }
        render_context.command_encoder().pop_debug_group();

        {
            let command_encoder = render_context.command_encoder();
            let mut cull_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("meshlet_culling_second_pass"),
                timestamp_writes: None,
            });
            cull_pass.set_bind_group(
                0,
                &meshlet_view_bind_groups.culling_second,
                &[view_offset.offset],
            );
            cull_pass.set_pipeline(culling_second_pipeline);
            cull_pass.dispatch_workgroups(culling_workgroups, 1, 1);
        }

        {
            let mut color_attachments_filled = [None, None];
            if let (Some(visibility_buffer), Some(material_depth_color)) = (
                meshlet_view_resources.visibility_buffer.as_ref(),
                meshlet_view_resources.material_depth_color.as_ref(),
            ) {
                color_attachments_filled = [
                    Some(RenderPassColorAttachment {
                        view: &visibility_buffer.default_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK.into()),
                            store: StoreOp::Store,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &material_depth_color.default_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK.into()),
                            store: StoreOp::Store,
                        },
                    }),
                ];
            }

            let mut draw_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("meshlet_visibility_buffer_second_pass"),
                color_attachments: if color_attachments_filled[0].is_none() {
                    &[]
                } else {
                    &color_attachments_filled
                },
                depth_stencil_attachment: Some(match view_depth {
                    (Some(view_depth), None) => view_depth.get_attachment(StoreOp::Store),
                    (None, Some(shadow_view)) => {
                        shadow_view.depth_attachment.get_attachment(StoreOp::Store)
                    }
                    _ => unreachable!(),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(viewport) = camera.and_then(|camera| camera.viewport.as_ref()) {
                draw_pass.set_camera_viewport(&viewport);
            }

            draw_pass.set_index_buffer(
                meshlet_view_resources
                    .visibility_buffer_draw_index_buffer
                    .slice(..),
                0,
                IndexFormat::Uint32,
            );
            draw_pass.set_bind_group(
                0,
                &meshlet_view_bind_groups.visibility_buffer,
                &[view_offset.offset],
            );
            draw_pass.set_render_pipeline(second_pass_visibility_buffer_pipeline);
            draw_pass.draw_indexed_indirect(
                &meshlet_view_resources.visibility_buffer_draw_command_buffer_second,
                0,
            );
        }

        if let (Some(material_depth), Some(copy_material_depth_bind_group)) = (
            meshlet_view_resources.material_depth.as_ref(),
            meshlet_view_bind_groups.copy_material_depth.as_ref(),
        ) {
            let mut copy_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("meshlet_copy_material_depth_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &material_depth.default_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if let Some(viewport) = camera.and_then(|camera| camera.viewport.as_ref()) {
                copy_pass.set_camera_viewport(&viewport);
            }

            copy_pass.set_bind_group(0, &copy_material_depth_bind_group, &[]);
            copy_pass.set_render_pipeline(copy_material_depth_pipeline);
            copy_pass.draw(0..3, 0..1);
        }

        render_context.command_encoder().pop_debug_group();

        Ok(())
    }
}
