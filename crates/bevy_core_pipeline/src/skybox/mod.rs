use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::QueryItem,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut, Resource},
};
use bevy_render::{
    camera::Exposure,
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{sampler, texture_2d, texture_cube, uniform_buffer},
        *,
    },
    renderer::RenderDevice,
    texture::{BevyDefault, Image},
    view::{ExtractedView, Msaa, ViewTarget, ViewUniform, ViewUniforms},
    Render, RenderApp, RenderSet,
};

use crate::core_3d::CORE_3D_DEPTH_FORMAT;

const SKYBOX_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(55594763423201);

pub struct SkyboxPlugin;

impl Plugin for SkyboxPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, SKYBOX_SHADER_HANDLE, "skybox.wgsl", Shader::from_wgsl);

        app.add_plugins((
            ExtractComponentPlugin::<Skybox>::default(),
            UniformComponentPlugin::<SkyboxUniforms>::default(),
        ));

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<SkyboxPipeline>>()
            .add_systems(
                Render,
                (
                    prepare_skybox_pipelines.in_set(RenderSet::Prepare),
                    prepare_skybox_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_device = render_app.world.resource::<RenderDevice>().clone();

        render_app.insert_resource(SkyboxPipeline::new(&render_device));
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
/// An option configuring the layout of the texture used as a skybox.
pub enum SkyboxTextureLayout {
    /// The skybox texture is a cubemap.
    ///
    /// See <https://en.wikipedia.org/wiki/Cube_mapping>.
    Cubemap,
    /// The skybox texture is an equirectangular projection.
    /// Equirectangular projections are commonly used for maps but have some drawbacks
    /// as you may encounter visual artifacts at both the top and bottom of the skybox.
    ///
    /// See <https://en.wikipedia.org/wiki/Equirectangular_projection>.
    Equirectangular,
}

/// Adds a skybox to a 3D camera, based on a texture.
///
/// Note that this component does not (currently) affect the scene's lighting.
/// To do so, use `EnvironmentMapLight` alongside this component.
///
/// See also <https://en.wikipedia.org/wiki/Skybox_(video_games)>.
#[derive(Component, Clone)]
pub struct Skybox {
    pub image: Handle<Image>,
    /// Scale factor applied to the skybox image.
    /// After applying this multiplier to the image samples, the resulting values should
    /// be in units of [cd/m^2](https://en.wikipedia.org/wiki/Candela_per_square_metre).
    pub brightness: f32,
    /// The layout of the given texture.
    pub skybox_texture_layout: SkyboxTextureLayout,
}

impl ExtractComponent for Skybox {
    type QueryData = (&'static Self, Option<&'static Exposure>);
    type QueryFilter = ();
    type Out = (Self, SkyboxUniforms);

    fn extract_component((skybox, exposure): QueryItem<'_, Self::QueryData>) -> Option<Self::Out> {
        let exposure = exposure
            .map(|e| e.exposure())
            .unwrap_or_else(|| Exposure::default().exposure());

        Some((
            skybox.clone(),
            SkyboxUniforms {
                brightness: skybox.brightness * exposure,
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
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _wasm_padding_8b: u32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _wasm_padding_12b: u32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _wasm_padding_16b: u32,
}

#[derive(Resource)]
struct SkyboxPipeline {
    bind_group_layout_tex_2d: BindGroupLayout,
    bind_group_layout_tex_cube: BindGroupLayout,
}

impl SkyboxPipeline {
    fn new(render_device: &RenderDevice) -> Self {
        Self {
            bind_group_layout_tex_2d: render_device.create_bind_group_layout(
                "skybox_bind_group_layout_texture_2d",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: true }),
                        sampler(SamplerBindingType::Filtering),
                        uniform_buffer::<ViewUniform>(true)
                            .visibility(ShaderStages::VERTEX_FRAGMENT),
                        uniform_buffer::<SkyboxUniforms>(true),
                    ),
                ),
            ),
            bind_group_layout_tex_cube: render_device.create_bind_group_layout(
                "skybox_bind_group_layout_texture_cube",
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
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct SkyboxPipelineKey {
    hdr: bool,
    samples: u32,
    depth_format: TextureFormat,
    skybox_texture_layout: SkyboxTextureLayout,
}

impl SpecializedRenderPipeline for SkyboxPipeline {
    type Key = SkyboxPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];
        if key.skybox_texture_layout == SkyboxTextureLayout::Cubemap {
            shader_defs.push("CUBEMAP".into());
        };
        let layout = match key.skybox_texture_layout {
            SkyboxTextureLayout::Cubemap => &self.bind_group_layout_tex_cube,
            SkyboxTextureLayout::Equirectangular => &self.bind_group_layout_tex_2d,
        };

        RenderPipelineDescriptor {
            label: Some("skybox_pipeline".into()),
            layout: vec![layout.clone()],
            push_constant_ranges: Vec::new(),
            vertex: VertexState {
                shader: SKYBOX_SHADER_HANDLE,
                shader_defs: shader_defs.clone(),
                entry_point: "skybox_vertex".into(),
                buffers: Vec::new(),
            },
            primitive: PrimitiveState::default(),
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
                shader: SKYBOX_SHADER_HANDLE,
                shader_defs,
                entry_point: "skybox_fragment".into(),
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
            }),
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
    msaa: Res<Msaa>,
    views: Query<(Entity, &Skybox, &ExtractedView)>,
) {
    for (entity, skybox, view) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            SkyboxPipelineKey {
                hdr: view.hdr,
                samples: msaa.samples(),
                depth_format: CORE_3D_DEPTH_FORMAT,
                skybox_texture_layout: skybox.skybox_texture_layout,
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
    images: Res<RenderAssets<Image>>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &Skybox, &DynamicUniformIndex<SkyboxUniforms>)>,
) {
    for (entity, skybox, skybox_uniform_index) in &views {
        if let (Some(skybox_texture), Some(view_uniforms), Some(skybox_uniforms)) = (
            images.get(&skybox.image),
            view_uniforms.uniforms.binding(),
            skybox_uniforms.binding(),
        ) {
            let layout = match skybox.skybox_texture_layout {
                SkyboxTextureLayout::Cubemap => &pipeline.bind_group_layout_tex_cube,
                SkyboxTextureLayout::Equirectangular => &pipeline.bind_group_layout_tex_2d,
            };

            let bind_group = render_device.create_bind_group(
                "skybox_bind_group",
                layout,
                &BindGroupEntries::sequential((
                    &skybox_texture.texture_view,
                    &skybox_texture.sampler,
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
