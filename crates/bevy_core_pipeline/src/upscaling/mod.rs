use crate::blit::{BlitPipeline, BlitPipelineKey};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_render::camera::{CameraOutputMode, ExtractedCamera};
use bevy_render::view::ViewTarget;
use bevy_render::{render_resource::*, Render, RenderApp, RenderSet};

mod node;

pub use node::UpscalingNode;

pub struct UpscalingPlugin;

impl Plugin for UpscalingPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                Render,
                prepare_view_upscaling_pipelines.in_set(RenderSet::Prepare),
            );
        }
    }
}

#[derive(Component)]
pub struct ViewUpscalingPipeline(CachedRenderPipelineId);

fn prepare_view_upscaling_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    blit_pipeline: Res<BlitPipeline>,
    view_targets: Query<(Entity, &ViewTarget, Option<&ExtractedCamera>)>,
) {
    for (entity, view_target, camera) in view_targets.iter() {
        let blend_state = if let Some(ExtractedCamera {
            output_mode: CameraOutputMode::Write { blend_state, .. },
            ..
        }) = camera
        {
            *blend_state
        } else {
            None
        };
        let key = BlitPipelineKey {
            texture_format: view_target.out_texture_format(),
            blend_state,
            samples: 1,
        };
        let pipeline = pipelines.specialize(&pipeline_cache, &blit_pipeline, key);

        commands
            .entity(entity)
            .insert(ViewUpscalingPipeline(pipeline));
    }
}
