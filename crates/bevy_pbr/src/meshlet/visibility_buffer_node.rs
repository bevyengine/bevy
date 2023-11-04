use super::{gpu_scene::MeshletGpuScene, pipelines::MeshletPipelines};
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
        pub const MESHLET_VISIBILITY_BUFFER_PASS: &str = "meshlet_visibility_pass";
    }
}

#[derive(Default)]
pub struct MeshletVisibilityBufferPassNode;
impl ViewNode for MeshletVisibilityBufferPassNode {
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
        let (
            Some(gpu_scene),
            Some(pipeline_cache),
            Some((Some(culling_pipeline), Some(visibility_buffer_pipeline))),
        ) = (
            world.get_resource::<MeshletGpuScene>(),
            world.get_resource::<PipelineCache>(),
            MeshletPipelines::get(world),
        )
        else {
            return Ok(());
        };
        // let (
        //     scene_meshlet_count,
        //     material_draws,
        //     Some(culling_bind_group),
        //     Some(draw_bind_group),
        //     Some(draw_index_buffer),
        //     Some(draw_command_buffer),
        // ) = gpu_scene.resources_for_view(graph.view_entity(), pipeline_cache)
        // else {
        //     return Ok(());
        // };

        // {
        //     let command_encoder = render_context.command_encoder();
        //     let mut culling_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
        //         label: Some("meshlet_culling_pass"),
        //     });
        //     culling_pass.set_bind_group(0, culling_bind_group, &[view_offset.offset]);
        //     culling_pass.set_pipeline(culling_pipeline);
        //     culling_pass.dispatch_workgroups((scene_meshlet_count + 127) / 128, 1, 1);
        // }

        // let color_load = if target.is_first_write() {
        //     match camera_3d.clear_color {
        //         ClearColorConfig::Default => LoadOp::Clear(world.resource::<ClearColor>().0.into()),
        //         ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
        //         ClearColorConfig::None => LoadOp::Load,
        //     }
        // } else {
        //     LoadOp::Load
        // };
        // let depth_load = if depth.is_first_write() {
        //     camera_3d.depth_load_op.clone()
        // } else {
        //     Camera3dDepthLoadOp::Load
        // }
        // .into();

        // let mut draw_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
        //     label: Some(draw_3d_graph::node::MESHLET_VISIBILITY_BUFFER_PASS),
        //     color_attachments: &[Some(target.get_color_attachment(Operations {
        //         load: color_load,
        //         store: true,
        //     }))],
        //     depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
        //         view: &depth.view,
        //         depth_ops: Some(Operations {
        //             load: depth_load,
        //             store: true,
        //         }),
        //         stencil_ops: None,
        //     }),
        // });
        // if let Some(viewport) = camera.viewport.as_ref() {
        //     draw_pass.set_camera_viewport(viewport);
        // }

        // draw_pass.set_index_buffer(draw_index_buffer.slice(..), 0, IndexFormat::Uint32);
        // draw_pass.set_bind_group(
        //     0,
        //     &mesh_view_bind_group.value,
        //     &[
        //         view_offset.offset,
        //         view_lights_offset.offset,
        //         view_fog_offset.offset,
        //     ],
        // );
        // draw_pass.set_bind_group(1, draw_bind_group, &[]);

        // for (material_draw_offset, material_bind_group, material_pipeline) in material_draws {
        //     draw_pass.set_bind_group(2, material_bind_group, &[]);
        //     draw_pass.set_render_pipeline(material_pipeline);
        //     draw_pass.draw_indexed_indirect(draw_command_buffer, material_draw_offset);
        // }

        Ok(())
    }
}
