//! Skybox prepass.

use bevy_asset::Handle;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Has, With},
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::{
        binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayout,
        BindGroupLayoutEntries, CachedRenderPipelineId, CompareFunction, DepthStencilState,
        FragmentState, PipelineCache, RenderPipelineDescriptor, Shader, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines, VertexState,
    },
    renderer::RenderDevice,
    view::{ViewUniform, ViewUniforms},
};
use bevy_utils::prelude::default;
use bitflags::bitflags;

use crate::{
    core_3d::CORE_3D_DEPTH_FORMAT,
    prepass::{
        prepass_target_descriptors, DeferredPrepass, MotionVectorPrepass, NormalPrepass,
        PreviousViewData, PreviousViewUniforms,
    },
    Skybox,
};

pub const SKYBOX_PREPASS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(376510055324461154);

#[derive(Resource)]
pub struct SkyboxPrepassPipeline {
    bind_group_layout: BindGroupLayout,
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SkyboxPrepassPipelineKey: u8 {
        const NORMAL_PREPASS    = 0x1;
        const DEFERRED_PREPASS  = 0x2;
    }
}

#[derive(Component)]
pub struct RenderSkyboxPrepassPipeline(pub CachedRenderPipelineId);

#[derive(Component)]
pub struct SkyboxPrepassBindGroup(pub BindGroup);

impl FromWorld for SkyboxPrepassPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        Self {
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
        }
    }
}

impl SpecializedRenderPipeline for SkyboxPrepassPipeline {
    type Key = SkyboxPrepassPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("skybox_prepass_pipeline".into()),
            layout: vec![],
            push_constant_ranges: vec![],
            vertex: VertexState {
                shader: SKYBOX_PREPASS_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "vertex_main".into(),
                buffers: vec![],
            },
            primitive: default(),
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: default(),
                bias: default(),
            }),
            multisample: default(),
            fragment: Some(FragmentState {
                shader: SKYBOX_PREPASS_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "fragment_main".into(),
                targets: prepass_target_descriptors(
                    key.contains(SkyboxPrepassPipelineKey::NORMAL_PREPASS),
                    true,
                    key.contains(SkyboxPrepassPipelineKey::DEFERRED_PREPASS),
                ),
            }),
        }
    }
}

pub fn prepare_skybox_prepass_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SkyboxPrepassPipeline>>,
    pipeline: Res<SkyboxPrepassPipeline>,
    views: Query<
        (Entity, Has<NormalPrepass>, Has<DeferredPrepass>),
        (With<Skybox>, With<MotionVectorPrepass>),
    >,
) {
    for (entity, normal_prepass, deferred_prepass) in &views {
        let mut pipeline_key = SkyboxPrepassPipelineKey::empty();
        pipeline_key.set(SkyboxPrepassPipelineKey::NORMAL_PREPASS, normal_prepass);
        pipeline_key.set(SkyboxPrepassPipelineKey::DEFERRED_PREPASS, deferred_prepass);

        let render_skybox_prepass_pipeline =
            pipelines.specialize(&pipeline_cache, &pipeline, pipeline_key);
        commands
            .entity(entity)
            .insert(RenderSkyboxPrepassPipeline(render_skybox_prepass_pipeline));
    }
}

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
