use crate::{
    blit::{BlitPipeline, BlitPipelineKey},
    core_2d::graph::{Core2d, Node2d},
    core_3d::graph::{Core3d, Node3d},
};
use bevy_app::{App, Plugin};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_platform::collections::HashMap;
use bevy_render::{
    camera::ExtractedMainColorTargetReadsFrom,
    diagnostic::RecordDiagnostics,
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt, ViewNode, ViewNodeRunner},
    render_resource::*,
    renderer::RenderContext,
    texture::GpuImage,
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderSystems,
};

/// This enables [`MainColorTargetReadsFrom`] support for the `core_2d` and `core_3d` pipelines.
#[derive(Default)]
pub struct ColorTargetInputPlugin;

impl Plugin for ColorTargetInputPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.add_systems(
            Render,
            prepare_color_target_input_pipelines.in_set(RenderSystems::Prepare),
        );
        {
            render_app
                .add_render_graph_node::<ViewNodeRunner<ColorTargetInputNode>>(
                    Core2d,
                    Node2d::ColorTargetInput,
                )
                .add_render_graph_edge(Core2d, Node2d::ColorTargetInput, Node2d::StartMainPass);
        }
        {
            render_app
                .add_render_graph_node::<ViewNodeRunner<ColorTargetInputNode>>(
                    Core3d,
                    Node3d::ColorTargetInput,
                )
                .add_render_graph_edge(Core3d, Node3d::ColorTargetInput, Node3d::StartMainPass);
        }
    }
}

#[derive(Default)]
pub struct ColorTargetInputNode;

impl ViewNode for ColorTargetInputNode {
    type ViewQuery = (
        &'static ExtractedView,
        &'static ViewTarget,
        &'static ColorTargetInputBlitPipeline,
        &'static ExtractedMainColorTargetReadsFrom,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view, view_target, blit_pipeline_id, reads_from): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let blit_pipeline = world.resource::<BlitPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let images = world.resource::<RenderAssets<GpuImage>>();

        // Blend all inputs.
        for (input, input_config) in &reads_from.0 {
            let Some(source) = images.get(*input) else {
                continue;
            };
            let Some(pipeline) = blit_pipeline_id
                .0
                .get(&input_config.blend_state)
                .and_then(|id| pipeline_cache.get_render_pipeline(*id))
            else {
                continue;
            };

            let diagnostics = render_context.diagnostic_recorder();

            let pass_descriptor = RenderPassDescriptor {
                label: Some("color_target_input"),
                color_attachments: &[Some(if view.msaa_samples > 1 {
                    // Write to both multisampled texture and main texture.
                    RenderPassColorAttachment {
                        // If MSAA is enabled, then the sampled texture will always exist
                        view: view_target.sampled_main_texture_view().unwrap(),
                        depth_slice: None,
                        resolve_target: Some(view_target.main_texture_other_view()),
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }
                } else {
                    // Just write to main texture.
                    RenderPassColorAttachment {
                        view: view_target.main_texture_view(),
                        depth_slice: None,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: StoreOp::Store,
                        },
                    }
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            };

            let bind_group = blit_pipeline.create_bind_group(
                render_context.render_device(),
                &source.texture_view,
                pipeline_cache,
                true,
            );

            let mut render_pass = render_context
                .command_encoder()
                .begin_render_pass(&pass_descriptor);
            let pass_span = diagnostics.pass_span(&mut render_pass, "color_target_input");

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..3, 0..1);

            pass_span.end(&mut render_pass);
        }
        Ok(())
    }
}

#[derive(Component)]
pub struct ColorTargetInputBlitPipeline(HashMap<Option<BlendState>, CachedRenderPipelineId>);

fn prepare_color_target_input_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BlitPipeline>>,
    blit_pipeline: Res<BlitPipeline>,
    view_targets: Query<(Entity, &ExtractedView, &ExtractedMainColorTargetReadsFrom)>,
) {
    for (entity, view, reads_from) in view_targets.iter() {
        // Collect all blend state pipelines.
        let mut map = HashMap::new();
        for (_, input_config) in &reads_from.0 {
            map.entry(input_config.blend_state).or_insert_with(|| {
                let key = BlitPipelineKey {
                    texture_format: view.color_target_format,
                    samples: view.msaa_samples,
                    blend_state: input_config.blend_state,
                    filtering: true,
                };

                pipelines.specialize(&pipeline_cache, &blit_pipeline, key)
            });
        }
        commands
            .entity(entity)
            .insert(ColorTargetInputBlitPipeline(map));
    }
}
