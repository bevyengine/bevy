use super::{SkyboxBindGroup, SkyboxMesh, SkyboxPipelineId};
use bevy_ecs::{query::QueryState, world::World};
use bevy_render::{
    camera::ExtractedCamera,
    mesh::GpuBufferInfo,
    prelude::Mesh,
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_resource::{
        LoadOp, Operations, PipelineCache, RenderPassDepthStencilAttachment, RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::{ViewDepthTexture, ViewTarget, ViewUniformOffset},
};

pub struct SkyboxNode {
    view_query: QueryState<(
        &'static SkyboxPipelineId,
        &'static SkyboxBindGroup,
        &'static ViewUniformOffset,
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static ViewDepthTexture,
    )>,
}

impl Node for SkyboxNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Ok((pipeline_id, bind_group, view_uniform_offset, camera, target, depth))
            = self.view_query.get_manual(world, graph.view_entity()) else { return Ok(()) };
        let skybox_mesh = world.resource::<SkyboxMesh>();
        let meshes = world.resource::<RenderAssets<Mesh>>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let (Some(gpu_mesh), Some(pipeline)) = (
            meshes.get(&skybox_mesh.handle),
            pipeline_cache.get_render_pipeline(pipeline_id.0),
        ) else { return Ok(()) };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("skybox"),
            color_attachments: &[Some(target.get_color_attachment(Operations {
                load: LoadOp::Load,
                store: true,
            }))],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &depth.view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: false,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group.0, &[view_uniform_offset.offset]);

        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        render_pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                render_pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                render_pass.draw_indexed(0..*count, 0, 0..1);
            }
            GpuBufferInfo::NonIndexed { vertex_count } => {
                render_pass.draw(0..*vertex_count, 0..1);
            }
        }

        Ok(())
    }

    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }
}

impl SkyboxNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}
