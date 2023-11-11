use super::{
    gpu_scene::{MeshletViewBindGroups, MeshletViewResources},
    pipelines::MeshletPipelines,
};
use bevy_core_pipeline::core_3d::{Camera3d, Camera3dDepthLoadOp};
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    camera::ExtractedCamera,
    color::Color,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        ComputePassDescriptor, IndexFormat, LoadOp, Operations, RenderPassColorAttachment,
        RenderPassDepthStencilAttachment, RenderPassDescriptor,
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
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static Camera3d,
        &'static ViewDepthTexture,
        &'static ViewUniformOffset,
        &'static MeshletViewBindGroups,
        &'static MeshletViewResources,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (camera, camera_3d, depth, view_offset, meshlet_view_bind_groups, meshlet_view_resources): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Some((
            Some(culling_pipeline),
            Some(visibility_buffer_pipeline),
            Some(copy_material_depth_pipeline),
        )) = MeshletPipelines::get(world)
        else {
            return Ok(());
        };

        let depth_load = if depth.is_first_write() {
            camera_3d.depth_load_op.clone()
        } else {
            Camera3dDepthLoadOp::Load
        }
        .into();

        render_context
            .command_encoder()
            .push_debug_group(draw_3d_graph::node::MESHLET_VISIBILITY_BUFFER_PASS);

        {
            let command_encoder = render_context.command_encoder();
            let mut cull_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("meshlet_culling_pass"),
            });
            cull_pass.set_bind_group(0, &meshlet_view_bind_groups.culling, &[view_offset.offset]);
            cull_pass.set_pipeline(culling_pipeline);
            cull_pass.dispatch_workgroups(
                (meshlet_view_resources.scene_meshlet_count + 127) / 128,
                1,
                1,
            );
        }

        {
            let mut draw_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("meshlet_visibility_buffer_render_pass"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &meshlet_view_resources.visibility_buffer.default_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK.into()),
                            store: true,
                        },
                    }),
                    Some(RenderPassColorAttachment {
                        view: &meshlet_view_resources.material_depth_color.default_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLACK.into()),
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(Operations {
                        load: depth_load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            if let Some(viewport) = camera.viewport.as_ref() {
                draw_pass.set_camera_viewport(viewport);
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
            draw_pass.set_render_pipeline(visibility_buffer_pipeline);
            draw_pass.draw_indexed_indirect(
                &meshlet_view_resources.visibility_buffer_draw_command_buffer,
                0,
            );
        }

        {
            let mut copy_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("meshlet_copy_material_depth_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &meshlet_view_resources.material_depth.default_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            if let Some(viewport) = camera.viewport.as_ref() {
                copy_pass.set_camera_viewport(viewport);
            }

            copy_pass.set_bind_group(0, &meshlet_view_bind_groups.copy_material_depth, &[]);
            copy_pass.set_render_pipeline(copy_material_depth_pipeline);
            copy_pass.draw(0..3, 0..1);
        }

        render_context.command_encoder().pop_debug_group();

        Ok(())
    }
}
