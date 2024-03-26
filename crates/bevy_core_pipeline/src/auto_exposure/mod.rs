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
use bevy_time::Time;

mod compensation_curve;
mod node;
mod pipeline;
mod settings;
mod state;

pub use compensation_curve::AutoExposureCompensationCurve;
use node::AutoExposureNode;
use pipeline::{
    AutoExposurePipeline, AutoExposureUniform, Pass, ViewAutoExposurePipeline,
    METERING_SHADER_HANDLE,
};
pub use settings::AutoExposureSettings;
use state::{extract_state_buffers, prepare_state_buffers, AutoExposureStateBuffers};

use crate::core_3d::graph::{Core3d, Node3d};

/// Plugin for the auto exposure feature.
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

        app.add_plugins(RenderAssetPlugin::<AutoExposureCompensationCurve>::default())
            .register_type::<AutoExposureCompensationCurve>()
            .init_asset::<AutoExposureCompensationCurve>()
            .register_asset_reflect::<AutoExposureCompensationCurve>();
        app.world
            .resource_mut::<Assets<AutoExposureCompensationCurve>>()
            .insert(Handle::default(), AutoExposureCompensationCurve::default());

        app.register_type::<AutoExposureSettings>();
        app.add_plugins(ExtractComponentPlugin::<AutoExposureSettings>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedComputePipelines<AutoExposurePipeline>>()
            .init_resource::<AutoExposureStateBuffers>()
            .add_systems(ExtractSchedule, extract_state_buffers)
            .add_systems(
                Render,
                (
                    prepare_state_buffers.in_set(RenderSet::Prepare),
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
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
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
                    size: 256 * 4,
                    usage: BufferUsages::STORAGE,
                    mapped_at_creation: false,
                }),
        }
    }
}

fn queue_view_auto_exposure_pipelines(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut compute_pipelines: ResMut<SpecializedComputePipelines<AutoExposurePipeline>>,
    pipeline: Res<AutoExposurePipeline>,
    buffers: Res<AutoExposureStateBuffers>,
    time: Res<Time>,
    view_targets: Query<(Entity, &AutoExposureSettings)>,
) {
    for (entity, settings) in view_targets.iter() {
        let histogram_pipeline =
            compute_pipelines.specialize(&mut pipeline_cache, &pipeline, Pass::Histogram);
        let average_pipeline =
            compute_pipelines.specialize(&mut pipeline_cache, &pipeline, Pass::Average);

        let Some(buffer) = buffers.buffers.get(&entity) else {
            continue;
        };

        commands.entity(entity).insert(ViewAutoExposurePipeline {
            histogram_pipeline,
            mean_luminance_pipeline: average_pipeline,
            state: buffer.state.clone(),
            compensation_curve: settings.compensation_curve.clone(),
            uniform: AutoExposureUniform {
                min_log_lum: settings.min,
                inv_log_lum_range: 1.0 / (settings.max - settings.min),
                log_lum_range: settings.max - settings.min,
                low_percent: settings.low_percent,
                high_percent: settings.high_percent,
                speed_up: settings.speed_brighten * time.delta_seconds(),
                speed_down: settings.speed_darken * time.delta_seconds(),
            },
            metering_mask: settings.metering_mask.clone(),
        });
    }
}
