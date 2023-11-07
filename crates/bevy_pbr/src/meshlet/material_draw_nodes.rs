use super::{
    gpu_scene::{MeshletViewBindGroups, MeshletViewResources},
    prepare_materials::MeshletViewMaterials,
    MeshletGpuScene,
};
use bevy_core_pipeline::{
    clear_color::{ClearColor, ClearColorConfig},
    core_3d::Camera3d,
};
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        LoadOp, Operations, PipelineCache, RenderPassDepthStencilAttachment, RenderPassDescriptor,
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
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static Camera3d,
        &'static ViewTarget,
        &'static ViewUniformOffset,
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
            camera_3d,
            target,
            view_offset,
            meshlet_view_materials,
            meshlet_view_bind_groups,
            meshlet_view_resources,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let (Some(gpu_scene), Some(pipeline_cache)) = (
            world.get_resource::<MeshletGpuScene>(),
            world.get_resource::<PipelineCache>(),
        ) else {
            return Ok(());
        };

        let load = if target.is_first_write() {
            match camera_3d.clear_color {
                ClearColorConfig::Default => LoadOp::Clear(world.resource::<ClearColor>().0.into()),
                ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                ClearColorConfig::None => LoadOp::Load,
            }
        } else {
            LoadOp::Load
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some(draw_3d_graph::node::MESHLET_MAIN_OPAQUE_PASS_3D),
            color_attachments: &[Some(
                target.get_color_attachment(Operations { load, store: true }),
            )],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &meshlet_view_resources.material_depth.default_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: false,
                }),
                stencil_ops: None,
            }),
        });
        if let Some(viewport) = camera.viewport.as_ref() {
            render_pass.set_camera_viewport(viewport);
        }

        // TODO: Set view bind groups

        for (material_id, material_pipeline_id, material_bind_group) in
            &meshlet_view_materials.opaque_pass
        {
            if gpu_scene.material_used(material_id) {
                if let Some(material_pipeline) =
                    pipeline_cache.get_render_pipeline(*material_pipeline_id)
                {
                    render_pass.set_bind_group(todo!(), material_bind_group, &[]);
                    render_pass.set_render_pipeline(material_pipeline);
                    render_pass.draw(*material_id..(*material_id + 3), 0..1);
                }
            }
        }

        Ok(())
    }
}
