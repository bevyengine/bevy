use crate::{
    blit::{BlitPipeline, BlitPipelineKey},
    core_2d::{self, CORE_2D},
    core_3d::{self, CORE_3D},
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext},
    renderer::RenderContext,
    view::{Msaa, ViewTarget},
    Render, RenderSet,
};
use bevy_render::{render_resource::*, RenderApp};

/// This enables "msaa writeback" support for the `core_2d` and `core_3d` pipelines, which can be enabled on cameras
/// using [`bevy_render::camera::Camera::msaa_writeback`]. See the docs on that field for more information.
pub struct MsaaWritebackPlugin;

impl Plugin for MsaaWritebackPlugin {
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            Render,
            prepare_msaa_writeback_pipelines.in_set(RenderSet::Prepare),
        );
        {
            use core_2d::graph::node::*;
            render_app
                .add_render_graph_node::<MsaaWritebackNode>(CORE_2D, MSAA_WRITEBACK)
                .add_render_graph_edge(CORE_2D, MSAA_WRITEBACK, MAIN_PASS);
        }
        {
            use core_3d::graph::node::*;
            render_app
                .add_render_graph_node::<MsaaWritebackNode>(CORE_3D, MSAA_WRITEBACK)
                .add_render_graph_edge(CORE_3D, MSAA_WRITEBACK, START_MAIN_PASS);
        }
    }
}

pub struct MsaaWritebackNode {
    cameras: QueryState<(&'static ViewTarget, &'static MsaaWritebackBlitPipeline)>,
}

impl FromWorld for MsaaWritebackNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            cameras: world.query(),
        }
    }
}

impl Node for MsaaWritebackNode {
    fn update(&mut self, world: &mut World) {
        self.cameras.update_archetypes(world);
    }
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();
        if let Ok((target, blit_pipeline_id)) = self.cameras.get_manual(world, view_entity) {
            let blit_pipeline = world.resource::<BlitPipeline>();
            let pipeline_cache = world.resource::<PipelineCache>();
            let pipeline = pipeline_cache
                .get_render_pipeline(blit_pipeline_id.0)
                .unwrap();

            // The current "main texture" needs to be bound as an input resource, and we need the "other"
            // unused target to be the "resolve target" for the MSAA write. Therefore this is the same
            // as a post process write!
            let post_process = target.post_process_write();

            let pass_descriptor = RenderPassDescriptor {
                label: Some("msaa_writeback"),
                // The target's "resolve target" is the "destination" in post_process
                // We will indirectly write the results to the "destination" using
                // the MSAA resolve step.
                color_attachments: &[Some(target.get_color_attachment(Operations {
                    load: LoadOp::Clear(Default::default()),
                    store: true,
                }))],
                depth_stencil_attachment: None,
            };

            let bind_group =
                render_context
                    .render_device()
                    .create_bind_group(&BindGroupDescriptor {
                        label: None,
                        layout: &blit_pipeline.texture_bind_group,
                        entries: &[
                            BindGroupEntry {
                                binding: 0,
                                resource: BindingResource::TextureView(post_process.source),
                            },
                            BindGroupEntry {
                                binding: 1,
                                resource: BindingResource::Sampler(&blit_pipeline.sampler),
                            },
                        ],
                    });

            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&pass_descriptor);

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
        Ok(())
    }
}

#[derive(Component)]
pub struct MsaaWritebackBlitPipeline(CachedRenderPipelineId);

fn prepare_msaa_writeback_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    blit_pipeline: Res<BlitPipeline>,
    view_targets: Query<(Entity, &ViewTarget, &ExtractedCamera)>,
    msaa: Res<Msaa>,
) {
    for (entity, view_target, camera) in view_targets.iter() {
        // only do writeback if writeback is enabled for the camera and this isn't the first camera in the target,
        // as there is nothing to write back for the first camera.
        if msaa.samples() > 1 && camera.msaa_writeback && camera.sorted_camera_index_for_target > 0
        {
            let key = BlitPipelineKey {
                texture_format: view_target.main_texture_format(),
                samples: msaa.samples(),
                blend_state: None,
            };

            let pipeline = pipelines.specialize(&pipeline_cache, &blit_pipeline, key);
            commands
                .entity(entity)
                .insert(MsaaWritebackBlitPipeline(pipeline));
        } else {
            // This isn't strictly necessary now, but if we move to retained render entity state I don't
            // want this to silently break
            commands
                .entity(entity)
                .remove::<MsaaWritebackBlitPipeline>();
        }
    }
}
