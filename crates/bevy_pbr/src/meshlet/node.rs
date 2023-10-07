use crate::{MeshViewBindGroup, ViewFogUniformOffset, ViewLightsUniformOffset};
use bevy_core_pipeline::{
    clear_color::{ClearColor, ClearColorConfig},
    core_3d::{Camera3d, Camera3dDepthLoadOp},
    prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass},
};
use bevy_ecs::{
    query::{Has, QueryItem},
    world::World,
};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor},
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
        Has<DepthPrepass>,
        Has<NormalPrepass>,
        Has<MotionVectorPrepass>,
        &'static MeshViewBindGroup,
        &'static ViewUniformOffset,
        &'static ViewLightsUniformOffset,
        &'static ViewFogUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            camera_3d,
            target,
            depth,
            depth_prepass,
            normal_prepass,
            motion_vector_prepass,
            mesh_view_bind_group,
            view_offset,
            view_lights_offset,
            view_fog_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some(draw_3d_graph::node::MAIN_MESHLET_OPAQUE_PASS_3D),
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
                depth_ops: Some(Operations {
                    load: if depth_prepass || normal_prepass || motion_vector_prepass {
                        Camera3dDepthLoadOp::Load
                    } else {
                        camera_3d.depth_load_op.clone()
                    }
                    .into(),
                    store: true,
                }),
                stencil_ops: None,
            }),
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

        let material = todo!();
        render_pass.set_render_pipeline(todo!());
        // TODO: Set material bind group

        // TODO: Setup meshlet per-material bind groups

        // TODO: Dispatch/draws
    }
}
