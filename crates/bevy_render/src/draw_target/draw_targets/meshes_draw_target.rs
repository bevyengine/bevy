use crate::{
    draw_target::DrawTarget,
    mesh::{self, Mesh},
    pass::RenderPass,
    pipeline::PipelineDescriptor,
    render_resource::ResourceInfo,
    Renderable,
};
use bevy_asset::Handle;
use legion::prelude::*;

#[derive(Default)]
pub struct MeshesDrawTarget;

impl MeshesDrawTarget {
    pub const NAME: &'static str = "Meshes";
}

impl DrawTarget for MeshesDrawTarget {
    fn draw(
        &self,
        world: &World,
        _resources: &Resources,
        render_pass: &mut dyn RenderPass,
        _pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
    ) {
        let mut current_mesh_handle = None;
        let mut current_mesh_index_len = 0;
        let mesh_query = <(Read<Handle<Mesh>>, Read<Renderable>)>::query();
        for (mesh_handle, renderable) in mesh_query.iter(world) {
            if !renderable.is_visible || renderable.is_instanced {
                continue;
            }

            let render_context = render_pass.get_render_context();
            let render_resources = render_context.resources();
            if current_mesh_handle != Some(*mesh_handle) {
                if let Some(vertex_buffer_resource) = render_resources
                    .get_asset_resource(*mesh_handle, mesh::VERTEX_BUFFER_ASSET_INDEX)
                {
                    let index_buffer_resource = render_resources
                        .get_asset_resource(*mesh_handle, mesh::INDEX_BUFFER_ASSET_INDEX)
                        .unwrap();
                    render_resources.get_resource_info(
                        index_buffer_resource,
                        &mut |resource_info| match resource_info {
                            Some(ResourceInfo::Buffer(Some(buffer_info))) => {
                                current_mesh_index_len = (buffer_info.size / 2) as u32
                            }
                            _ => panic!("expected a buffer type"),
                        },
                    );
                    render_pass.set_index_buffer(index_buffer_resource, 0);
                    render_pass.set_vertex_buffer(0, vertex_buffer_resource, 0);
                }
                // TODO: Verify buffer format matches render pass
                current_mesh_handle = Some(*mesh_handle);
            }

            // TODO: validate bind group properties against shader uniform properties at least once
            render_pass
                .set_render_resources(pipeline_descriptor, &renderable.render_resource_assignments);
            render_pass.draw_indexed(0..current_mesh_index_len, 0, 0..1);
        }
    }

    fn get_name(&self) -> String {
        MeshesDrawTarget::NAME.to_string()
    }
}
