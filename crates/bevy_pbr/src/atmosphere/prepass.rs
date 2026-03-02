//! Motion vector prepass support for atmosphere rendering.
//!
//! When a camera has both [`Atmosphere`](ExtractedAtmosphere) and [`MotionVectorPrepass`] but no
//! [`Skybox`], this module writes motion vectors for sky pixels (depth == 0) so that effects like
//! TAA and motion blur work correctly on the atmospheric sky.
//!
//! The pipeline is structurally identical to [`SkyboxPrepassPipeline`]. Both compute camera
//! relative motion vectors because the sky is at infinity.
//!
//! [`SkyboxPrepassPipeline`]: bevy_core_pipeline::skybox::prepass::SkyboxPrepassPipeline

use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::{
    core_3d::CORE_3D_DEPTH_FORMAT,
    prepass::{
        prepass_target_descriptors, MotionVectorPrepass, NormalPrepass, PreviousViewData,
        PreviousViewUniforms,
    },
    skybox::prepass::{RenderSkyboxPrepassPipeline, SkyboxPrepassBindGroup},
    FullscreenShader,
};
use bevy_ecs::{
    entity::Entity,
    query::{Has, With, Without},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_light::Skybox;
use bevy_render::{
    render_resource::{
        binding_types::uniform_buffer, BindGroupEntries, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, CompareFunction, DepthStencilState, FragmentState,
        MultisampleState, PipelineCache, RenderPipelineDescriptor, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines,
    },
    renderer::RenderDevice,
    view::{Msaa, ViewUniform, ViewUniforms},
};
use bevy_shader::Shader;
use bevy_utils::prelude::default;

use super::ExtractedAtmosphere;

/// Pipeline for writing motion vectors to the prepass for cameras with [`Atmosphere`] but no
/// [`Skybox`].
///
/// When a [`Skybox`] is also present, the existing [`SkyboxPrepassPipeline`] already covers
/// sky-pixel motion vectors, so this pipeline is skipped.
///
/// [`SkyboxPrepassPipeline`]: bevy_core_pipeline::skybox::prepass::SkyboxPrepassPipeline
#[derive(Resource)]
pub struct AtmospherePrepassPipeline {
    bind_group_layout: BindGroupLayoutDescriptor,
    fullscreen_shader: FullscreenShader,
    fragment_shader: Handle<Shader>,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct AtmospherePrepassPipelineKey {
    samples: u32,
    normal_prepass: bool,
}

pub fn init_atmosphere_prepass_pipeline(
    mut commands: Commands,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(AtmospherePrepassPipeline {
        bind_group_layout: BindGroupLayoutDescriptor::new(
            "atmosphere_prepass_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<PreviousViewData>(true),
                ),
            ),
        ),
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "atmosphere_prepass.wgsl"),
    });
}

impl SpecializedRenderPipeline for AtmospherePrepassPipeline {
    type Key = AtmospherePrepassPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("atmosphere_prepass_pipeline".into()),
            layout: vec![self.bind_group_layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
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
                targets: prepass_target_descriptors(key.normal_prepass, true, false),
                ..default()
            }),
            ..default()
        }
    }
}

/// Specialize and cache the [`AtmospherePrepassPipeline`] for each camera that has [`Atmosphere`]
/// and [`MotionVectorPrepass`] but no [`Skybox`] (the skybox prepass already covers that case).
///
/// The resulting [`CachedRenderPipelineId`] is stored as a [`RenderSkyboxPrepassPipeline`]
/// component, so the existing prepass node picks it up without any changes to `bevy_core_pipeline`.
pub fn prepare_atmosphere_prepass_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<AtmospherePrepassPipeline>>,
    pipeline: Res<AtmospherePrepassPipeline>,
    views: Query<
        (Entity, Has<NormalPrepass>, &Msaa),
        (
            With<ExtractedAtmosphere>,
            With<MotionVectorPrepass>,
            Without<Skybox>,
        ),
    >,
) {
    for (entity, normal_prepass, msaa) in &views {
        let id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            AtmospherePrepassPipelineKey {
                samples: msaa.samples(),
                normal_prepass,
            },
        );
        commands
            .entity(entity)
            .insert(RenderSkyboxPrepassPipeline(id));
    }
}

/// Creates the bind groups required by the [`AtmospherePrepassPipeline`], storing them as
/// [`SkyboxPrepassBindGroup`] so the prepass node can use them.
pub fn prepare_atmosphere_prepass_bind_groups(
    mut commands: Commands,
    pipeline: Res<AtmospherePrepassPipeline>,
    view_uniforms: Res<ViewUniforms>,
    prev_view_uniforms: Res<PreviousViewUniforms>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    views: Query<
        Entity,
        (
            With<ExtractedAtmosphere>,
            With<MotionVectorPrepass>,
            Without<Skybox>,
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
            "atmosphere_prepass_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout),
            &BindGroupEntries::sequential((view_binding, prev_view_binding)),
        );
        commands
            .entity(entity)
            .insert(SkyboxPrepassBindGroup(bind_group));
    }
}
