use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Handle};
use bevy_ecs::prelude::*;
use bevy_ecs::{query::QueryItem, system::lifetimeless::Read};
use bevy_math::{vec2, Vec2};
use bevy_reflect::prelude::*;
use bevy_render::render_resource::Shader;
use bevy_render::ExtractSchedule;
use bevy_render::{
    camera::Camera,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::RenderGraphApp,
    render_resource::{
        Buffer, BufferDescriptor, BufferUsages, Extent3d, PipelineCache,
        SpecializedComputePipelines, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsages, TextureView, TextureViewDescriptor,
    },
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    Extract, Render, RenderApp, RenderSet,
};
use bevy_utils::{default, HashMap};

mod node;
mod pipeline;

use node::AutoExposureNode;
use pipeline::{
    AutoExposureParams, AutoExposurePipeline, Pass, ViewAutoExposurePipeline,
    METERING_SHADER_HANDLE,
};

use crate::core_3d::graph::{Core3d, Node3d};

/// Plugin for the auto exposure feature.
pub struct AutoExposurePlugin;

/// Component that enables auto exposure for a camera.
#[derive(Component, Clone, Reflect)]
#[reflect(Component)]
pub struct AutoExposure {
    /// The minimum exposure value for the camera.
    pub min: f32,
    /// The maximum exposure value for the camera.
    pub max: f32,
    /// The percentage of darkest pixels to ignore when metering.
    pub low_percent: u32,
    /// The percentage of brightest pixels to ignore when metering.
    pub high_percent: u32,
    /// The speed at which the exposure adapts from dark to bright scenes.
    pub speed_up: f32,
    /// The speed at which the exposure adapts from bright to dark scenes.
    pub speed_down: f32,
    /// The mask to apply when metering. Bright spots on the mask will contribute more to the
    /// metering, and dark spots will contribute less.
    pub metering_mask: Handle<Image>,
    /// Exposure compensation curve to apply after metering.
    /// The X axis corresponds to the measured exposure, and the Y axis corresponds to the
    /// exposure compensation to apply.
    /// Note that the compensation values are clamped between -8 and +8.
    pub compensation_curve: Vec<Vec2>,
}

#[derive(Resource)]
struct AutoExposureResources {
    histogram: Buffer,
}

struct ExtractedAutoExposureBuffer {
    min: f32,
    max: f32,
    compensation_curve: Vec<Vec2>,
}

#[derive(Resource)]
struct ExtractedAutoExposureBuffers {
    changed: Vec<(Entity, ExtractedAutoExposureBuffer)>,
    removed: Vec<Entity>,
}

struct AutoExposureBuffer {
    exposure: Buffer,
    compensation_curve: TextureView,
}

#[derive(Resource, Default)]
struct AutoExposureBuffers {
    buffers: HashMap<Entity, AutoExposureBuffer>,
}

impl Default for AutoExposure {
    fn default() -> Self {
        Self {
            min: -8.0,
            max: 8.0,
            low_percent: 60,
            high_percent: 95,
            speed_up: 3.0,
            speed_down: 1.0,
            metering_mask: default(),
            compensation_curve: vec![vec2(-8.0, 0.0), vec2(8.0, 0.0)],
        }
    }
}

impl ExtractComponent for AutoExposure {
    type QueryData = Read<Self>;
    type QueryFilter = With<Camera>;
    type Out = Self;

    fn extract_component(item: QueryItem<'_, Self::QueryData>) -> Option<Self> {
        Some(item.clone())
    }
}

impl Plugin for AutoExposurePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            METERING_SHADER_HANDLE,
            "auto_exposure.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<AutoExposure>();
        app.add_plugins(ExtractComponentPlugin::<AutoExposure>::default());

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedComputePipelines<AutoExposurePipeline>>()
            .init_resource::<AutoExposureBuffers>()
            .add_systems(ExtractSchedule, extract_auto_exposure_buffers)
            .add_systems(
                Render,
                (
                    prepare_auto_exposure_buffers.in_set(RenderSet::Prepare),
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

fn extract_auto_exposure_buffers(
    mut commands: Commands,
    changed: Extract<Query<(Entity, &AutoExposure), Changed<AutoExposure>>>,
    mut removed: Extract<RemovedComponents<AutoExposure>>,
) {
    commands.insert_resource(ExtractedAutoExposureBuffers {
        changed: changed
            .iter()
            .map(|(entity, auto_exposure)| {
                (
                    entity,
                    ExtractedAutoExposureBuffer {
                        min: auto_exposure.min,
                        max: auto_exposure.max,
                        compensation_curve: auto_exposure.compensation_curve.clone(),
                    },
                )
            })
            .collect(),
        removed: removed.read().collect(),
    });
}

fn prepare_auto_exposure_buffers(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut extracted: ResMut<ExtractedAutoExposureBuffers>,
    mut buffers: ResMut<AutoExposureBuffers>,
) {
    for (entity, buffer) in extracted.changed.drain(..) {
        let exposure = device.create_buffer(&BufferDescriptor {
            label: Some("auto exposure state buffer"),
            size: 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let compensation_curve_desc = TextureDescriptor {
            label: Some("auto exposure compensation curve"),
            size: Extent3d {
                width: 256,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D1,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[TextureFormat::R8Unorm],
        };

        let mut data = [0u8; 256];
        let y_to_val = |y: f32| ((y + 8.0) / 16.0 * 255.0).clamp(0.0, 255.0) as u8;
        for i in 1..256 {
            let ev = buffer.min + ((i - 1) as f32 / 254.0) * (buffer.max - buffer.min);

            if let Some(j) = buffer
                .compensation_curve
                .iter()
                .enumerate()
                .find_map(|(i, v)| if v.x >= ev { Some(i) } else { None })
            {
                if j == 0 {
                    data[i] = y_to_val(buffer.compensation_curve[0].y);
                    continue;
                }

                let v0 = buffer.compensation_curve[j - 1];
                let v1 = buffer.compensation_curve[j];
                if v0.x == v1.x {
                    data[i] = y_to_val(v0.y);
                    continue;
                }

                let t = (ev - v0.x) / (v1.x - v0.x);
                let y = v0.y + t * (v1.y - v0.y);
                data[i] = y_to_val(y);
                continue;
            }

            data[i] = y_to_val(buffer.compensation_curve[buffer.compensation_curve.len() - 1].y);
        }
        data[0] = data[1];

        let compensation_curve = device
            .create_texture_with_data(&queue, &compensation_curve_desc, default(), &data)
            .create_view(&TextureViewDescriptor {
                label: Some("auto exposure compensation curve view"),
                ..default()
            });

        buffers.buffers.insert(
            entity,
            AutoExposureBuffer {
                exposure,
                compensation_curve,
            },
        );
    }

    for entity in extracted.removed.drain(..) {
        buffers.buffers.remove(&entity);
    }
}

fn queue_view_auto_exposure_pipelines(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut compute_pipelines: ResMut<SpecializedComputePipelines<AutoExposurePipeline>>,
    pipeline: Res<AutoExposurePipeline>,
    // time: Res<Time>,
    buffers: Res<AutoExposureBuffers>,
    view_targets: Query<(Entity, &AutoExposure)>,
) {
    for (entity, auto_exposure) in view_targets.iter() {
        let histogram_pipeline =
            compute_pipelines.specialize(&mut pipeline_cache, &pipeline, Pass::Histogram);
        let average_pipeline =
            compute_pipelines.specialize(&mut pipeline_cache, &pipeline, Pass::Average);

        let Some(buffer) = buffers.buffers.get(&entity) else {
            continue;
        };

        let delta = 1.0 / 144.0;

        commands.entity(entity).insert(ViewAutoExposurePipeline {
            histogram_pipeline,
            mean_luminance_pipeline: average_pipeline,
            state: buffer.exposure.clone(),
            compensation_curve: buffer.compensation_curve.clone(),
            params: AutoExposureParams {
                min_log_lum: auto_exposure.min,
                inv_log_lum_range: 1.0 / (auto_exposure.max - auto_exposure.min),
                log_lum_range: auto_exposure.max - auto_exposure.min,
                low_percent: auto_exposure.low_percent,
                high_percent: auto_exposure.high_percent,
                speed_up: auto_exposure.speed_up * delta,
                speed_down: auto_exposure.speed_down * delta,
            },
            metering_mask: auto_exposure.metering_mask.clone(),
        });
    }
}
