use super::{
    gpu_scene::{MeshletViewBindGroups, MeshletViewResources},
    material_draw_prepare::MeshletViewMaterials,
    MeshletGpuScene,
};
use crate::{MeshViewBindGroup, ViewFogUniformOffset, ViewLightsUniformOffset};
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        LoadOp, Operations, PipelineCache, RenderPassDepthStencilAttachment, RenderPassDescriptor,
        StoreOp,
    },
    renderer::RenderContext,
    view::{ViewTarget, ViewUniformOffset},
};

pub mod draw_3d_graph {
    pub mod node {
        pub const MESHLET_MAIN_OPAQUE_PASS_3D: &str = "meshlet_main_opaque_pass_3d";
    }
}

#[derive(Default)]
pub struct MeshletMainOpaquePass3dNode;
impl ViewNode for MeshletMainOpaquePass3dNode {
    type ViewData = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static MeshViewBindGroup,
        &'static ViewUniformOffset,
        &'static ViewLightsUniformOffset,
        &'static ViewFogUniformOffset,
        &'static MeshletViewMaterials,
        &'static MeshletViewBindGroups,
        &'static MeshletViewResources,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            target,
            mesh_view_bind_group,
            view_offset,
            view_lights_offset,
            view_fog_offset,
            meshlet_view_materials,
            meshlet_view_bind_groups,
            meshlet_view_resources,
        ): QueryItem<Self::ViewData>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let (
            Some(meshlet_gpu_scene),
            Some(pipeline_cache),
            Some(meshlet_material_depth),
            Some(meshlet_material_draw_bind_group),
        ) = (
            world.get_resource::<MeshletGpuScene>(),
            world.get_resource::<PipelineCache>(),
            meshlet_view_resources.material_depth.as_ref(),
            meshlet_view_bind_groups.material_draw.as_ref(),
        )
        else {
            return Ok(());
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some(draw_3d_graph::node::MESHLET_MAIN_OPAQUE_PASS_3D),
            color_attachments: &[Some(target.get_color_attachment())],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &meshlet_material_depth.default_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        render_pass.set_bind_group(
            0,
            &mesh_view_bind_group.value,
            &[
                view_offset.offset,
                view_lights_offset.offset,
                view_fog_offset.offset,
            ],
        );
        render_pass.set_bind_group(1, meshlet_material_draw_bind_group, &[]);

        for (material_id, material_pipeline_id, material_bind_group) in
            &meshlet_view_materials.opaque_pass
        {
            if meshlet_gpu_scene.material_present_in_scene(material_id) {
                if let Some(material_pipeline) =
                    pipeline_cache.get_render_pipeline(*material_pipeline_id)
                {
                    let x = *material_id * 3;
                    render_pass.set_bind_group(2, material_bind_group, &[]);
                    render_pass.set_render_pipeline(material_pipeline);
                    render_pass.draw(x..(x + 3), 0..1);
                }
            }
        }

        Ok(())
    }
}
