use bevy_core_pipeline::{
    core_3d::Camera3d, fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    extract_component::ComponentUniforms,
    render_resource::{
        binding_types::{
            sampler, texture_2d, texture_storage_2d, texture_storage_3d, uniform_buffer,
        },
        AddressMode, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedComputePipelineId, CachedRenderPipelineId, ColorTargetState, ColorWrites,
        ComputePipelineDescriptor, Extent3d, FilterMode, FragmentState, MultisampleState,
        PipelineCache, PrimitiveState, RenderPipelineDescriptor, Sampler, SamplerBindingType,
        SamplerDescriptor, ShaderStages, StorageTextureAccess, TextureDescriptor, TextureDimension,
        TextureFormat, TextureSampleType, TextureUsages,
    },
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
    view::{ViewUniform, ViewUniforms},
};

use crate::{GpuLights, LightMeta};

use super::{shaders, Atmosphere, AtmosphereSettings};

#[derive(Resource)]
pub(crate) struct AtmosphereBindGroupLayouts {
    pub transmittance_lut: BindGroupLayout,
    pub multiscattering_lut: BindGroupLayout,
    pub sky_view_lut: BindGroupLayout,
    pub aerial_view_lut: BindGroupLayout,
    pub render_sky: BindGroupLayout,
}

impl FromWorld for AtmosphereBindGroupLayouts {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let transmittance_lut = render_device.create_bind_group_layout(
            "transmittance_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (0, uniform_buffer::<Atmosphere>(true)),
                    (1, uniform_buffer::<AtmosphereSettings>(true)),
                ),
            ),
        );

        let multiscattering_lut = render_device.create_bind_group_layout(
            "multiscattering_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<Atmosphere>(true)),
                    (1, uniform_buffer::<AtmosphereSettings>(true)),
                    (4, texture_2d(TextureSampleType::Float { filterable: true })), //transmittance lut and sampler
                    (5, sampler(SamplerBindingType::Filtering)),
                    (
                        //multiscattering lut storage texture
                        12,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        let sky_view_lut = render_device.create_bind_group_layout(
            "sky_view_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (0, uniform_buffer::<Atmosphere>(true)),
                    (1, uniform_buffer::<AtmosphereSettings>(true)),
                    (2, uniform_buffer::<ViewUniform>(true)),
                    (3, uniform_buffer::<GpuLights>(true)),
                    (4, texture_2d(TextureSampleType::Float { filterable: true })), //transmittance lut and sampler
                    (5, sampler(SamplerBindingType::Filtering)),
                    (6, texture_2d(TextureSampleType::Float { filterable: true })), //multiscattering lut and sampler
                    (7, sampler(SamplerBindingType::Filtering)),
                ),
            ),
        );

        let aerial_view_lut = render_device.create_bind_group_layout(
            "aerial_view_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<Atmosphere>(true)),
                    (1, uniform_buffer::<AtmosphereSettings>(true)),
                    (2, uniform_buffer::<ViewUniform>(true)),
                    (3, uniform_buffer::<GpuLights>(true)),
                    (4, texture_2d(TextureSampleType::Float { filterable: true })), //transmittance lut and sampler
                    (5, sampler(SamplerBindingType::Filtering)),
                    (6, texture_2d(TextureSampleType::Float { filterable: true })), //multiscattering lut and sampler
                    (7, sampler(SamplerBindingType::Filtering)),
                    (
                        //Aerial view lut storage texture
                        13,
                        texture_storage_3d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        let render_sky = render_device.create_bind_group_layout(
            "render_sky_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (2, uniform_buffer::<ViewUniform>(true)),
                    (8, texture_2d(TextureSampleType::Float { filterable: true })),
                    (9, sampler(SamplerBindingType::Filtering)),
                ),
            ),
        );

        Self {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
            render_sky,
        }
    }
}

#[derive(Resource)]
pub struct AtmosphereSamplers {
    pub transmittance_lut: Sampler,
    pub multiscattering_lut: Sampler,
    pub sky_view_lut: Sampler,
    pub aerial_view_lut: Sampler,
}

impl FromWorld for AtmosphereSamplers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let transmittance_lut = render_device.create_sampler(&SamplerDescriptor {
            label: Some("transmittance_lut_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let multiscattering_lut = render_device.create_sampler(&SamplerDescriptor {
            label: Some("multiscattering_lut_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let sky_view_lut = render_device.create_sampler(&SamplerDescriptor {
            label: Some("sky_view_lut_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            address_mode_u: AddressMode::Repeat, //want to wrap along the equator
            ..Default::default()
        });

        let aerial_view_lut = render_device.create_sampler(&SamplerDescriptor {
            label: Some("aerial_view_lut_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        Self {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
        }
    }
}

#[derive(Resource)]
pub(crate) struct AtmospherePipelines {
    pub transmittance_lut: CachedRenderPipelineId,
    pub multiscattering_lut: CachedComputePipelineId,
    pub sky_view_lut: CachedRenderPipelineId,
    pub aerial_view_lut: CachedComputePipelineId,
    pub render_sky: CachedRenderPipelineId,
}

impl FromWorld for AtmospherePipelines {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let layouts = world.resource::<AtmosphereBindGroupLayouts>();

        let transmittance_lut = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("transmittance_lut_pipeline".into()),
            layout: vec![layouts.transmittance_lut.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: shaders::TRANSMITTANCE_LUT.clone(),
                shader_defs: vec![],
                entry_point: "main".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
        });

        let multiscattering_lut =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("multi_scattering_lut_pipeline".into()),
                layout: vec![layouts.multiscattering_lut.clone()],
                push_constant_ranges: vec![],
                shader: shaders::MULTISCATTERING_LUT,
                shader_defs: vec![],
                entry_point: "main".into(),
            });

        let sky_view_lut = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("sky_view_lut_pipeline".into()),
            layout: vec![layouts.sky_view_lut.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: shaders::SKY_VIEW_LUT.clone(),
                shader_defs: vec![],
                entry_point: "main".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
        });

        let aerial_view_lut = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("aerial_view_lut_pipeline".into()),
            layout: vec![layouts.aerial_view_lut.clone()],
            push_constant_ranges: vec![],
            shader: shaders::AERIAL_VIEW_LUT,
            shader_defs: vec![],
            entry_point: "main".into(),
        });

        let render_sky = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("render_sky_pipeline".into()),
            layout: vec![layouts.render_sky.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: shaders::RENDER_SKY.clone(),
                shader_defs: vec![],
                entry_point: "main".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
        });

        Self {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
            render_sky,
        }
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
    views: Query<(Entity, &AtmosphereSettings), With<Atmosphere>>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
) {
    for (entity, lut_settings) in &views {
        let transmittance_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("transmittance_lut"),
                size: Extent3d {
                    width: lut_settings.transmittance_lut_size.x,
                    height: lut_settings.multiscattering_lut_size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let multiscattering_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("multiscattering_lut"),
                size: Extent3d {
                    width: lut_settings.multiscattering_lut_size.x,
                    height: lut_settings.multiscattering_lut_size.y,
                    depth_or_array_layers: 1,
                },
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
                size: Extent3d {
                    width: lut_settings.sky_view_lut_size.x,
                    height: lut_settings.sky_view_lut_size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float, //TODO: check if needs hdr
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let aerial_view_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("aerial_view_lut"),
                size: Extent3d {
                    width: lut_settings.aerial_view_lut_size.x,
                    height: lut_settings.aerial_view_lut_size.y,
                    depth_or_array_layers: lut_settings.aerial_view_lut_size.z,
                },
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

#[derive(Component)]
pub(crate) struct AtmosphereBindGroups {
    pub transmittance_lut: BindGroup,
    pub multiscattering_lut: BindGroup,
    pub sky_view_lut: BindGroup,
    pub aerial_view_lut: BindGroup,
    pub render_sky: BindGroup,
}

#[expect(clippy::too_many_arguments)]
pub(super) fn prepare_atmosphere_bind_groups(
    views: Query<(Entity, &AtmosphereTextures), (With<Camera3d>, With<Atmosphere>)>,
    render_device: Res<RenderDevice>,
    layouts: Res<AtmosphereBindGroupLayouts>,
    samplers: Res<AtmosphereSamplers>,
    view_uniforms: Res<ViewUniforms>,
    lights_uniforms: Res<LightMeta>,
    atmosphere_uniforms: Res<ComponentUniforms<Atmosphere>>,
    settings_uniforms: Res<ComponentUniforms<AtmosphereSettings>>,
    mut commands: Commands,
) {
    let atmosphere_binding = atmosphere_uniforms
        .binding()
        .expect("Failed to prepare atmosphere bind groups. Atmosphere uniform buffer missing");

    let settings_binding = settings_uniforms.binding().expect(
        "Failed to prepare atmosphere bind groups. AtmosphereSettings uniform buffer missing",
    );

    let view_binding = view_uniforms
        .uniforms
        .binding()
        .expect("Failed to prepare atmosphere bind groups. View uniform buffer missing");

    let lights_binding = lights_uniforms
        .view_gpu_lights
        .binding()
        .expect("Failed to prepare atmosphere bind groups. Lights uniform buffer missing");

    for (entity, textures) in &views {
        let transmittance_lut = render_device.create_bind_group(
            "transmittance_lut_bind_group",
            &layouts.transmittance_lut,
            &BindGroupEntries::with_indices((
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
            )),
        );

        let multiscattering_lut = render_device.create_bind_group(
            "multiscattering_lut_bind_group",
            &layouts.multiscattering_lut,
            &BindGroupEntries::with_indices((
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                (4, &textures.transmittance_lut.default_view),
                (5, &samplers.transmittance_lut),
                (12, &textures.multiscattering_lut.default_view),
            )),
        );

        let sky_view_lut = render_device.create_bind_group(
            "sky_view_lut_bind_group",
            &layouts.sky_view_lut,
            &BindGroupEntries::with_indices((
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                (2, view_binding.clone()),
                (3, lights_binding.clone()),
                (4, &textures.transmittance_lut.default_view),
                (5, &samplers.transmittance_lut),
                (6, &textures.multiscattering_lut.default_view),
                (7, &samplers.multiscattering_lut),
            )),
        );

        let aerial_view_lut = render_device.create_bind_group(
            "sky_view_lut_bind_group",
            &layouts.aerial_view_lut,
            &BindGroupEntries::with_indices((
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                (2, view_binding.clone()),
                (3, lights_binding.clone()),
                (4, &textures.transmittance_lut.default_view),
                (5, &samplers.transmittance_lut),
                (6, &textures.multiscattering_lut.default_view),
                (7, &samplers.multiscattering_lut),
                (13, &textures.aerial_view_lut.default_view),
            )),
        );

        let render_sky = render_device.create_bind_group(
            "render_sky_bind_group",
            &layouts.render_sky,
            &BindGroupEntries::with_indices((
                (2, view_binding.clone()),
                (8, &textures.sky_view_lut.default_view),
                (9, &samplers.sky_view_lut),
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
}
