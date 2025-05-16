use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_platform::collections::HashSet;
use bevy_render::{
    camera::{CameraOutputMode, ExtractedCamera},
    render_resource::*,
    renderer::RenderDevice,
    texture_blitter::{TextureBlitter, TextureBlitterBuilder},
    view::ViewTarget,
    Render, RenderApp, RenderSystems,
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
pub struct ViewUpscalingTextureBlitter(TextureBlitter);

fn prepare_view_upscaling_pipelines(
    mut commands: Commands,
    view_targets: Query<(Entity, &ViewTarget, Option<&ExtractedCamera>)>,
    render_device: Res<RenderDevice>,
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

        let mut texture_blitter_builder = TextureBlitterBuilder::new(
            render_device.wgpu_device(),
            view_target.out_texture_format(),
        );
        if let Some(blend_state) = blend_state {
            texture_blitter_builder = texture_blitter_builder.blend_state(blend_state);
        }
        let texture_blitter = texture_blitter_builder.build();

        commands
            .entity(entity)
            .insert(ViewUpscalingTextureBlitter(texture_blitter));
    }
}
