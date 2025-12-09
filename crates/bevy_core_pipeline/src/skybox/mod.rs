use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::Exposure;
use bevy_ecs::{
    error::BevyError,
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
    variants: Variants<RenderPipeline, SkyboxPipelineSpecializer>,
}

fn init_skybox_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "skybox_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_cube(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<ViewUniform>(true).visibility(ShaderStages::VERTEX_FRAGMENT),
                uniform_buffer::<SkyboxUniforms>(true),
            ),
        ),
    );

    let shader = load_embedded_asset!(asset_server.as_ref(), "skybox.wgsl");

    let variants = Variants::new(
        SkyboxPipelineSpecializer,
        RenderPipelineDescriptor {
            label: Some("skybox_pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            vertex: VertexState {
                shader: shader.clone(),
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::R8Unorm, // placeholder.
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
            fragment: Some(FragmentState {
                shader,
                ..default()
            }),
            ..default()
        },
    );

    commands.insert_resource(SkyboxPipeline {
        bind_group_layout,
        variants,
    });
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, SpecializerKey)]
struct SkyboxPipelineKey {
    hdr: bool,
    samples: u32,
    depth_format: TextureFormat,
}

struct SkyboxPipelineSpecializer;

impl Specializer<RenderPipeline> for SkyboxPipelineSpecializer {
    type Key = SkyboxPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut RenderPipelineDescriptor,
    ) -> Result<Canonical<Self::Key>, BevyError> {
        descriptor.depth_stencil_mut()?.format = key.depth_format;
        descriptor.multisample.count = key.samples;
        descriptor.fragment_mut()?.set_target(
            0,
            ColorTargetState {
                format: if key.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
                blend: None,
                write_mask: ColorWrites::ALL,
            },
        );

        Ok(key)
    }
}

#[derive(Component)]
pub struct SkyboxPipelineId(pub CachedRenderPipelineId);

fn prepare_skybox_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipeline: ResMut<SkyboxPipeline>,
    views: Query<(Entity, &ExtractedView, &Msaa), With<Skybox>>,
) -> Result<(), BevyError> {
    for (entity, view, msaa) in &views {
        let pipeline_id = pipeline.variants.specialize(
            &pipeline_cache,
            SkyboxPipelineKey {
                hdr: view.hdr,
                samples: msaa.samples(),
                depth_format: CORE_3D_DEPTH_FORMAT,
            },
        )?;

        commands
            .entity(entity)
            .insert(SkyboxPipelineId(pipeline_id));
    }
    Ok(())
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
