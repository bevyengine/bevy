use bevy_app::{App, Plugin};
use bevy_color::LinearRgba;
use bevy_core_pipeline::{
    blit::{BlitPipeline, BlitPipelineKey},
    core_2d::graph::{Core2d, Node2d},
    core_3d::graph::{Core3d, Node3d},
};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt, ViewNode, ViewNodeRunner},
    render_resource::*,
    renderer::RenderContext,
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
        render_context: &mut RenderContext<'w>,
        (target, blit_pipeline_id, msaa): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        if *msaa == Msaa::Off {
            return Ok(());
        }

        let blit_pipeline = world.resource::<BlitPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(pipeline) = pipeline_cache.get_render_pipeline(blit_pipeline_id.0) else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

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
            blit_pipeline.create_bind_group(render_context.render_device(), post_process.source);

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);
        let pass_span = diagnostics.pass_span(&mut render_pass, "msaa_writeback");

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        pass_span.end(&mut render_pass);

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
