use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, AssetApp, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_render::extract_component::ExtractComponentPlugin;
use bevy_render::render_asset::RenderAssetPlugin;
use bevy_render::render_resource::Shader;
use bevy_render::ExtractSchedule;
use bevy_render::{
    render_graph::RenderGraphApp,
    render_resource::{
        Buffer, BufferDescriptor, BufferUsages, PipelineCache, SpecializedComputePipelines,
    },
    renderer::RenderDevice,
    Render, RenderApp, RenderSet,
};

mod buffers;
mod compensation_curve;
mod node;
mod pipeline;
mod settings;

use buffers::{extract_buffers, prepare_buffers, AutoExposureBuffers};
pub use compensation_curve::{AutoExposureCompensationCurve, AutoExposureCompensationCurveError};
use node::AutoExposureNode;
use pipeline::{
    AutoExposurePass, AutoExposurePipeline, ViewAutoExposurePipeline, METERING_SHADER_HANDLE,
};
pub use settings::AutoExposureSettings;

use crate::auto_exposure::compensation_curve::GpuAutoExposureCompensationCurve;
use crate::core_3d::graph::{Core3d, Node3d};

/// Plugin for the auto exposure feature.
///
/// See [`AutoExposureSettings`] for more details.
pub struct AutoExposurePlugin;

#[derive(Resource)]
struct AutoExposureResources {
    histogram: Buffer,
}

impl Plugin for AutoExposurePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            METERING_SHADER_HANDLE,
            "auto_exposure.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(RenderAssetPlugin::<GpuAutoExposureCompensationCurve>::default())
            .register_type::<AutoExposureCompensationCurve>()
            .init_asset::<AutoExposureCompensationCurve>()
            .register_asset_reflect::<AutoExposureCompensationCurve>();
        app.world_mut()
            .resource_mut::<Assets<AutoExposureCompensationCurve>>()
            .insert(&Handle::default(), AutoExposureCompensationCurve::default());

        app.register_type::<AutoExposureSettings>();
        app.add_plugins(ExtractComponentPlugin::<AutoExposureSettings>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedComputePipelines<AutoExposurePipeline>>()
            .init_resource::<AutoExposureBuffers>()
            .add_systems(ExtractSchedule, extract_buffers)
            .add_systems(
                Render,
                (
                    prepare_buffers.in_set(RenderSet::Prepare),
                    queue_view_auto_exposure_pipelines.in_set(RenderSet::Queue),
                ),
            )
            .add_render_graph_node::<AutoExposureNode>(Core3d, node::AutoExposure)
            .add_render_graph_edges(
                Core3d,
                (Node3d::EndMainPass, node::AutoExposure, Node3d::Tonemapping),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<AutoExposurePipeline>();
        render_app.init_resource::<AutoExposureResources>();
    }
}

impl FromWorld for AutoExposureResources {
    fn from_world(world: &mut World) -> Self {
        Self {
            histogram: world
                .resource::<RenderDevice>()
                .create_buffer(&BufferDescriptor {
                    label: Some("histogram buffer"),
                    size: pipeline::HISTOGRAM_BIN_COUNT * 4,
                    usage: BufferUsages::STORAGE,
                    mapped_at_creation: false,
                }),
        }
    }
}

fn queue_view_auto_exposure_pipelines(
    mut commands: Commands,
    pipeline_cache: ResMut<PipelineCache>,
    mut compute_pipelines: ResMut<SpecializedComputePipelines<AutoExposurePipeline>>,
    pipeline: Res<AutoExposurePipeline>,
    view_targets: Query<(Entity, &AutoExposureSettings)>,
) {
    for (entity, settings) in view_targets.iter() {
        let histogram_pipeline =
            compute_pipelines.specialize(&pipeline_cache, &pipeline, AutoExposurePass::Histogram);
        let average_pipeline =
            compute_pipelines.specialize(&pipeline_cache, &pipeline, AutoExposurePass::Average);

        commands.entity(entity).insert(ViewAutoExposurePipeline {
            histogram_pipeline,
            mean_luminance_pipeline: average_pipeline,
            compensation_curve: settings.compensation_curve.clone(),
            metering_mask: settings.metering_mask.clone(),
        });
    }
}
