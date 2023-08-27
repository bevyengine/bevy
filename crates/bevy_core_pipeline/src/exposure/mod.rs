use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_render::render_graph::RenderGraphApp;
use bevy_render::renderer::{RenderDevice, RenderQueue};
use bevy_render::view::{ViewTarget, ViewUniform};
use bevy_render::{render_resource::*, Render, RenderApp, RenderSet};

mod node;

pub use node::AutoExposureNode;

use crate::core_3d::graph::node::{AUTOEXPOSURE, END_MAIN_PASS, TONEMAPPING};
use crate::core_3d::CORE_3D;

const AUTO_EXPOSURE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 15323641124464253870);

pub struct AutoExposurePlugin;
impl Plugin for AutoExposurePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            AUTO_EXPOSURE_SHADER_HANDLE,
            "autoexposure.wgsl",
            Shader::from_wgsl
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<SpecializedComputePipelines<AutoExposurePipeline>>()
                .init_resource::<AutoExposureStorage>()
                .add_render_graph_node::<AutoExposureNode>(CORE_3D, AUTOEXPOSURE)
                .add_render_graph_edges(CORE_3D, &[END_MAIN_PASS, AUTOEXPOSURE, TONEMAPPING])
                .add_systems(
                    Render,
                    (
                        queue_view_auto_exposure_pipelines.in_set(RenderSet::Queue),
                        prepare_storage_buffer.in_set(RenderSet::Prepare),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<AutoExposurePipeline>();
        }
    }
}

#[derive(Resource)]
pub struct AutoExposurePipeline {
    bind_group: BindGroupLayout,
}

impl SpecializedComputePipeline for AutoExposurePipeline {
    type Key = usize;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        ComputePipelineDescriptor {
            label: Some("autoexposure pipeline".into()),
            layout: vec![self.bind_group.clone()],
            push_constant_ranges: Vec::new(),
            shader: AUTO_EXPOSURE_SHADER_HANDLE.typed(),
            shader_defs: Vec::new(),
            entry_point: match key {
                0 => "build_histogram",
                1 => "compute_exposure",
                _ => unreachable!(),
            }
            .into(),
        }
    }
}

impl FromWorld for AutoExposurePipeline {
    fn from_world(render_world: &mut World) -> Self {
        let entries = vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ];

        AutoExposurePipeline {
            bind_group: render_world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: Some("autoexposure_bind_group_layout"),
                    entries: &entries,
                }),
        }
    }
}

#[derive(Resource, Default)]
pub struct AutoExposureStorage(Option<Buffer>);

pub fn prepare_storage_buffer(
    render_device: Res<RenderDevice>,
    _render_queue: Res<RenderQueue>,
    mut storage: ResMut<AutoExposureStorage>,
) {
    if storage.0.is_none() {
        storage.0 = Some(render_device.create_buffer(&BufferDescriptor {
            label: Some("autoexposure_storage_buffer"),
            size: 64 * 4,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    }
}

#[derive(Component)]
pub struct ViewAutoExposurePipeline([CachedComputePipelineId; 2]);

pub fn queue_view_auto_exposure_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedComputePipelines<AutoExposurePipeline>>,
    upscaling_pipeline: Res<AutoExposurePipeline>,
    view_targets: Query<Entity, With<ViewTarget>>,
) {
    for entity in view_targets.iter() {
        commands.entity(entity).insert(ViewAutoExposurePipeline([
            pipelines.specialize(&pipeline_cache, &upscaling_pipeline, 0),
            pipelines.specialize(&pipeline_cache, &upscaling_pipeline, 1),
        ]));
    }
}
