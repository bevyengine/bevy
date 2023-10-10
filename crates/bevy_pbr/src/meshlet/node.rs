use super::{
    gpu_scene::{MeshletGpuScene, MeshletMeshGpuSceneSlice},
    per_frame_resources::MeshletPerFrameResources,
};
use crate::{MeshTransforms, MeshViewBindGroup, ViewFogUniformOffset, ViewLightsUniformOffset};
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
    render_resource::{
        ComputePassDescriptor, IndexFormat, LoadOp, Operations, RenderPassDepthStencilAttachment,
        RenderPassDescriptor,
    },
    renderer::{RenderContext, RenderQueue},
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
        let gpu_scene = world.resource::<MeshletGpuScene>();
        if gpu_scene.total_instanced_meshlet_count() == 0 {
            return Ok(());
        }
        let gpu_scene_bind_group = gpu_scene.create_bind_group(render_context.render_device());
        let (culling_bind_group, draw_bind_group, draw_command_buffer, draw_index_buffer) =
            world.resource::<MeshletPerFrameResources>().create(
                todo!(),
                gpu_scene,
                world.resource::<RenderQueue>(),
                render_context.render_device(),
            );

        {
            let command_encoder = render_context.command_encoder();
            let mut culling_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("meshlet_culling_pass"),
            });

            culling_pass.set_bind_group(
                0,
                &mesh_view_bind_group.value,
                &[
                    view_offset.offset,
                    view_lights_offset.offset,
                    view_fog_offset.offset,
                ],
            );
            culling_pass.set_bind_group(1, &gpu_scene_bind_group, &[]);
            culling_pass.set_bind_group(2, &culling_bind_group, &[]);

            culling_pass.set_pipeline(todo!("Culling pipeline"));
            culling_pass.dispatch_workgroups(
                div_ceil(gpu_scene.total_instanced_meshlet_count(), 128),
                1,
                1,
            );
        }

        {
            let mut draw_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
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
            draw_pass.set_bind_group(1, &gpu_scene_bind_group, &[]);
            draw_pass.set_bind_group(2, &draw_bind_group, &[]);

            for (i, material) in [todo!()].iter().enumerate() {
                draw_pass.set_bind_group(3, todo!("Material bind group"), &[]);
                draw_pass.set_render_pipeline(todo!("Material pipeline"));
                draw_pass.draw_indexed_indirect(&draw_command_buffer, i as u64);
            }
        }

        Ok(())
    }
}

/// Divide `numerator` by `denominator`, rounded up to the nearest multiple of `denominator`.
fn div_ceil(numerator: u32, denominator: u32) -> u32 {
    (numerator + denominator - 1) / denominator
}
