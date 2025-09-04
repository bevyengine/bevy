//! Adds motion vector support to skyboxes. See [`SkyboxPrepassPipeline`] for details.

use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, With},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_render::{
    render_resource::{
        binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayout,
        BindGroupLayoutEntries, CachedRenderPipelineId, CompareFunction, DepthStencilState,
        FragmentState, MultisampleState, PipelineCache, RenderPipelineDescriptor, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines,
    },
    renderer::RenderDevice,
    view::{Msaa, ViewUniform, ViewUniforms},
};
use bevy_shader::Shader;
use bevy_utils::prelude::default;

use crate::{
    core_3d::CORE_3D_DEPTH_FORMAT,
    prepass::{
        prepass_target_descriptors, MotionVectorPrepass, NormalPrepass, PreviousViewData,
        PreviousViewUniforms,
    },
    FullscreenShader, Skybox,
};

/// This pipeline writes motion vectors to the prepass for all [`Skybox`]es.
///
/// This allows features like motion blur and TAA to work correctly on the skybox. Without this, for
/// example, motion blur would not be applied to the skybox when the camera is rotated and motion
/// blur is enabled.
#[derive(Resource)]
pub struct SkyboxPrepassPipeline {
    bind_group_layout: BindGroupLayout,
    fullscreen_shader: FullscreenShader,
    fragment_shader: Handle<Shader>,
}

/// Used to specialize the [`SkyboxPrepassPipeline`].
#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct SkyboxPrepassPipelineKey {
    samples: u32,
    normal_prepass: bool,
}

/// Stores the ID for a camera's specialized pipeline, so it can be retrieved from the
/// [`PipelineCache`].
#[derive(Component)]
pub struct RenderSkyboxPrepassPipeline(pub CachedRenderPipelineId);

/// Stores the [`SkyboxPrepassPipeline`] bind group for a camera. This is later used by the prepass
/// render graph node to add this binding to the prepass's render pass.
#[derive(Component)]
pub struct SkyboxPrepassBindGroup(pub BindGroup);

pub fn init_skybox_prepass_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(SkyboxPrepassPipeline {
        bind_group_layout: render_device.create_bind_group_layout(
            "skybox_prepass_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<PreviousViewData>(true),
                ),
            ),
        ),
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "skybox_prepass.wgsl"),
    });
}

impl SpecializedRenderPipeline for SkyboxPrepassPipeline {
    type Key = SkyboxPrepassPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("skybox_prepass_pipeline".into()),
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

/// Specialize and cache the [`SkyboxPrepassPipeline`] for each camera with a [`Skybox`].
pub fn prepare_skybox_prepass_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SkyboxPrepassPipeline>>,
    pipeline: Res<SkyboxPrepassPipeline>,
    views: Query<(Entity, Has<NormalPrepass>, &Msaa), (With<Skybox>, With<MotionVectorPrepass>)>,
) {
    for (entity, normal_prepass, msaa) in &views {
        let pipeline_key = SkyboxPrepassPipelineKey {
            samples: msaa.samples(),
            normal_prepass,
        };

        let render_skybox_prepass_pipeline =
            pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key);
        commands
            .entity(entity)
            .insert(RenderSkyboxPrepassPipeline(render_skybox_prepass_pipeline));
    }
}

/// Creates the required bind groups for the [`SkyboxPrepassPipeline`]. This binds the view uniforms
/// from the CPU for access in the prepass shader on the GPU, allowing us to compute camera motion
/// between frames. This is then stored in the [`SkyboxPrepassBindGroup`] component on the camera.
pub fn prepare_skybox_prepass_bind_groups(
    mut commands: Commands,
    pipeline: Res<SkyboxPrepassPipeline>,
    view_uniforms: Res<ViewUniforms>,
    prev_view_uniforms: Res<PreviousViewUniforms>,
    render_device: Res<RenderDevice>,
    views: Query<Entity, (With<Skybox>, With<MotionVectorPrepass>)>,
) {
    for entity in &views {
        let (Some(view_uniforms), Some(prev_view_uniforms)) = (
            view_uniforms.uniforms.binding(),
            prev_view_uniforms.uniforms.binding(),
        ) else {
            continue;
        };
        let bind_group = render_device.create_bind_group(
            "skybox_prepass_bind_group",
            &pipeline.bind_group_layout,
            &BindGroupEntries::sequential((view_uniforms, prev_view_uniforms)),
        );

        commands
            .entity(entity)
            .insert(SkyboxPrepassBindGroup(bind_group));
    }
}
