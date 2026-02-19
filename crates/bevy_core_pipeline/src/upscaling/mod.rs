use crate::blit::{BlitPipeline, BlitPipelineKey};
use bevy_app::prelude::*;
use bevy_camera::CameraOutputMode;
use bevy_ecs::prelude::*;
use bevy_platform::collections::HashSet;
use bevy_render::{
    camera::ExtractedCamera, render_resource::*, view::ViewTarget, Render, RenderApp, RenderSystems,
};

mod node;

pub use node::UpscalingNode;

pub struct UpscalingPlugin;

impl Plugin for UpscalingPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                Render,
                // This system should probably technically be run *after* all of the other systems
                // that might modify `PipelineCache` via interior mutability, but for now,
                // we've chosen to simply ignore the ambiguities out of a desire for a better refactor
                // and aversion to extensive and intrusive system ordering.
                // See https://github.com/bevyengine/bevy/issues/14770 for more context.
                prepare_view_upscaling_pipelines
                    .in_set(RenderSystems::Prepare)
                    .ambiguous_with_all(),
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
    let mut output_textures = <HashSet<_>>::default();
    for (entity, view_target, camera) in view_targets.iter() {
        let out_texture_id = view_target.out_texture().id();
        let blend_state = if let Some(extracted_camera) = camera {
            match extracted_camera.output_mode {
                CameraOutputMode::Skip => None,
                CameraOutputMode::Write { blend_state, .. } => {
                    let already_seen = output_textures.contains(&out_texture_id);
                    output_textures.insert(out_texture_id);

                    match blend_state {
                        None => {
                            // If we've already seen this output for a camera and it doesn't have an output blend
                            // mode configured, default to alpha blend so that we don't accidentally overwrite
                            // the output texture
                            if already_seen {
                                Some(BlendState::ALPHA_BLENDING)
                            } else {
                                None
                            }
                        }
                        _ => blend_state,
                    }
                }
            }
        } else {
            output_textures.insert(out_texture_id);
            None
        };

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
