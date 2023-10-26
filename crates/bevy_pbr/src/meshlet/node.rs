use super::{culling_pipeline::MeshletCullingPipeline, gpu_scene::MeshletGpuScene};
use crate::{MeshViewBindGroup, ViewFogUniformOffset, ViewLightsUniformOffset};
use bevy_core_pipeline::{
    clear_color::{ClearColor, ClearColorConfig},
    core_3d::{Camera3d, Camera3dDepthLoadOp},
};
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        ComputePassDescriptor, IndexFormat, LoadOp, Operations, PipelineCache,
        RenderPassDepthStencilAttachment, RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::{ViewDepthTexture, ViewTarget, ViewUniformOffset},
};

pub mod draw_3d_graph {
    pub mod node {
        pub const MAIN_MESHLET_OPAQUE_PASS_3D: &str = "main_meshlet_opaque_pass_3d";
    }
}

#[derive(Default)]
pub struct MainMeshletOpaquePass3dNode;
impl ViewNode for MainMeshletOpaquePass3dNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static Camera3d,
        &'static ViewTarget,
        &'static ViewDepthTexture,
        &'static MeshViewBindGroup,
        &'static ViewUniformOffset,
        &'static ViewLightsUniformOffset,
        &'static ViewFogUniformOffset,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            camera_3d,
            target,
            depth,
            mesh_view_bind_group,
            view_offset,
            view_lights_offset,
            view_fog_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let (Some(gpu_scene), Some(pipeline_cache), Some(culling_pipeline), Some(view_entity)) = (
            world.get_resource::<MeshletGpuScene>(),
            world.get_resource::<PipelineCache>(),
            MeshletCullingPipeline::get(world),
            graph.view_entity(),
        ) else {
            return Ok(());
        };
        let (
            scene_meshlet_count,
            materials,
            Some(culling_bind_group),
            Some(draw_bind_group),
            Some(draw_index_buffer),
            Some(draw_command_buffer),
        ) = gpu_scene.resources_for_view(view_entity)
        else {
            return Ok(());
        };

        {
            let command_encoder = render_context.command_encoder();
            let mut culling_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("meshlet_culling_pass"),
            });
            culling_pass.set_bind_group(0, culling_bind_group, &[view_offset.offset]);
            culling_pass.set_pipeline(culling_pipeline);
            culling_pass.dispatch_workgroups((scene_meshlet_count + 127) / 128, 1, 1);
        }

        {
            let color_load = if target.is_first_write() {
                match camera_3d.clear_color {
                    ClearColorConfig::Default => {
                        LoadOp::Clear(world.resource::<ClearColor>().0.into())
                    }
                    ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                    ClearColorConfig::None => LoadOp::Load,
                }
            } else {
                LoadOp::Load
            };
            let depth_load = if depth.is_first_write() {
                camera_3d.depth_load_op.clone()
            } else {
                Camera3dDepthLoadOp::Load
            }
            .into();

            let mut draw_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some(draw_3d_graph::node::MAIN_MESHLET_OPAQUE_PASS_3D),
                color_attachments: &[Some(target.get_color_attachment(Operations {
                    load: color_load,
                    store: true,
                }))],
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

            draw_pass.set_index_buffer(draw_index_buffer.slice(..), 0, IndexFormat::Uint32);
            draw_pass.set_bind_group(
                0,
                &mesh_view_bind_group.value,
                &[
                    view_offset.offset,
                    view_lights_offset.offset,
                    view_fog_offset.offset,
                ],
            );
            draw_pass.set_bind_group(1, draw_bind_group, &[]);

            for (i, (pipeline_id, material_bind_group)) in materials.enumerate() {
                if let Some(pipeline) = pipeline_cache.get_render_pipeline(pipeline_id) {
                    draw_pass.set_bind_group(2, material_bind_group, &[]);
                    draw_pass.set_render_pipeline(pipeline);
                    draw_pass.draw_indexed_indirect(draw_command_buffer, i as u64);
                }
            }
        }

        Ok(())
    }
}
