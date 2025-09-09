use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::Exposure;
use bevy_ecs::{
    prelude::{Component, Entity},
    query::{QueryItem, With},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::{BevyDefault, Image};
use bevy_math::{Mat4, Quat};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{sampler, texture_cube, uniform_buffer},
        *,
    },
    renderer::RenderDevice,
    texture::GpuImage,
    view::{ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniforms},
    Render, RenderApp, RenderStartup, RenderSystems,
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

        app.add_plugins((
            ExtractComponentPlugin::<Skybox>::default(),
            UniformComponentPlugin::<SkyboxUniforms>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<SkyboxPipeline>>()
            .init_resource::<SpecializedRenderPipelines<SkyboxPrepassPipeline>>()
            .init_resource::<PreviousViewUniforms>()
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

/// Adds a skybox to a 3D camera, based on a cubemap texture.
///
/// Note that this component does not (currently) affect the scene's lighting.
/// To do so, use `EnvironmentMapLight` alongside this component.
///
/// See also <https://en.wikipedia.org/wiki/Skybox_(video_games)>.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct Skybox {
    pub image: Handle<Image>,
    /// Scale factor applied to the skybox image.
    /// After applying this multiplier to the image samples, the resulting values should
    /// be in units of [cd/m^2](https://en.wikipedia.org/wiki/Candela_per_square_metre).
    pub brightness: f32,

    /// View space rotation applied to the skybox cubemap.
    /// This is useful for users who require a different axis, such as the Z-axis, to serve
    /// as the vertical axis.
    pub rotation: Quat,
}

impl Default for Skybox {
    fn default() -> Self {
        Skybox {
            image: Handle::default(),
            brightness: 0.0,
            rotation: Quat::IDENTITY,
        }
    }
}

impl ExtractComponent for Skybox {
    type QueryData = (&'static Self, Option<&'static Exposure>);
    type QueryFilter = ();
    type Out = (Self, SkyboxUniforms);

    fn extract_component(
        (skybox, exposure): QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self::Out> {
        let exposure = exposure
            .map(Exposure::exposure)
            .unwrap_or_else(|| Exposure::default().exposure());

        Some((
            skybox.clone(),
            SkyboxUniforms {
                brightness: skybox.brightness * exposure,
                transform: Transform::from_rotation(skybox.rotation.inverse()).to_matrix(),
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                _wasm_padding_8b: 0,
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                _wasm_padding_12b: 0,
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                _wasm_padding_16b: 0,
            },
        ))
    }
}

// TODO: Replace with a push constant once WebGPU gets support for that
#[derive(Component, ShaderType, Clone)]
pub struct SkyboxUniforms {
    brightness: f32,
    transform: Mat4,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _wasm_padding_8b: u32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _wasm_padding_12b: u32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _wasm_padding_16b: u32,
}

#[derive(Resource)]
struct SkyboxPipeline {
    bind_group_layout: BindGroupLayout,
    shader: Handle<Shader>,
}

impl SkyboxPipeline {
    fn new(render_device: &RenderDevice, shader: Handle<Shader>) -> Self {
        Self {
            bind_group_layout: render_device.create_bind_group_layout(
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

fn init_skybox_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
) {
    let shader = load_embedded_asset!(asset_server.as_ref(), "skybox.wgsl");
    commands.insert_resource(SkyboxPipeline::new(&render_device, shader));
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct SkyboxPipelineKey {
    hdr: bool,
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
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
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
    views: Query<(Entity, &ExtractedView, &Msaa), With<Skybox>>,
) {
    for (entity, view, msaa) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            SkyboxPipelineKey {
                hdr: view.hdr,
                samples: msaa.samples(),
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
                &pipeline.bind_group_layout,
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
