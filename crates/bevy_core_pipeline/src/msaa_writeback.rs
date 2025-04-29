use crate::{
    blit::{BlitPipeline, BlitPipelineKey},
    core_2d::graph::{Core2d, Node2d},
    core_3d::graph::{Core3d, Node3d},
};
use bevy_app::{App, Plugin};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    frame_graph::FrameGraph,
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::*,
    view::{Msaa, ViewTarget},
    Render, RenderApp, RenderSet,
};

/// This enables "msaa writeback" support for the `core_2d` and `core_3d` pipelines, which can be enabled on cameras
/// using [`bevy_render::camera::Camera::msaa_writeback`]. See the docs on that field for more information.
pub struct MsaaWritebackPlugin;

impl Plugin for MsaaWritebackPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            prepare_msaa_writeback_pipelines.in_set(RenderSet::Prepare),
        );
        {
            render_app
                .add_render_graph_node::<ViewNodeRunner<MsaaWritebackNode>>(
                    Core2d,
                    Node2d::MsaaWriteback,
                )
                .add_render_graph_edge(Core2d, Node2d::MsaaWriteback, Node2d::StartMainPass);
        }
        {
            render_app
                .add_render_graph_node::<ViewNodeRunner<MsaaWritebackNode>>(
                    Core3d,
                    Node3d::MsaaWriteback,
                )
                .add_render_graph_edge(Core3d, Node3d::MsaaWriteback, Node3d::StartMainPass);
        }
    }
}

#[derive(Default)]
pub struct MsaaWritebackNode;

impl ViewNode for MsaaWritebackNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static MsaaWritebackBlitPipeline,
        &'static Msaa,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        _frame_graph: &mut FrameGraph,
        (_target, _blit_pipeline_id, _msaa): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
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
    view_targets: Query<(Entity, &ViewTarget, &ExtractedCamera, &Msaa)>,
) {
    for (entity, view_target, camera, msaa) in view_targets.iter() {
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
