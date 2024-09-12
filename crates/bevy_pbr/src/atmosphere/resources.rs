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
            sampler, texture_2d, texture_3d, texture_storage_2d, texture_storage_3d, uniform_buffer,
        },
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedComputePipelineId, CachedRenderPipelineId, ColorTargetState, ColorWrites,
        ComputePipelineDescriptor, Extent3d, FilterMode, FragmentState, MultisampleState,
        PipelineCache, PrimitiveState, RenderPipelineDescriptor, Sampler, SamplerBindingType,
        SamplerDescriptor, ShaderStages, StorageTextureAccess, TextureDescriptor, TextureDimension,
        TextureFormat, TextureSampleType, TextureUsages,
    },
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
    view::{ViewDepthTexture, ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};

use super::{shaders, Atmosphere, AtmosphereSettings};

#[derive(Resource)]
pub(crate) struct AtmosphereBindGroupLayouts {
    pub transmittance_lut: BindGroupLayout,
    pub multiscattering_lut: BindGroupLayout,
    pub sky_view_lut: BindGroupLayout,
    pub aerial_view_lut: BindGroupLayout,
    pub apply_atmosphere: BindGroupLayout,
}

impl FromWorld for AtmosphereBindGroupLayouts {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let transmittance_lut = render_device.create_bind_group_layout(
            "transmittance_lut_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<Atmosphere>(true),
                    uniform_buffer::<AtmosphereSettings>(true),
                ),
            ),
        );

        let multiscattering_lut = render_device.create_bind_group_layout(
            "multiscattering_lut_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    uniform_buffer::<Atmosphere>(true),
                    uniform_buffer::<AtmosphereSettings>(true),
                    texture_2d(TextureSampleType::Float { filterable: true }), //transmittance_lut. need sampler?;
                    sampler(SamplerBindingType::Filtering),
                    texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        );

        let sky_view_lut = render_device.create_bind_group_layout(
            "sky_view_lut_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<Atmosphere>(true),
                    uniform_buffer::<AtmosphereSettings>(true),
                    texture_2d(TextureSampleType::Float { filterable: true }), //transmittance_lut
                    sampler(SamplerBindingType::Filtering),
                    texture_2d(TextureSampleType::Float { filterable: true }), //multiscattering_lut
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        let aerial_view_lut = render_device.create_bind_group_layout(
            "aerial_view_lut_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    uniform_buffer::<Atmosphere>(true),
                    uniform_buffer::<AtmosphereSettings>(true), //TODO: maybe unnecessary?
                    texture_2d(TextureSampleType::Float { filterable: true }), //transmittance_lut
                    sampler(SamplerBindingType::Filtering),
                    texture_2d(TextureSampleType::Float { filterable: true }), //multiscattering_lut
                    sampler(SamplerBindingType::Filtering),
                    texture_storage_3d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        );

        let apply = render_device.create_bind_group_layout(
            "apply_atmosphere_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<ViewUniform>(true),
                    texture_2d(TextureSampleType::Depth),
                    sampler(SamplerBindingType::Filtering),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    texture_3d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        Self {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
            apply_atmosphere: apply,
        }
    }
}

#[derive(Resource)]
pub struct AtmosphereSamplers {
    //TODO: maybe this is redundant, but I'm guessing you can't bind samplers more than once at the same time
    pub transmittance_lut: Sampler,
    pub multiscattering_lut: Sampler,
    pub sky_view_lut: Sampler,
    pub aerial_view_lut: Sampler,
    pub view_depth: Sampler, //TODO: get actual depth sampler (or just texture load?);
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
            ..Default::default()
        });

        let aerial_view_lut = render_device.create_sampler(&SamplerDescriptor {
            label: Some("aerial_view_lut_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        let view_depth = render_device.create_sampler(&SamplerDescriptor {
            label: Some("view_depth_sampler"),
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
            view_depth,
        }
    }
}

#[derive(Resource)]
pub(crate) struct AtmospherePipelines {
    pub transmittance_lut: CachedRenderPipelineId,
    pub multiscattering_lut: CachedComputePipelineId,
    pub sky_view_lut: CachedRenderPipelineId,
    pub aerial_view_lut: CachedComputePipelineId,
    pub apply_atmosphere: CachedRenderPipelineId,
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

        let multi_scattering_lut =
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

        let apply_atmosphere = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("apply_atmosphere_pipeline".into()),
            layout: vec![layouts.apply_atmosphere.clone()],
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                shader: shaders::APPLY_ATMOSPHERE.clone(),
                shader_defs: vec![],
                entry_point: "main".into(),
                targets: vec![Some(ColorTargetState {
                    //TODO: only works with HDR for now. Need to integrate non-hdr + tonemap in shader. But then this node doesn't work at all, so idk. Maybe just add stuff to view bind group and integrate with normal lighting?
                    format: ViewTarget::TEXTURE_FORMAT_HDR,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
        });

        Self {
            transmittance_lut,
            multiscattering_lut: multi_scattering_lut,
            sky_view_lut,
            aerial_view_lut,
            apply_atmosphere,
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
    pub apply: BindGroup,
}

#[expect(clippy::too_many_arguments)]
pub(super) fn prepare_atmosphere_bind_groups(
    views: Query<
        (Entity, &AtmosphereTextures, &ViewDepthTexture),
        (With<Camera3d>, With<Atmosphere>),
    >,
    render_device: Res<RenderDevice>,
    layouts: Res<AtmosphereBindGroupLayouts>,
    samplers: Res<AtmosphereSamplers>,
    view_uniforms: Res<ViewUniforms>,
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

    for (entity, textures, depth_texture) in &views {
        let transmittance_lut = render_device.create_bind_group(
            "transmittance_lut_bind_group",
            &layouts.transmittance_lut,
            &BindGroupEntries::sequential((atmosphere_binding.clone(), settings_binding.clone())),
        );

        let multiscattering_lut = render_device.create_bind_group(
            "multiscattering_lut_bind_group",
            &layouts.multiscattering_lut,
            &BindGroupEntries::sequential((
                atmosphere_binding.clone(),
                settings_binding.clone(),
                &textures.transmittance_lut.default_view,
                &samplers.transmittance_lut,
                &textures.multiscattering_lut.default_view,
                &samplers.multiscattering_lut,
            )),
        );

        let sky_view_lut = render_device.create_bind_group(
            "sky_view_lut_bind_group",
            &layouts.sky_view_lut,
            &BindGroupEntries::sequential((
                atmosphere_binding.clone(),
                settings_binding.clone(),
                &textures.transmittance_lut.default_view,
                &samplers.transmittance_lut,
                &textures.multiscattering_lut.default_view,
                &samplers.multiscattering_lut,
            )),
        );

        let aerial_view_lut = render_device.create_bind_group(
            "sky_view_lut_bind_group",
            &layouts.aerial_view_lut,
            &BindGroupEntries::sequential((
                atmosphere_binding.clone(),
                settings_binding.clone(),
                &textures.transmittance_lut.default_view,
                &samplers.transmittance_lut,
                &textures.multiscattering_lut.default_view,
                &samplers.multiscattering_lut,
                &textures.aerial_view_lut.default_view,
            )),
        );

        let apply_atmosphere = render_device.create_bind_group(
            "apply_atmosphere_bind_group",
            &layouts.apply_atmosphere,
            &BindGroupEntries::sequential((
                view_binding.clone(),
                depth_texture.view(),
                &samplers.view_depth,
                &textures.sky_view_lut.default_view,
                &samplers.sky_view_lut,
                &textures.aerial_view_lut.default_view,
                &samplers.aerial_view_lut,
            )),
        );

        commands.entity(entity).insert(AtmosphereBindGroups {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
            apply: apply_atmosphere,
        });
    }
}
