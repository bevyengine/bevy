use bevy_core_pipeline::{
    core_3d::Camera3d, fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_math::{Mat4, Vec3};
use bevy_render::{
    camera::Camera,
    extract_component::ComponentUniforms,
    frame_graph::{BindGroupHandle, FrameGraph, TransientTexture, ResourceMeta, TextureInfo},
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
    view::{ExtractedView, Msaa, ViewDepthTexture, ViewUniform, ViewUniforms},
};

use crate::{GpuLights, LightMeta};

use super::{shaders, Atmosphere, AtmosphereSettings};

#[derive(Resource)]
pub(crate) struct AtmosphereBindGroupLayouts {
    pub transmittance_lut: BindGroupLayout,
    pub multiscattering_lut: BindGroupLayout,
    pub sky_view_lut: BindGroupLayout,
    pub aerial_view_lut: BindGroupLayout,
}

#[derive(Resource)]
pub(crate) struct RenderSkyBindGroupLayouts {
    pub render_sky: BindGroupLayout,
    pub render_sky_msaa: BindGroupLayout,
}

impl FromWorld for AtmosphereBindGroupLayouts {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let transmittance_lut = render_device.create_bind_group_layout(
            "transmittance_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<Atmosphere>(true)),
                    (1, uniform_buffer::<AtmosphereSettings>(true)),
                    (
                        // transmittance lut storage texture
                        13,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
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
                    (5, texture_2d(TextureSampleType::Float { filterable: true })), //transmittance lut and sampler
                    (6, sampler(SamplerBindingType::Filtering)),
                    (
                        //multiscattering lut storage texture
                        13,
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
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<Atmosphere>(true)),
                    (1, uniform_buffer::<AtmosphereSettings>(true)),
                    (2, uniform_buffer::<AtmosphereTransform>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    (5, texture_2d(TextureSampleType::Float { filterable: true })), //transmittance lut and sampler
                    (6, sampler(SamplerBindingType::Filtering)),
                    (7, texture_2d(TextureSampleType::Float { filterable: true })), //multiscattering lut and sampler
                    (8, sampler(SamplerBindingType::Filtering)),
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

        let aerial_view_lut = render_device.create_bind_group_layout(
            "aerial_view_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<Atmosphere>(true)),
                    (1, uniform_buffer::<AtmosphereSettings>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    (5, texture_2d(TextureSampleType::Float { filterable: true })), //transmittance lut and sampler
                    (6, sampler(SamplerBindingType::Filtering)),
                    (7, texture_2d(TextureSampleType::Float { filterable: true })), //multiscattering lut and sampler
                    (8, sampler(SamplerBindingType::Filtering)),
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
        let render_device = world.resource::<RenderDevice>();
        let render_sky = render_device.create_bind_group_layout(
            "render_sky_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (0, uniform_buffer::<Atmosphere>(true)),
                    (1, uniform_buffer::<AtmosphereSettings>(true)),
                    (2, uniform_buffer::<AtmosphereTransform>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    (5, texture_2d(TextureSampleType::Float { filterable: true })), //transmittance lut and sampler
                    (6, sampler(SamplerBindingType::Filtering)),
                    (9, texture_2d(TextureSampleType::Float { filterable: true })), //sky view lut and sampler
                    (10, sampler(SamplerBindingType::Filtering)),
                    (
                        // aerial view lut and sampler
                        11,
                        texture_3d(TextureSampleType::Float { filterable: true }),
                    ),
                    (12, sampler(SamplerBindingType::Filtering)),
                    (
                        //view depth texture
                        13,
                        texture_2d(TextureSampleType::Depth),
                    ),
                ),
            ),
        );

        let render_sky_msaa = render_device.create_bind_group_layout(
            "render_sky_msaa_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (0, uniform_buffer::<Atmosphere>(true)),
                    (1, uniform_buffer::<AtmosphereSettings>(true)),
                    (2, uniform_buffer::<AtmosphereTransform>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    (5, texture_2d(TextureSampleType::Float { filterable: true })), //transmittance lut and sampler
                    (6, sampler(SamplerBindingType::Filtering)),
                    (9, texture_2d(TextureSampleType::Float { filterable: true })), //sky view lut and sampler
                    (10, sampler(SamplerBindingType::Filtering)),
                    (
                        // aerial view lut and sampler
                        11,
                        texture_3d(TextureSampleType::Float { filterable: true }),
                    ),
                    (12, sampler(SamplerBindingType::Filtering)),
                    (
                        //view depth texture
                        13,
                        texture_2d_multisampled(TextureSampleType::Depth),
                    ),
                ),
            ),
        );

        Self {
            render_sky,
            render_sky_msaa,
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

        let base_sampler = SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        };

        let transmittance_lut = render_device.create_sampler(&SamplerDescriptor {
            label: Some("transmittance_lut_sampler"),
            ..base_sampler
        });

        let multiscattering_lut = render_device.create_sampler(&SamplerDescriptor {
            label: Some("multiscattering_lut_sampler"),
            ..base_sampler
        });

        let sky_view_lut = render_device.create_sampler(&SamplerDescriptor {
            label: Some("sky_view_lut_sampler"),
            address_mode_u: AddressMode::Repeat,
            ..base_sampler
        });

        let aerial_view_lut = render_device.create_sampler(&SamplerDescriptor {
            label: Some("aerial_view_lut_sampler"),
            ..base_sampler
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
            push_constant_ranges: vec![],
            shader: shaders::TRANSMITTANCE_LUT,
            shader_defs: vec![],
            entry_point: "main".into(),
            zero_initialize_workgroup_memory: false,
        });

        let multiscattering_lut =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("multi_scattering_lut_pipeline".into()),
                layout: vec![layouts.multiscattering_lut.clone()],
                push_constant_ranges: vec![],
                shader: shaders::MULTISCATTERING_LUT,
                shader_defs: vec![],
                entry_point: "main".into(),
                zero_initialize_workgroup_memory: false,
            });

        let sky_view_lut = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("sky_view_lut_pipeline".into()),
            layout: vec![layouts.sky_view_lut.clone()],
            push_constant_ranges: vec![],
            shader: shaders::SKY_VIEW_LUT,
            shader_defs: vec![],
            entry_point: "main".into(),
            zero_initialize_workgroup_memory: false,
        });

        let aerial_view_lut = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("aerial_view_lut_pipeline".into()),
            layout: vec![layouts.aerial_view_lut.clone()],
            push_constant_ranges: vec![],
            shader: shaders::AERIAL_VIEW_LUT,
            shader_defs: vec![],
            entry_point: "main".into(),
            zero_initialize_workgroup_memory: false,
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
    pub hdr: bool,
    pub dual_source_blending: bool,
}

impl SpecializedRenderPipeline for RenderSkyBindGroupLayouts {
    type Key = RenderSkyPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();

        if key.msaa_samples > 1 {
            shader_defs.push("MULTISAMPLED".into());
        }
        if key.hdr {
            shader_defs.push("TONEMAP_IN_SHADER".into());
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
            push_constant_ranges: vec![],
            vertex: fullscreen_shader_vertex_state(),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.msaa_samples,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            zero_initialize_workgroup_memory: false,
            fragment: Some(FragmentState {
                shader: shaders::RENDER_SKY.clone(),
                shader_defs,
                entry_point: "main".into(),
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
            }),
        }
    }
}

pub(super) fn queue_render_sky_pipelines(
    views: Query<(Entity, &Camera, &Msaa), With<Atmosphere>>,
    pipeline_cache: Res<PipelineCache>,
    layouts: Res<RenderSkyBindGroupLayouts>,
    mut specializer: ResMut<SpecializedRenderPipelines<RenderSkyBindGroupLayouts>>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    for (entity, camera, msaa) in &views {
        let id = specializer.specialize(
            &pipeline_cache,
            &layouts,
            RenderSkyPipelineKey {
                msaa_samples: msaa.samples(),
                hdr: camera.hdr,
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
    pub transmittance_lut: ResourceMeta<TransientTexture>,
    pub multiscattering_lut: ResourceMeta<TransientTexture>,
    pub sky_view_lut: ResourceMeta<TransientTexture>,
    pub aerial_view_lut: ResourceMeta<TransientTexture>,
}

impl AtmosphereTextures {
    pub fn get_transmittance_lut_key(entity: Entity) -> String {
        format!("transmittance_lut_{}", entity)
    }

    pub fn get_multiscattering_lut_key(entity: Entity) -> String {
        format!("multiscattering_lut{}", entity)
    }

    pub fn get_sky_view_lut_key(entity: Entity) -> String {
        format!("sky_view_lut{}", entity)
    }

    pub fn get_aerial_view_lut_key(entity: Entity) -> String {
        format!("aerial_view_lut{}", entity)
    }
}

pub(super) fn prepare_atmosphere_textures(
    views: Query<(Entity, &AtmosphereSettings), With<Atmosphere>>,
    mut commands: Commands,
) {
    for (entity, lut_settings) in &views {
        let transmittance_lut = ResourceMeta {
            key: AtmosphereTextures::get_transmittance_lut_key(entity),
            desc: TextureInfo {
                label: Some("transmittance_lut".into()),
                size: Extent3d {
                    width: lut_settings.transmittance_lut_size.x,
                    height: lut_settings.transmittance_lut_size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: vec![],
            },
        };

        let multiscattering_lut = ResourceMeta {
            key: AtmosphereTextures::get_multiscattering_lut_key(entity),
            desc: TextureInfo {
                label: Some("multiscattering_lut".into()),
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
                view_formats: vec![],
            },
        };

        let sky_view_lut = ResourceMeta {
            key: AtmosphereTextures::get_sky_view_lut_key(entity),
            desc: TextureInfo {
                label: Some("sky_view_lut".into()),
                size: Extent3d {
                    width: lut_settings.sky_view_lut_size.x,
                    height: lut_settings.sky_view_lut_size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: vec![],
            },
        };

        let aerial_view_lut = ResourceMeta {
            key: AtmosphereTextures::get_aerial_view_lut_key(entity),
            desc: TextureInfo {
                label: Some("aerial_view_lut".into()),
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
                view_formats: vec![],
            },
        };

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
    atmosphere_from_world: Mat4,
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
    views: Query<(Entity, &ExtractedView), (With<Atmosphere>, With<Camera3d>)>,
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
        let world_from_view = view.world_from_view.compute_matrix();
        let camera_z = world_from_view.z_axis.truncate();
        let camera_y = world_from_view.y_axis.truncate();
        let atmo_z = camera_z
            .with_y(0.0)
            .try_normalize()
            .unwrap_or_else(|| camera_y.with_y(0.0).normalize());
        let atmo_y = Vec3::Y;
        let atmo_x = atmo_y.cross(atmo_z).normalize();
        let world_from_atmosphere = Mat4::from_cols(
            atmo_x.extend(0.0),
            atmo_y.extend(0.0),
            atmo_z.extend(0.0),
            world_from_view.w_axis,
        );

        let atmosphere_from_world = world_from_atmosphere.inverse();

        commands.entity(entity).insert(AtmosphereTransformsOffset {
            index: writer.write(&AtmosphereTransform {
                world_from_atmosphere,
                atmosphere_from_world,
            }),
        });
    }
}

#[derive(Component)]
pub(crate) struct AtmosphereBindGroups {
    pub transmittance_lut: BindGroupHandle,
    pub multiscattering_lut: BindGroupHandle,
    pub sky_view_lut: BindGroupHandle,
    pub aerial_view_lut: BindGroupHandle,
    pub render_sky: BindGroupHandle,
}

pub(super) fn prepare_atmosphere_bind_groups(
    views: Query<
        (Entity, &AtmosphereTextures, &ViewDepthTexture, &Msaa),
        (With<Camera3d>, With<Atmosphere>),
    >,
    layouts: Res<AtmosphereBindGroupLayouts>,
    render_sky_layouts: Res<RenderSkyBindGroupLayouts>,
    samplers: Res<AtmosphereSamplers>,
    view_uniforms: Res<ViewUniforms>,
    lights_uniforms: Res<LightMeta>,
    atmosphere_transforms: Res<AtmosphereTransforms>,
    atmosphere_uniforms: Res<ComponentUniforms<Atmosphere>>,
    settings_uniforms: Res<ComponentUniforms<AtmosphereSettings>>,
    mut commands: Commands,
    mut frame_graph: ResMut<FrameGraph>,
) {
    if views.iter().len() == 0 {
        return;
    }

    let atmosphere_handle = atmosphere_uniforms
        .make_binding_resource_handle(&mut frame_graph)
        .expect("Failed to prepare atmosphere bind groups. Atmosphere uniform buffer missing");

    let transforms_handle = atmosphere_transforms
        .uniforms()
        .make_binding_resource_handle(&mut frame_graph)
        .expect("Failed to prepare atmosphere bind groups. Atmosphere transforms buffer missing");

    let settings_handle = settings_uniforms
        .make_binding_resource_handle(&mut frame_graph)
        .expect(
            "Failed to prepare atmosphere bind groups. AtmosphereSettings uniform buffer missing",
        );

    let view_handle = view_uniforms
        .uniforms
        .make_binding_resource_handle(&mut frame_graph)
        .expect("Failed to prepare atmosphere bind groups. View uniform buffer missing");

    let lights_handle = lights_uniforms
        .view_gpu_lights
        .make_binding_resource_handle(&mut frame_graph)
        .expect("Failed to prepare atmosphere bind groups. Lights uniform buffer missing");

    for (entity, textures, view_depth_texture, msaa) in &views {
        let transmittance_lut = frame_graph
            .create_bind_group_handle_builder(
                Some("transmittance_lut_bind_group".into()),
                &layouts.transmittance_lut,
            )
            .add_handle(0, &atmosphere_handle)
            .add_handle(1, &settings_handle)
            .add_helper(13, &textures.transmittance_lut)
            .build();

        let multiscattering_lut = frame_graph
            .create_bind_group_handle_builder(
                Some("multiscattering_lut_bind_group".into()),
                &layouts.multiscattering_lut,
            )
            .add_handle(0, &atmosphere_handle)
            .add_handle(1, &settings_handle)
            .add_helper(5, &textures.transmittance_lut)
            .add_handle(6, &samplers.transmittance_lut)
            .add_helper(13, &textures.multiscattering_lut)
            .build();

        let sky_view_lut = frame_graph
            .create_bind_group_handle_builder(
                Some("sky_view_lut_bind_group".into()),
                &layouts.sky_view_lut,
            )
            .add_handle(0, &atmosphere_handle)
            .add_handle(1, &settings_handle)
            .add_handle(2, &transforms_handle)
            .add_handle(3, &view_handle)
            .add_handle(4, &lights_handle)
            .add_helper(5, &textures.transmittance_lut)
            .add_handle(6, &samplers.transmittance_lut)
            .add_helper(7, &textures.multiscattering_lut)
            .add_handle(8, &samplers.multiscattering_lut)
            .add_helper(13, &textures.sky_view_lut)
            .build();

        let aerial_view_lut = frame_graph
            .create_bind_group_handle_builder(
                Some("aerial_view_lut_bind_group".into()),
                &layouts.aerial_view_lut,
            )
            .add_handle(0, &atmosphere_handle)
            .add_handle(1, &settings_handle)
            .add_handle(2, &transforms_handle)
            .add_handle(3, &view_handle)
            .add_handle(4, &lights_handle)
            .add_helper(5, &textures.transmittance_lut)
            .add_handle(6, &samplers.transmittance_lut)
            .add_helper(7, &textures.multiscattering_lut)
            .add_handle(8, &samplers.multiscattering_lut)
            .add_helper(13, &textures.aerial_view_lut)
            .build();

        let render_sky = frame_graph
            .create_bind_group_handle_builder(
                Some("render_sky_bind_group".into()),
                if *msaa == Msaa::Off {
                    &render_sky_layouts.render_sky
                } else {
                    &render_sky_layouts.render_sky_msaa
                },
            )
            .add_handle(0, &atmosphere_handle)
            .add_handle(1, &settings_handle)
            .add_handle(2, &transforms_handle)
            .add_handle(3, &view_handle)
            .add_handle(4, &lights_handle)
            .add_helper(5, &textures.transmittance_lut)
            .add_handle(6, &samplers.transmittance_lut)
            .add_helper(9, &textures.sky_view_lut)
            .add_handle(10, &samplers.sky_view_lut)
            .add_helper(11, &textures.aerial_view_lut)
            .add_handle(12, &samplers.aerial_view_lut)
            .add_helper(13, &view_depth_texture.texture)
            .build();

        commands.entity(entity).insert(AtmosphereBindGroups {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
            render_sky,
        });
    }
}
