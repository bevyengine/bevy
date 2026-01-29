use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::Exposure;
use bevy_ecs::{
    prelude::{Component, Entity},
    query::With,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_image::BevyDefault;
use bevy_light::Skybox;
use bevy_math::Mat4;
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{sampler, texture_cube, uniform_buffer},
        *,
    },
    renderer::RenderDevice,
    sync_world::RenderEntity,
    texture::GpuImage,
    view::{ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniforms},
    Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::Shader;
use bevy_transform::components::Transform;
use bevy_utils::default;
use prepass::SkyboxPrepassPipeline;

use crate::{
    core_3d::CORE_3D_DEPTH_FORMAT, prepass::PreviousViewUniforms,
    skybox::prepass::init_skybox_prepass_pipeline,
};

pub mod prepass;

pub struct SkyboxPlugin;

impl Plugin for SkyboxPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "skybox.wgsl");
        embedded_asset!(app, "skybox_prepass.wgsl");

        app.add_plugins(UniformComponentPlugin::<SkyboxUniforms>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<SkyboxPipeline>>()
            .init_resource::<SpecializedRenderPipelines<SkyboxPrepassPipeline>>()
            .init_resource::<PreviousViewUniforms>()
            .add_systems(ExtractSchedule, extract_skybox)
            .add_systems(
                RenderStartup,
                (init_skybox_pipeline, init_skybox_prepass_pipeline),
            )
            .add_systems(
                Render,
                (
                    prepare_skybox_pipelines.in_set(RenderSystems::Prepare),
                    prepass::prepare_skybox_prepass_pipelines.in_set(RenderSystems::Prepare),
                    prepare_skybox_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                    prepass::prepare_skybox_prepass_bind_groups
                        .in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

// This is needed because of the orphan rule not allowing implementing
// foreign trait ExtractComponent on foreign type Skybox
pub fn extract_skybox(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(RenderEntity, &Skybox, Option<&Exposure>)>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, skybox, exposure) in &query {
        let exposure = exposure
            .map(Exposure::exposure)
            .unwrap_or_else(|| Exposure::default().exposure());
        let uniforms = SkyboxUniforms {
            brightness: skybox.brightness * exposure,
            transform: Transform::from_rotation(skybox.rotation.inverse()).to_matrix(),
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            _webgl2_padding_8b: 0,
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            _webgl2_padding_12b: 0,
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            _webgl2_padding_16b: 0,
        };
        values.push((entity, (skybox.clone(), uniforms)));
    }
    *previous_len = values.len();
    commands.try_insert_batch(values);
}

// TODO: Replace with a push constant once WebGPU gets support for that
#[derive(Component, ShaderType, Clone)]
pub struct SkyboxUniforms {
    brightness: f32,
    transform: Mat4,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _webgl2_padding_8b: u32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _webgl2_padding_12b: u32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _webgl2_padding_16b: u32,
}

#[derive(Resource)]
struct SkyboxPipeline {
    bind_group_layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
}

impl SkyboxPipeline {
    fn new(shader: Handle<Shader>) -> Self {
        Self {
            bind_group_layout: BindGroupLayoutDescriptor::new(
                "skybox_bind_group_layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_cube(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        uniform_buffer::<ViewUniform>(true)
                            .visibility(ShaderStages::VERTEX_FRAGMENT),
                        uniform_buffer::<SkyboxUniforms>(true),
                    ),
                ),
            ),
            shader,
        }
    }
}

fn init_skybox_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    let shader = load_embedded_asset!(asset_server.as_ref(), "skybox.wgsl");
    commands.insert_resource(SkyboxPipeline::new(shader));
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct SkyboxPipelineKey {
    texture_format: TextureFormat,
    samples: u32,
    depth_format: TextureFormat,
}

impl SpecializedRenderPipeline for SkyboxPipeline {
    type Key = SkyboxPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("skybox_pipeline".into()),
            layout: vec![self.bind_group_layout.clone()],
            vertex: VertexState {
                shader: self.shader.clone(),
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: key.depth_format,
                depth_write_enabled: false,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    // BlendState::REPLACE is not needed here, and None will be potentially much faster in some cases.
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

#[derive(Component)]
pub struct SkyboxPipelineId(pub CachedRenderPipelineId);

fn prepare_skybox_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SkyboxPipeline>>,
    pipeline: Res<SkyboxPipeline>,
    views: Query<(Entity, &ExtractedView), With<Skybox>>,
) {
    for (entity, view) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            SkyboxPipelineKey {
                texture_format: view.color_target_format,
                samples: view.msaa_samples,
                depth_format: CORE_3D_DEPTH_FORMAT,
            },
        );

        commands
            .entity(entity)
            .insert(SkyboxPipelineId(pipeline_id));
    }
}

#[derive(Component)]
pub struct SkyboxBindGroup(pub (BindGroup, u32));

fn prepare_skybox_bind_groups(
    mut commands: Commands,
    pipeline: Res<SkyboxPipeline>,
    view_uniforms: Res<ViewUniforms>,
    skybox_uniforms: Res<ComponentUniforms<SkyboxUniforms>>,
    images: Res<RenderAssets<GpuImage>>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    views: Query<(Entity, &Skybox, &DynamicUniformIndex<SkyboxUniforms>)>,
) {
    for (entity, skybox, skybox_uniform_index) in &views {
        if let (Some(skybox), Some(view_uniforms), Some(skybox_uniforms)) = (
            images.get(&skybox.image),
            view_uniforms.uniforms.binding(),
            skybox_uniforms.binding(),
        ) {
            let bind_group = render_device.create_bind_group(
                "skybox_bind_group",
                &pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout),
                &BindGroupEntries::sequential((
                    &skybox.texture_view,
                    &skybox.sampler,
                    view_uniforms,
                    skybox_uniforms,
                )),
            );

            commands
                .entity(entity)
                .insert(SkyboxBindGroup((bind_group, skybox_uniform_index.index())));
        }
    }
}
