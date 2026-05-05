//! Default background motion vector prepass.
//!
//! When a camera has [`MotionVectorPrepass`] but no [`NoBackgroundMotionVectors`], this module
//! writes motion vectors for background pixels (depth == 0 in reversed-Z) based on camera
//! rotation, so that effects like TAA and motion blur work correctly on the background.
//!
//! This is a general solution that works for any background: skyboxes, atmospheric sky,
//! solid color backgrounds, etc.

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, QueryItem, With, Without},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
};
use bevy_log::warn;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_resource::{
        binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, CachedRenderPipelineId, CompareFunction, DepthStencilState,
        DownlevelFlags, FragmentState, MultisampleState, PipelineCache, RenderPipelineDescriptor,
        ShaderStages, SpecializedRenderPipeline, SpecializedRenderPipelines,
    },
    renderer::{RenderAdapter, RenderDevice},
    sync_component::SyncComponent,
    view::{Msaa, ViewUniform, ViewUniforms},
    GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::Shader;
use bevy_utils::prelude::default;

use crate::{
    core_3d::CORE_3D_DEPTH_FORMAT,
    prepass::{
        prepass_target_descriptors, MotionVectorPrepass, NormalPrepass, PreviousViewData,
        PreviousViewUniforms,
    },
    FullscreenShader,
};

/// When added to a camera with [`MotionVectorPrepass`], disables the automatic background motion
/// vector prepass.
///
/// By default, any camera with [`MotionVectorPrepass`] will automatically write camera-rotation
/// motion vectors for background pixels (those with no geometry, i.e. depth == 0 in reversed-Z).
/// Add this component to opt out, for example if you are writing custom background motion vectors
/// for your own effect.
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
pub struct NoBackgroundMotionVectors;

impl SyncComponent for NoBackgroundMotionVectors {
    type Target = Self;
}

impl ExtractComponent for NoBackgroundMotionVectors {
    type QueryData = Read<NoBackgroundMotionVectors>;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(_item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(NoBackgroundMotionVectors)
    }
}

/// Stores the background motion vectors pipeline ID on the camera entity. Used by the prepass node.
#[derive(Component)]
pub struct BackgroundMotionVectorsPipelineId(pub CachedRenderPipelineId);

/// Stores the background motion vectors bind group on the camera entity. Used by the prepass node.
#[derive(Component)]
pub struct BackgroundMotionVectorsBindGroup(pub BindGroup);

/// Plugin that writes camera-rotation motion vectors for background pixels on cameras with
/// [`MotionVectorPrepass`].
///
/// Add [`NoBackgroundMotionVectors`] to a camera to opt out.
#[derive(Default)]
pub struct BackgroundMotionVectorsPlugin;

impl BackgroundMotionVectorsPlugin {
    /// [`DownlevelFlags`] required for this plugin to function.
    pub fn required_downlevel_flags() -> DownlevelFlags {
        DownlevelFlags::INDEPENDENT_BLEND
    }
}

impl Plugin for BackgroundMotionVectorsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "background_motion_vectors.wgsl");
        app.register_type::<NoBackgroundMotionVectors>()
            .add_plugins(ExtractComponentPlugin::<NoBackgroundMotionVectors>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_gpu_resource::<PreviousViewUniforms>();
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_adapter = render_app.world().resource::<RenderAdapter>();
        let downlevel_flags = render_adapter.get_downlevel_capabilities().flags;
        if !downlevel_flags.contains(BackgroundMotionVectorsPlugin::required_downlevel_flags()) {
            warn!(
                "BackgroundMotionVectorsPlugin not loaded. GPU lacks support for required downlevel capability flags: {:?}.",
                BackgroundMotionVectorsPlugin::required_downlevel_flags().difference(downlevel_flags)
            );
            return;
        }

        render_app
            .init_gpu_resource::<SpecializedRenderPipelines<BackgroundMotionVectorsPipeline>>()
            .add_systems(RenderStartup, init_background_motion_vectors_pipeline)
            .add_systems(
                Render,
                (
                    prepare_background_motion_vectors_pipelines.in_set(RenderSystems::Prepare),
                    prepare_background_motion_vectors_bind_groups
                        .in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

#[derive(Resource)]
struct BackgroundMotionVectorsPipeline {
    bind_group_layout: BindGroupLayoutDescriptor,
    fullscreen_shader: FullscreenShader,
    fragment_shader: Handle<Shader>,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct BackgroundMotionVectorsPipelineKey {
    samples: u32,
    normal_prepass: bool,
}

fn init_background_motion_vectors_pipeline(
    mut commands: Commands,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(BackgroundMotionVectorsPipeline {
        bind_group_layout: BindGroupLayoutDescriptor::new(
            "background_motion_vectors_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<PreviousViewData>(true),
                ),
            ),
        ),
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(
            asset_server.as_ref(),
            "background_motion_vectors.wgsl"
        ),
    });
}

impl SpecializedRenderPipeline for BackgroundMotionVectorsPipeline {
    type Key = BackgroundMotionVectorsPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut targets = prepass_target_descriptors(key.normal_prepass, true, false);
        // The shader only outputs to attachment at location 1, set write mask of the other attachments to empty
        // to avoid WebGPU validation error "Color target has no corresponding fragment stage output but writeMask is not zero".
        for target in
            targets
                .iter_mut()
                .enumerate()
                .filter_map(|(i, t)| if i == 1 { None } else { t.as_mut() })
        {
            target.write_mask = bevy_render::render_resource::ColorWrites::empty();
        }

        RenderPipelineDescriptor {
            label: Some("background_motion_vectors_pipeline".into()),
            layout: vec![self.bind_group_layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: default(),
                bias: default(),
            }),
            multisample: MultisampleState {
                count: key.samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                targets,
                ..default()
            }),
            ..default()
        }
    }
}

fn prepare_background_motion_vectors_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BackgroundMotionVectorsPipeline>>,
    pipeline: Res<BackgroundMotionVectorsPipeline>,
    views: Query<
        (Entity, Has<NormalPrepass>, &Msaa),
        (
            With<MotionVectorPrepass>,
            Without<NoBackgroundMotionVectors>,
        ),
    >,
) {
    for (entity, normal_prepass, msaa) in &views {
        let id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BackgroundMotionVectorsPipelineKey {
                samples: msaa.samples(),
                normal_prepass,
            },
        );
        commands
            .entity(entity)
            .insert(BackgroundMotionVectorsPipelineId(id));
    }
}

fn prepare_background_motion_vectors_bind_groups(
    mut commands: Commands,
    pipeline: Res<BackgroundMotionVectorsPipeline>,
    view_uniforms: Res<ViewUniforms>,
    prev_view_uniforms: Res<PreviousViewUniforms>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    views: Query<
        Entity,
        (
            With<MotionVectorPrepass>,
            Without<NoBackgroundMotionVectors>,
        ),
    >,
) {
    for entity in &views {
        let (Some(view_binding), Some(prev_view_binding)) = (
            view_uniforms.uniforms.binding(),
            prev_view_uniforms.uniforms.binding(),
        ) else {
            continue;
        };
        let bind_group = render_device.create_bind_group(
            "background_motion_vectors_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout),
            &BindGroupEntries::sequential((view_binding, prev_view_binding)),
        );
        commands
            .entity(entity)
            .insert(BackgroundMotionVectorsBindGroup(bind_group));
    }
}
