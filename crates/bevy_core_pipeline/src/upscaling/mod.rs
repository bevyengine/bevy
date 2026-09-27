use crate::blit::{BlitPipeline, BlitPipelineKey};
use bevy_app::prelude::*;
use bevy_camera::CameraOutputMode;
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::*,
    view::{ResolvedCompositingSpace, StackBlit, ViewStackContract, ViewTarget},
    Render, RenderApp, RenderStartup, RenderSystems,
};

mod node;

pub use node::upscaling;

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
            render_app.add_systems(RenderStartup, clear_view_upscaling_pipelines);
        }
    }
}

#[derive(Component)]
pub struct ViewUpscalingPipeline(CachedRenderPipelineId, BlitPipelineKey);

/// This is not required on first startup but is required during render recovery
fn clear_view_upscaling_pipelines(
    mut commands: Commands,
    views: Query<Entity, With<ViewUpscalingPipeline>>,
) {
    for entity in &views {
        commands.entity(entity).remove::<ViewUpscalingPipeline>();
    }
}

fn prepare_view_upscaling_pipelines(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    blit_pipeline: Res<BlitPipeline>,
    view_targets: Query<(
        Entity,
        &ViewTarget,
        Option<&ExtractedCamera>,
        Option<&ViewUpscalingPipeline>,
        Option<&ViewStackContract>,
        Option<&ResolvedCompositingSpace>,
    )>,
) {
    for (entity, view_target, camera, maybe_pipeline, contract, resolved_space) in
        view_targets.iter()
    {
        let replaces = match contract.map(|contract| contract.blit) {
            Some(StackBlit::Skip) => {
                // The upscaling pass requires this pipeline, so removing it
                // skips the blit.
                if maybe_pipeline.is_some() {
                    commands.entity(entity).remove::<ViewUpscalingPipeline>();
                }
                continue;
            }
            Some(StackBlit::Replace) => true,
            Some(StackBlit::Blend) => false,
            // Without a contract, only the target's first camera replaces.
            None => camera.is_some_and(|camera| camera.sorted_camera_index_for_target == 0),
        };

        let blend_state = if let Some(extracted_camera) = camera {
            match extracted_camera.output_mode {
                CameraOutputMode::Skip => None,
                CameraOutputMode::Write { blend_state, .. } => match blend_state {
                    // Later cameras blend so they don't overwrite earlier output.
                    None if !replaces => Some(BlendState::ALPHA_BLENDING),
                    _ => blend_state,
                },
            }
        } else {
            None
        };

        let Some(target_format) = view_target.out_texture_view_format() else {
            continue;
        };

        let key = BlitPipelineKey {
            target_format,
            blend_state,
            samples: 1,
            source_space: ResolvedCompositingSpace::space(resolved_space),
        };

        if maybe_pipeline.is_none_or(|ViewUpscalingPipeline(_, cached_key)| *cached_key != key) {
            let pipeline = pipelines.specialize(&pipeline_cache, &blit_pipeline, key);

            // Ensure the pipeline is loaded before continuing the frame to prevent frames without
            // any GPU work submitted
            pipeline_cache.block_on_render_pipeline(pipeline);

            commands
                .entity(entity)
                .insert(ViewUpscalingPipeline(pipeline, key));
        }
    }
}
