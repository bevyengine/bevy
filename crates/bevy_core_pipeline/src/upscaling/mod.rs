use crate::blit::{BlitPipeline, BlitPipelineKey};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_render::camera::{CameraOutputMode, ExtractedCamera};
use bevy_render::view::ViewTarget;
use bevy_render::{render_resource::*, Render, RenderApp, RenderSet};
use bevy_utils::HashSet;

mod node;

pub use node::UpscalingNode;

pub struct UpscalingPlugin;

impl Plugin for UpscalingPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
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
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    blit_pipeline: Res<BlitPipeline>,
    view_targets: Query<(Entity, &ViewTarget, Option<&ExtractedCamera>)>,
) {
    let mut output_textures = HashSet::new();
    for (entity, view_target, camera) in view_targets.iter() {
        let out_texture_id = view_target.out_texture().id();
        let blend_state = if let Some(ExtractedCamera {
            output_mode: CameraOutputMode::Write { blend_state, .. },
            ..
        }) = camera
        {
            match *blend_state {
                None => {
                    // If we've already seen this output for a camera and it doesn't have a output blend
                    // mode configured, default to alpha blend so that we don't accidentally overwrite
                    // the output texture
                    if output_textures.contains(&out_texture_id) {
                        Some(BlendState::ALPHA_BLENDING)
                    } else {
                        None
                    }
                }
                _ => *blend_state,
            }
        } else {
            None
        };
        output_textures.insert(out_texture_id);

        let key = BlitPipelineKey {
            texture_format: view_target.out_texture_format(),
            blend_state,
            samples: 1,
        };
        let pipeline = pipelines.specialize(&pipeline_cache, &blit_pipeline, key);

        // Ensure the pipeline is loaded before continuing the frame to prevent frames without any GPU work submitted
        pipeline_cache.block_on_render_pipeline(pipeline);

        commands
            .entity(entity)
            .insert(ViewUpscalingPipeline(pipeline));
    }
}
