use crate::{
    resources::{
        AtmosphereSampler, AtmosphereTextures, AtmosphereTransform, AtmosphereTransforms,
        AtmosphereTransformsOffset, GpuAtmosphere,
    },
    CloudLayer, ExtractedAtmosphere, GpuAtmosphereSettings, GpuLights, LightMeta,
    ScatteringMediumSampler, ViewLightsUniformOffset,
};
use bevy_asset::{load_embedded_asset, AssetServer, Assets, Handle, RenderAssetUsages};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryState, With, Without},
    resource::Resource,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_image::Image;
use bevy_light::{AtmosphereEnvironmentMapLight, GeneratedEnvironmentMapLight};
use bevy_math::{Quat, UVec2};
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex, ExtractComponent},
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_resource::{binding_types::*, *},
    renderer::{RenderContext, RenderDevice},
    texture::{CachedTexture, GpuImage},
    view::{ViewUniform, ViewUniformOffset, ViewUniforms},
};
use bevy_utils::default;
use tracing::warn;

use crate::fbm_noise::FbmNoiseTexture;
use crate::perlin_worley_noise::PerlinWorleyNoiseTexture;
use crate::GpuScatteringMedium;

// Render world representation of an environment map light for the atmosphere
#[derive(Component, ExtractComponent, Clone)]
pub struct AtmosphereEnvironmentMap {
    pub environment_map: Handle<Image>,
    pub size: UVec2,
}

#[derive(Component)]
pub struct AtmosphereProbeTextures {
    pub environment: TextureView,
    pub transmittance_lut: CachedTexture,
    pub multiscattering_lut: CachedTexture,
    pub sky_view_lut: CachedTexture,
    pub aerial_view_lut: CachedTexture,
    pub cloud_shadow_map: CachedTexture,
}

#[derive(Component)]
pub(crate) struct AtmosphereProbeBindGroups {
    pub environment: BindGroup,
    pub environment_clouds: Option<BindGroup>,
}

#[derive(Resource)]
pub struct AtmosphereProbeLayouts {
    pub environment: BindGroupLayoutDescriptor,
    pub environment_clouds: BindGroupLayoutDescriptor,
}

#[derive(Resource)]
pub struct AtmosphereProbePipeline {
    pub environment: CachedComputePipelineId,
    pub environment_clouds: CachedComputePipelineId,
}

pub fn init_atmosphere_probe_layout(mut commands: Commands) {
    let environment = BindGroupLayoutDescriptor::new(
        "environment_bind_group_layout",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::COMPUTE,
            (
                // uniforms
                (0, uniform_buffer::<GpuAtmosphere>(true)),
                (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                (2, uniform_buffer::<AtmosphereTransform>(true)),
                (3, uniform_buffer::<ViewUniform>(true)),
                (4, uniform_buffer::<GpuLights>(true)),
                // scattering medium luts and sampler (required by `raymarch_atmosphere`)
                (5, texture_2d(TextureSampleType::default())),
                (6, texture_2d(TextureSampleType::default())),
                (7, sampler(SamplerBindingType::Filtering)),
                // atmosphere luts and sampler
                (8, texture_2d(TextureSampleType::default())), // transmittance
                (9, texture_2d(TextureSampleType::default())), // multiscattering
                (10, texture_2d(TextureSampleType::default())), // sky view
                (11, texture_3d(TextureSampleType::default())), // aerial view
                (12, sampler(SamplerBindingType::Filtering)),
                // output 2D array texture
                (
                    13,
                    texture_storage_2d_array(
                        TextureFormat::Rgba16Float,
                        StorageTextureAccess::WriteOnly,
                    ),
                ),
            ),
        ),
    );

    // Clouds-enabled variant: adds scattering medium LUTs + cloud bindings used by `raymarch_atmosphere()`.
    let environment_clouds = BindGroupLayoutDescriptor::new(
        "environment_bind_group_layout_clouds",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::COMPUTE,
            (
                // uniforms
                (0, uniform_buffer::<GpuAtmosphere>(true)),
                (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                (2, uniform_buffer::<AtmosphereTransform>(true)),
                (3, uniform_buffer::<ViewUniform>(true)),
                (4, uniform_buffer::<GpuLights>(true)),
                // scattering medium luts and sampler (required by raymarch_atmosphere)
                (5, texture_2d(TextureSampleType::default())),
                (6, texture_2d(TextureSampleType::default())),
                (7, sampler(SamplerBindingType::Filtering)),
                // atmosphere luts and sampler
                (8, texture_2d(TextureSampleType::default())), // transmittance
                (9, texture_2d(TextureSampleType::default())), // multiscattering
                (10, texture_2d(TextureSampleType::default())), // sky view
                (11, texture_3d(TextureSampleType::default())), // aerial view
                (12, sampler(SamplerBindingType::Filtering)),
                // cloud bindings (match `clouds.wgsl` / `bindings.wgsl`)
                (14, uniform_buffer::<CloudLayer>(true)),
                (15, texture_2d(TextureSampleType::Float { filterable: true })),
                (16, sampler(SamplerBindingType::Filtering)),
                (17, texture_2d(TextureSampleType::Float { filterable: true })), // cloud shadow map
                (18, texture_3d(TextureSampleType::Float { filterable: true })), // perlin-worley
                // output 2D array texture
                (
                    13,
                    texture_storage_2d_array(
                        TextureFormat::Rgba16Float,
                        StorageTextureAccess::WriteOnly,
                    ),
                ),
            ),
        ),
    );

    commands.insert_resource(AtmosphereProbeLayouts {
        environment,
        environment_clouds,
    });
}

pub(super) fn prepare_atmosphere_probe_bind_groups(
    probes: Query<(Entity, &AtmosphereProbeTextures), With<AtmosphereEnvironmentMap>>,
    render_device: Res<RenderDevice>,
    layouts: Res<AtmosphereProbeLayouts>,
    atmosphere_sampler: Res<AtmosphereSampler>,
    (
        medium_sampler,
        view_uniforms,
        lights_uniforms,
        atmosphere_transforms,
        atmosphere_uniforms,
        settings_uniforms,
        cloud_layer_uniforms,
        fbm_noise_texture,
        perlin_worley_noise_texture,
        cloud_noise_sampler,
        gpu_media,
        views,
        pipeline_cache,
    ): (
        Res<ScatteringMediumSampler>,
        Res<ViewUniforms>,
        Res<LightMeta>,
        Res<AtmosphereTransforms>,
        Res<ComponentUniforms<GpuAtmosphere>>,
        Res<ComponentUniforms<GpuAtmosphereSettings>>,
        Res<ComponentUniforms<CloudLayer>>,
        Res<FbmNoiseTexture>,
        Res<PerlinWorleyNoiseTexture>,
        Res<crate::resources::CloudNoiseSampler>,
        Res<RenderAssets<GpuScatteringMedium>>,
        Query<&ExtractedAtmosphere, With<ExtractedAtmosphere>>,
        Res<PipelineCache>,
    ),
    mut commands: Commands,
) {
    let atmosphere = views.iter().next();

    for (entity, textures) in &probes {
        let Some(atmosphere) = atmosphere else {
            continue;
        };
        let Some(gpu_medium) = gpu_media.get(atmosphere.medium) else {
            continue;
        };

        let environment = render_device.create_bind_group(
            "environment_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.environment),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_uniforms.binding().unwrap()),
                (1, settings_uniforms.binding().unwrap()),
                (2, atmosphere_transforms.uniforms().binding().unwrap()),
                (3, view_uniforms.uniforms.binding().unwrap()),
                (4, lights_uniforms.view_gpu_lights.binding().unwrap()),
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
                // output 2D array texture
                (13, &textures.environment),
            )),
        );

        // Optional clouds-enabled bind group (requires CloudLayer + scattering medium + noise textures).
        let environment_clouds = (|| -> Option<BindGroup> {
            let cloud_layer_binding = cloud_layer_uniforms.binding()?;
            // `gpu_medium` comes from the selected view's atmosphere above.

            Some(render_device.create_bind_group(
                "environment_bind_group_clouds",
                &pipeline_cache.get_bind_group_layout(&layouts.environment_clouds),
                &BindGroupEntries::with_indices((
                    // uniforms
                    (0, atmosphere_uniforms.binding().unwrap()),
                    (1, settings_uniforms.binding().unwrap()),
                    (2, atmosphere_transforms.uniforms().binding().unwrap()),
                    (3, view_uniforms.uniforms.binding().unwrap()),
                    (4, lights_uniforms.view_gpu_lights.binding().unwrap()),
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
                    // cloud bindings
                    (14, cloud_layer_binding.clone()),
                    (15, &fbm_noise_texture.texture.default_view),
                    (16, &**cloud_noise_sampler),
                    (17, &textures.cloud_shadow_map.default_view),
                    (18, &perlin_worley_noise_texture.texture.default_view),
                    // output
                    (13, &textures.environment),
                )),
            ))
        })();

        commands
            .entity(entity)
            .insert(AtmosphereProbeBindGroups {
                environment,
                environment_clouds,
            });
    }
}

pub(super) fn prepare_probe_textures(
    view_textures: Query<&AtmosphereTextures, With<ExtractedAtmosphere>>,
    probes: Query<
        (Entity, &AtmosphereEnvironmentMap),
        (
            With<AtmosphereEnvironmentMap>,
            Without<AtmosphereProbeTextures>,
        ),
    >,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut commands: Commands,
) {
    for (probe, render_env_map) in &probes {
        let environment = gpu_images.get(&render_env_map.environment_map).unwrap();
        // create a cube view
        let environment_view = environment.texture.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });
        // Get the first view entity's textures to borrow
        if let Some(view_textures) = view_textures.iter().next() {
            commands.entity(probe).insert(AtmosphereProbeTextures {
                environment: environment_view,
                transmittance_lut: view_textures.transmittance_lut.clone(),
                multiscattering_lut: view_textures.multiscattering_lut.clone(),
                sky_view_lut: view_textures.sky_view_lut.clone(),
                aerial_view_lut: view_textures.aerial_view_lut.clone(),
                cloud_shadow_map: view_textures.cloud_shadow_map.clone(),
            });
        }
    }
}

pub fn init_atmosphere_probe_pipeline(
    pipeline_cache: Res<PipelineCache>,
    layouts: Res<AtmosphereProbeLayouts>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let environment = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("environment_pipeline".into()),
        layout: vec![layouts.environment.clone()],
        shader: load_embedded_asset!(asset_server.as_ref(), "environment.wgsl"),
        ..default()
    });

    let environment_clouds = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("environment_pipeline_clouds".into()),
        layout: vec![layouts.environment_clouds.clone()],
        shader: load_embedded_asset!(asset_server.as_ref(), "environment.wgsl"),
        shader_defs: vec!["CLOUDS_ENABLED".into()],
        ..default()
    });

    commands.insert_resource(AtmosphereProbePipeline {
        environment,
        environment_clouds,
    });
}

// Ensure power-of-two dimensions to avoid edge update issues on cubemap faces
pub fn validate_environment_map_size(size: UVec2) -> UVec2 {
    let new_size = UVec2::new(
        size.x.max(1).next_power_of_two(),
        size.y.max(1).next_power_of_two(),
    );
    if new_size != size {
        warn!(
            "Non-power-of-two AtmosphereEnvironmentMapLight size {}, correcting to {new_size}",
            size
        );
    }
    new_size
}

pub fn prepare_atmosphere_probe_components(
    probes: Query<(Entity, &AtmosphereEnvironmentMapLight), (Without<AtmosphereEnvironmentMap>,)>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    for (entity, env_map_light) in &probes {
        // Create a cubemap image in the main world that we can reference
        let new_size = validate_environment_map_size(env_map_light.size);
        let mut environment_image = Image::new_fill(
            Extent3d {
                width: new_size.x,
                height: new_size.y,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            &[0; 8],
            TextureFormat::Rgba16Float,
            RenderAssetUsages::all(),
        );

        environment_image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        environment_image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
            | TextureUsages::STORAGE_BINDING
            | TextureUsages::COPY_SRC;

        // Add the image to assets to get a handle
        let environment_handle = images.add(environment_image);

        commands.entity(entity).insert(AtmosphereEnvironmentMap {
            environment_map: environment_handle.clone(),
            size: new_size,
        });

        commands
            .entity(entity)
            .insert(GeneratedEnvironmentMapLight {
                environment_map: environment_handle,
                intensity: env_map_light.intensity,
                rotation: Quat::IDENTITY,
                affects_lightmapped_mesh_diffuse: env_map_light.affects_lightmapped_mesh_diffuse,
            });
    }
}

pub(super) struct EnvironmentNode {
    main_view_query: QueryState<(
        Read<DynamicUniformIndex<GpuAtmosphere>>,
        Read<DynamicUniformIndex<GpuAtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Option<Read<DynamicUniformIndex<CloudLayer>>>,
    )>,
    probe_query: QueryState<(
        Read<AtmosphereProbeBindGroups>,
        Read<AtmosphereEnvironmentMap>,
    )>,
}

impl FromWorld for EnvironmentNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            main_view_query: QueryState::new(world),
            probe_query: QueryState::new(world),
        }
    }
}

impl Node for EnvironmentNode {
    fn update(&mut self, world: &mut World) {
        self.main_view_query.update_archetypes(world);
        self.probe_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<AtmosphereProbePipeline>();
        let view_entity = graph.view_entity();

        let Some(environment_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.environment)
        else {
            return Ok(());
        };

        let (Ok((
            atmosphere_uniforms_offset,
            settings_uniforms_offset,
            atmosphere_transforms_offset,
            view_uniforms_offset,
            lights_uniforms_offset,
            cloud_layer_offset,
        )),) = (self.main_view_query.get_manual(world, view_entity),)
        else {
            return Ok(());
        };

        for (bind_groups, env_map_light) in self.probe_query.iter_manual(world) {
            let use_clouds = cloud_layer_offset.is_some() && bind_groups.environment_clouds.is_some();

            let pipeline = if use_clouds {
                pipeline_cache.get_compute_pipeline(pipelines.environment_clouds)
            } else {
                Some(environment_pipeline)
            };

            let Some(environment_pipeline) = pipeline else {
                continue;
            };

            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("environment_pass"),
                        timestamp_writes: None,
                    });

            pass.set_pipeline(environment_pipeline);

            if use_clouds {
                let cloud_layer_offset = cloud_layer_offset.unwrap();
                pass.set_bind_group(
                    0,
                    bind_groups.environment_clouds.as_ref().unwrap(),
                    &[
                        atmosphere_uniforms_offset.index(),
                        settings_uniforms_offset.index(),
                        atmosphere_transforms_offset.index(),
                        view_uniforms_offset.offset,
                        lights_uniforms_offset.offset,
                        cloud_layer_offset.index(),
                    ],
                );
            } else {
                pass.set_bind_group(
                    0,
                    &bind_groups.environment,
                    &[
                        atmosphere_uniforms_offset.index(),
                        settings_uniforms_offset.index(),
                        atmosphere_transforms_offset.index(),
                        view_uniforms_offset.offset,
                        lights_uniforms_offset.offset,
                    ],
                );
            }

            pass.dispatch_workgroups(
                env_map_light.size.x / 8,
                env_map_light.size.y / 8,
                6, // 6 cubemap faces
            );
        }

        Ok(())
    }
}
