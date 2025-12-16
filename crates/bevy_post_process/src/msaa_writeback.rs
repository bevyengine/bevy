use bevy_app::{App, Plugin};
use bevy_camera::MsaaWriteback;
use bevy_color::LinearRgba;
use bevy_core_pipeline::{
    blit::{BlitPipeline, BlitPipelineKey},
    schedule::{Core2d, Core2dSystems, Core3d, Core3dSystems},
};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_resource::*,
    renderer::{RenderContext, ViewQuery},
    view::{Msaa, ViewTarget},
    Render, RenderApp, RenderSystems,
};

/// This enables "msaa writeback" support for the `core_2d` and `core_3d` pipelines, which can be enabled on cameras
/// using [`bevy_camera::Camera::msaa_writeback`]. See the docs on that field for more information.
#[derive(Default)]
pub struct MsaaWritebackPlugin;

impl Plugin for MsaaWritebackPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            prepare_msaa_writeback_pipelines.in_set(RenderSystems::Prepare),
        );
        render_app.add_systems(Core3d, msaa_writeback.before(Core3dSystems::EndPrepasses));
        render_app.add_systems(Core2d, msaa_writeback.before(Core2dSystems::StartMainPass));
    }
}

pub(crate) fn msaa_writeback(
    view: ViewQuery<(&ViewTarget, &MsaaWritebackBlitPipeline, &Msaa)>,
    blit_pipeline: Res<BlitPipeline>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, blit_pipeline_id, msaa) = view.into_inner();

    if *msaa == Msaa::Off {
        return;
    }

    let Some(pipeline) = pipeline_cache.get_render_pipeline(blit_pipeline_id.0) else {
        return;
    };

    // The current "main texture" needs to be bound as an input resource, and we need the "other"
    // unused target to be the "resolve target" for the MSAA write. Therefore this is the same
    // as a post process write!
    let post_process = target.post_process_write();

    let pass_descriptor = RenderPassDescriptor {
        label: Some("msaa_writeback"),
        // The target's "resolve target" is the "destination" in post_process.
        // We will indirectly write the results to the "destination" using
        // the MSAA resolve step.
        color_attachments: &[Some(RenderPassColorAttachment {
            // If MSAA is enabled, then the sampled texture will always exist
            view: target.sampled_main_texture_view().unwrap(),
            depth_slice: None,
            resolve_target: Some(post_process.destination),
            ops: Operations {
                load: LoadOp::Clear(LinearRgba::BLACK.into()),
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    };

    let bind_group =
        blit_pipeline.create_bind_group(ctx.render_device(), post_process.source, &pipeline_cache);

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "msaa_writeback");

    {
        let mut render_pass = ctx.command_encoder().begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    time_span.end(ctx.command_encoder());
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
        // Determine if we should do MSAA writeback based on the camera's setting
        let should_writeback = match camera.msaa_writeback {
            MsaaWriteback::Off => false,
            MsaaWriteback::Auto => camera.sorted_camera_index_for_target > 0,
            MsaaWriteback::Always => true,
        };

        if msaa.samples() > 1 && should_writeback {
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
