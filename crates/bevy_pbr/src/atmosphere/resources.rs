use crate::{
    ExtractedAtmosphere, GpuLights, GpuScatteringMedium, LightMeta, ScatteringMedium,
    ScatteringMediumSampler,
};
use bevy_asset::{load_embedded_asset, AssetId, Handle};
use bevy_camera::{Camera, Camera3d};
use bevy_core_pipeline::FullscreenShader;
use bevy_derive::Deref;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    error::BevyError,
    query::With,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_image::ToExtents;
use bevy_math::{Affine3A, Mat4, Vec3, Vec3A};
use bevy_render::{
    extract_component::ComponentUniforms,
    render_asset::RenderAssets,
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{CachedTexture, TextureCache},
    view::{ExtractedView, Msaa, ViewDepthTexture, ViewUniform, ViewUniforms},
};
use bevy_shader::Shader;
use bevy_utils::default;

use super::GpuAtmosphereSettings;

#[derive(Resource)]
pub(crate) struct AtmosphereBindGroupLayouts {
    pub transmittance_lut: BindGroupLayoutDescriptor,
    pub multiscattering_lut: BindGroupLayoutDescriptor,
    pub sky_view_lut: BindGroupLayoutDescriptor,
    pub aerial_view_lut: BindGroupLayoutDescriptor,
}

#[derive(Resource)]
pub(crate) struct RenderSkyBindGroupLayouts {
    pub render_sky: BindGroupLayoutDescriptor,
    pub render_sky_msaa: BindGroupLayoutDescriptor,
    pub fullscreen_shader: FullscreenShader,
    pub fragment_shader: Handle<Shader>,
}

impl AtmosphereBindGroupLayouts {
    pub fn new() -> Self {
        let transmittance_lut = BindGroupLayoutDescriptor::new(
            "transmittance_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    // scattering medium luts and sampler
                    (5, texture_2d(TextureSampleType::default())),
                    (6, texture_2d(TextureSampleType::default())),
                    (7, sampler(SamplerBindingType::Filtering)),
                    // transmittance lut storage texture
                    (
                        13,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        let multiscattering_lut = BindGroupLayoutDescriptor::new(
            "multiscattering_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    // scattering medium luts and sampler
                    (5, texture_2d(TextureSampleType::default())),
                    (6, texture_2d(TextureSampleType::default())),
                    (7, sampler(SamplerBindingType::Filtering)),
                    // atmosphere luts and sampler
                    (8, texture_2d(TextureSampleType::default())), // transmittance
                    (12, sampler(SamplerBindingType::Filtering)),
                    // multiscattering lut storage texture
                    (
                        13,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        let sky_view_lut = BindGroupLayoutDescriptor::new(
            "sky_view_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    (2, uniform_buffer::<AtmosphereTransform>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    // scattering medium luts and sampler
                    (5, texture_2d(TextureSampleType::default())),
                    (6, texture_2d(TextureSampleType::default())),
                    (7, sampler(SamplerBindingType::Filtering)),
                    // atmosphere luts and sampler
                    (8, texture_2d(TextureSampleType::default())), // transmittance
                    (9, texture_2d(TextureSampleType::default())), // multiscattering
                    (12, sampler(SamplerBindingType::Filtering)),
                    // sky view lut storage texture
                    (
                        13,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        let aerial_view_lut = BindGroupLayoutDescriptor::new(
            "aerial_view_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    // scattering medium luts and sampler
                    (5, texture_2d(TextureSampleType::default())),
                    (6, texture_2d(TextureSampleType::default())),
                    (7, sampler(SamplerBindingType::Filtering)),
                    // atmosphere luts and sampler
                    (8, texture_2d(TextureSampleType::default())), // transmittance
                    (9, texture_2d(TextureSampleType::default())), // multiscattering
                    (12, sampler(SamplerBindingType::Filtering)),
                    // eerial view lut storage texture
                    (
                        13,
                        texture_storage_3d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        Self {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
        }
    }
}

impl FromWorld for RenderSkyBindGroupLayouts {
    fn from_world(world: &mut World) -> Self {
        let render_sky = BindGroupLayoutDescriptor::new(
            "render_sky_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    (2, uniform_buffer::<AtmosphereTransform>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    // scattering medium luts and sampler
                    (5, texture_2d(TextureSampleType::default())),
                    (6, texture_2d(TextureSampleType::default())),
                    (7, sampler(SamplerBindingType::Filtering)),
                    // atmosphere luts and sampler
                    (8, texture_2d(TextureSampleType::default())), // transmittance
                    (9, texture_2d(TextureSampleType::default())), // multiscattering
                    (10, texture_2d(TextureSampleType::default())), // sky view
                    (11, texture_3d(TextureSampleType::default())), // aerial view
                    (12, sampler(SamplerBindingType::Filtering)),
                    // view depth texture
                    (13, texture_2d(TextureSampleType::Depth)),
                ),
            ),
        );

        let render_sky_msaa = BindGroupLayoutDescriptor::new(
            "render_sky_msaa_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    (2, uniform_buffer::<AtmosphereTransform>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    // scattering medium luts and sampler
                    (5, texture_2d(TextureSampleType::default())),
                    (6, texture_2d(TextureSampleType::default())),
                    (7, sampler(SamplerBindingType::Filtering)),
                    // atmosphere luts and sampler
                    (8, texture_2d(TextureSampleType::default())), // transmittance
                    (9, texture_2d(TextureSampleType::default())), // multiscattering
                    (10, texture_2d(TextureSampleType::default())), // sky view
                    (11, texture_3d(TextureSampleType::default())), // aerial view
                    (12, sampler(SamplerBindingType::Filtering)),
                    // view depth texture
                    (13, texture_2d_multisampled(TextureSampleType::Depth)),
                ),
            ),
        );

        Self {
            render_sky,
            render_sky_msaa,
            fullscreen_shader: world.resource::<FullscreenShader>().clone(),
            fragment_shader: load_embedded_asset!(world, "render_sky.wgsl"),
        }
    }
}

#[derive(Resource, Deref)]
pub struct AtmosphereSampler(Sampler);

impl FromWorld for AtmosphereSampler {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self(sampler)
    }
}

#[derive(Resource)]
pub(crate) struct AtmosphereLutPipelines {
    pub transmittance_lut: CachedComputePipelineId,
    pub multiscattering_lut: CachedComputePipelineId,
    pub sky_view_lut: CachedComputePipelineId,
    pub aerial_view_lut: CachedComputePipelineId,
}

impl FromWorld for AtmosphereLutPipelines {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let layouts = world.resource::<AtmosphereBindGroupLayouts>();

        let transmittance_lut = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("transmittance_lut_pipeline".into()),
            layout: vec![layouts.transmittance_lut.clone()],
            shader: load_embedded_asset!(world, "transmittance_lut.wgsl"),
            ..default()
        });

        let multiscattering_lut =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("multi_scattering_lut_pipeline".into()),
                layout: vec![layouts.multiscattering_lut.clone()],
                shader: load_embedded_asset!(world, "multiscattering_lut.wgsl"),
                ..default()
            });

        let sky_view_lut = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("sky_view_lut_pipeline".into()),
            layout: vec![layouts.sky_view_lut.clone()],
            shader: load_embedded_asset!(world, "sky_view_lut.wgsl"),
            ..default()
        });

        let aerial_view_lut = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("aerial_view_lut_pipeline".into()),
            layout: vec![layouts.aerial_view_lut.clone()],
            shader: load_embedded_asset!(world, "aerial_view_lut.wgsl"),
            ..default()
        });

        Self {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
        }
    }
}

#[derive(Component)]
pub(crate) struct RenderSkyPipelineId(pub CachedRenderPipelineId);

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub(crate) struct RenderSkyPipelineKey {
    pub msaa_samples: u32,
    pub dual_source_blending: bool,
}

impl SpecializedRenderPipeline for RenderSkyBindGroupLayouts {
    type Key = RenderSkyPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();

        if key.msaa_samples > 1 {
            shader_defs.push("MULTISAMPLED".into());
        }
        if key.dual_source_blending {
            shader_defs.push("DUAL_SOURCE_BLENDING".into());
        }

        let dst_factor = if key.dual_source_blending {
            BlendFactor::Src1
        } else {
            BlendFactor::SrcAlpha
        };

        RenderPipelineDescriptor {
            label: Some(format!("render_sky_pipeline_{}", key.msaa_samples).into()),
            layout: vec![if key.msaa_samples == 1 {
                self.render_sky.clone()
            } else {
                self.render_sky_msaa.clone()
            }],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            multisample: MultisampleState {
                count: key.msaa_samples,
                ..default()
            },
            ..default()
        }
    }
}

pub(super) fn queue_render_sky_pipelines(
    views: Query<(Entity, &Msaa), (With<Camera>, With<ExtractedAtmosphere>)>,
    pipeline_cache: Res<PipelineCache>,
    layouts: Res<RenderSkyBindGroupLayouts>,
    mut specializer: ResMut<SpecializedRenderPipelines<RenderSkyBindGroupLayouts>>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    for (entity, msaa) in &views {
        let id = specializer.specialize(
            &pipeline_cache,
            &layouts,
            RenderSkyPipelineKey {
                msaa_samples: msaa.samples(),
                dual_source_blending: render_device
                    .features()
                    .contains(WgpuFeatures::DUAL_SOURCE_BLENDING),
            },
        );
        commands.entity(entity).insert(RenderSkyPipelineId(id));
    }
}

#[derive(Component)]
pub struct AtmosphereTextures {
    pub transmittance_lut: CachedTexture,
    pub multiscattering_lut: CachedTexture,
    pub sky_view_lut: CachedTexture,
    pub aerial_view_lut: CachedTexture,
}

pub(super) fn prepare_atmosphere_textures(
    views: Query<(Entity, &GpuAtmosphereSettings), With<ExtractedAtmosphere>>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
) {
    for (entity, lut_settings) in &views {
        let transmittance_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("transmittance_lut"),
                size: lut_settings.transmittance_lut_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let multiscattering_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("multiscattering_lut"),
                size: lut_settings.multiscattering_lut_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let sky_view_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("sky_view_lut"),
                size: lut_settings.sky_view_lut_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let aerial_view_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("aerial_view_lut"),
                size: lut_settings.aerial_view_lut_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        commands.entity(entity).insert({
            AtmosphereTextures {
                transmittance_lut,
                multiscattering_lut,
                sky_view_lut,
                aerial_view_lut,
            }
        });
    }
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error("ScatteringMedium missing with id {0:?}: make sure the asset was not removed.")]
struct ScatteringMediumMissingError(AssetId<ScatteringMedium>);

/// The shader-uniform representation of an Atmosphere.
#[derive(Clone, Component, ShaderType)]
pub struct GpuAtmosphere {
    //TODO: rename to Planet later?
    pub ground_albedo: Vec3,
    pub bottom_radius: f32,
    pub top_radius: f32,
}

pub fn prepare_atmosphere_uniforms(
    mut commands: Commands,
    atmospheres: Query<(Entity, &ExtractedAtmosphere)>,
) -> Result<(), BevyError> {
    for (entity, atmosphere) in atmospheres {
        commands.entity(entity).insert(GpuAtmosphere {
            ground_albedo: atmosphere.ground_albedo,
            bottom_radius: atmosphere.bottom_radius,
            top_radius: atmosphere.top_radius,
        });
    }
    Ok(())
}

#[derive(Resource, Default)]
pub struct AtmosphereTransforms {
    uniforms: DynamicUniformBuffer<AtmosphereTransform>,
}

impl AtmosphereTransforms {
    #[inline]
    pub fn uniforms(&self) -> &DynamicUniformBuffer<AtmosphereTransform> {
        &self.uniforms
    }
}

#[derive(ShaderType)]
pub struct AtmosphereTransform {
    world_from_atmosphere: Mat4,
}

#[derive(Component)]
pub struct AtmosphereTransformsOffset {
    index: u32,
}

impl AtmosphereTransformsOffset {
    #[inline]
    pub fn index(&self) -> u32 {
        self.index
    }
}

pub(super) fn prepare_atmosphere_transforms(
    views: Query<(Entity, &ExtractedView), (With<ExtractedAtmosphere>, With<Camera3d>)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut atmo_uniforms: ResMut<AtmosphereTransforms>,
    mut commands: Commands,
) {
    let atmo_count = views.iter().len();
    let Some(mut writer) =
        atmo_uniforms
            .uniforms
            .get_writer(atmo_count, &render_device, &render_queue)
    else {
        return;
    };

    for (entity, view) in &views {
        let world_from_view = view.world_from_view.affine();
        let camera_z = world_from_view.matrix3.z_axis;
        let camera_y = world_from_view.matrix3.y_axis;
        let atmo_z = camera_z
            .with_y(0.0)
            .try_normalize()
            .unwrap_or_else(|| camera_y.with_y(0.0).normalize());
        let atmo_y = Vec3A::Y;
        let atmo_x = atmo_y.cross(atmo_z).normalize();
        let world_from_atmosphere =
            Affine3A::from_cols(atmo_x, atmo_y, atmo_z, world_from_view.translation);

        let world_from_atmosphere = Mat4::from(world_from_atmosphere);

        commands.entity(entity).insert(AtmosphereTransformsOffset {
            index: writer.write(&AtmosphereTransform {
                world_from_atmosphere,
            }),
        });
    }
}

#[derive(Component)]
pub(crate) struct AtmosphereBindGroups {
    pub transmittance_lut: BindGroup,
    pub multiscattering_lut: BindGroup,
    pub sky_view_lut: BindGroup,
    pub aerial_view_lut: BindGroup,
    pub render_sky: BindGroup,
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
enum AtmosphereBindGroupError {
    #[error("Failed to prepare atmosphere bind groups. Atmosphere uniform buffer missing")]
    Atmosphere,
    #[error(
        "Failed to prepare atmosphere bind groups. AtmosphereTransforms uniform buffer missing"
    )]
    Transforms,
    #[error("Failed to prepare atmosphere bind groups. AtmosphereSettings uniform buffer missing")]
    Settings,
    #[error("Failed to prepare atmosphere bind groups. View uniform buffer missing")]
    ViewUniforms,
    #[error("Failed to prepare atmosphere bind groups. Light uniform buffer missing")]
    LightUniforms,
}

pub(super) fn prepare_atmosphere_bind_groups(
    views: Query<
        (
            Entity,
            &ExtractedAtmosphere,
            &AtmosphereTextures,
            &ViewDepthTexture,
            &Msaa,
        ),
        (With<Camera3d>, With<ExtractedAtmosphere>),
    >,
    render_device: Res<RenderDevice>,
    layouts: Res<AtmosphereBindGroupLayouts>,
    render_sky_layouts: Res<RenderSkyBindGroupLayouts>,
    atmosphere_sampler: Res<AtmosphereSampler>,
    view_uniforms: Res<ViewUniforms>,
    lights_uniforms: Res<LightMeta>,
    atmosphere_transforms: Res<AtmosphereTransforms>,
    atmosphere_uniforms: Res<ComponentUniforms<GpuAtmosphere>>,
    settings_uniforms: Res<ComponentUniforms<GpuAtmosphereSettings>>,
    gpu_media: Res<RenderAssets<GpuScatteringMedium>>,
    medium_sampler: Res<ScatteringMediumSampler>,
    pipeline_cache: Res<PipelineCache>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    if views.iter().len() == 0 {
        return Ok(());
    }

    let atmosphere_binding = atmosphere_uniforms
        .binding()
        .ok_or(AtmosphereBindGroupError::Atmosphere)?;

    let transforms_binding = atmosphere_transforms
        .uniforms()
        .binding()
        .ok_or(AtmosphereBindGroupError::Transforms)?;

    let settings_binding = settings_uniforms
        .binding()
        .ok_or(AtmosphereBindGroupError::Settings)?;

    let view_binding = view_uniforms
        .uniforms
        .binding()
        .ok_or(AtmosphereBindGroupError::ViewUniforms)?;

    let lights_binding = lights_uniforms
        .view_gpu_lights
        .binding()
        .ok_or(AtmosphereBindGroupError::LightUniforms)?;

    for (entity, atmosphere, textures, view_depth_texture, msaa) in &views {
        let gpu_medium = gpu_media
            .get(atmosphere.medium)
            .ok_or(ScatteringMediumMissingError(atmosphere.medium))?;

        let transmittance_lut = render_device.create_bind_group(
            "transmittance_lut_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.transmittance_lut),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                // scattering medium luts and sampler
                (5, &gpu_medium.density_lut_view),
                (6, &gpu_medium.scattering_lut_view),
                (7, medium_sampler.sampler()),
                // transmittance lut storage texture
                (13, &textures.transmittance_lut.default_view),
            )),
        );

        let multiscattering_lut = render_device.create_bind_group(
            "multiscattering_lut_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.multiscattering_lut),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                // scattering medium luts and sampler
                (5, &gpu_medium.density_lut_view),
                (6, &gpu_medium.scattering_lut_view),
                (7, medium_sampler.sampler()),
                // atmosphere luts and sampler
                (8, &textures.transmittance_lut.default_view),
                (12, &**atmosphere_sampler),
                // multiscattering lut storage texture
                (13, &textures.multiscattering_lut.default_view),
            )),
        );

        let sky_view_lut = render_device.create_bind_group(
            "sky_view_lut_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.sky_view_lut),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                (2, transforms_binding.clone()),
                (3, view_binding.clone()),
                (4, lights_binding.clone()),
                // scattering medium luts and sampler
                (5, &gpu_medium.density_lut_view),
                (6, &gpu_medium.scattering_lut_view),
                (7, medium_sampler.sampler()),
                // atmosphere luts and sampler
                (8, &textures.transmittance_lut.default_view),
                (9, &textures.multiscattering_lut.default_view),
                (12, &**atmosphere_sampler),
                // sky view lut storage texture
                (13, &textures.sky_view_lut.default_view),
            )),
        );

        let aerial_view_lut = render_device.create_bind_group(
            "sky_view_lut_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.aerial_view_lut),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                (3, view_binding.clone()),
                (4, lights_binding.clone()),
                // scattering medium luts and sampler
                (5, &gpu_medium.density_lut_view),
                (6, &gpu_medium.scattering_lut_view),
                (7, medium_sampler.sampler()),
                // atmosphere luts and sampler
                (8, &textures.transmittance_lut.default_view),
                (9, &textures.multiscattering_lut.default_view),
                (12, &**atmosphere_sampler),
                // aerial view lut storage texture
                (13, &textures.aerial_view_lut.default_view),
            )),
        );

        let render_sky = render_device.create_bind_group(
            "render_sky_bind_group",
            &pipeline_cache.get_bind_group_layout(if *msaa == Msaa::Off {
                &render_sky_layouts.render_sky
            } else {
                &render_sky_layouts.render_sky_msaa
            }),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                (2, transforms_binding.clone()),
                (3, view_binding.clone()),
                (4, lights_binding.clone()),
                // scattering medium luts and sampler
                (5, &gpu_medium.density_lut_view),
                (6, &gpu_medium.scattering_lut_view),
                (7, medium_sampler.sampler()),
                // atmosphere luts and sampler
                (8, &textures.transmittance_lut.default_view),
                (9, &textures.multiscattering_lut.default_view),
                (10, &textures.sky_view_lut.default_view),
                (11, &textures.aerial_view_lut.default_view),
                (12, &**atmosphere_sampler),
                // view depth texture
                (13, view_depth_texture.view()),
            )),
        );

        commands.entity(entity).insert(AtmosphereBindGroups {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
            render_sky,
        });
    }

    Ok(())
}

#[derive(ShaderType)]
#[repr(C)]
pub(crate) struct AtmosphereData {
    pub atmosphere: GpuAtmosphere,
    pub settings: GpuAtmosphereSettings,
}

pub fn init_atmosphere_buffer(mut commands: Commands) {
    commands.insert_resource(AtmosphereBuffer {
        buffer: StorageBuffer::from(AtmosphereData {
            atmosphere: GpuAtmosphere {
                ground_albedo: Vec3::ZERO,
                bottom_radius: 0.0,
                top_radius: 0.0,
            },
            settings: GpuAtmosphereSettings::default(),
        }),
    });
}

#[derive(Resource)]
pub struct AtmosphereBuffer {
    pub(crate) buffer: StorageBuffer<AtmosphereData>,
}

pub(crate) fn write_atmosphere_buffer(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    atmosphere_entity: Query<(&GpuAtmosphere, &GpuAtmosphereSettings), With<Camera3d>>,
    mut atmosphere_buffer: ResMut<AtmosphereBuffer>,
) {
    let Ok((atmosphere, settings)) = atmosphere_entity.single() else {
        return;
    };

    atmosphere_buffer.buffer.set(AtmosphereData {
        atmosphere: atmosphere.clone(),
        settings: settings.clone(),
    });
    atmosphere_buffer.buffer.write_buffer(&device, &queue);
}
