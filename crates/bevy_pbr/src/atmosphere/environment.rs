use crate::{
    resources::{
        AtmosphereSamplers, AtmosphereTextures, AtmosphereTransform, AtmosphereTransforms,
        AtmosphereTransformsOffset,
    },
    AtmosphereSettings, GpuLights, LightMeta, ViewLightsUniformOffset,
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

use super::Atmosphere;

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
}

#[derive(Component)]
pub(crate) struct AtmosphereProbeBindGroups {
    pub environment: BindGroup,
}

#[derive(Resource)]
pub struct AtmosphereProbeLayouts {
    pub environment: BindGroupLayout,
}

#[derive(Resource)]
pub struct AtmosphereProbePipeline {
    pub environment: CachedComputePipelineId,
}

pub fn init_atmosphere_probe_layout(mut commands: Commands, render_device: Res<RenderDevice>) {
    let environment = render_device.create_bind_group_layout(
        "environment_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                uniform_buffer::<Atmosphere>(true),
                uniform_buffer::<AtmosphereSettings>(true),
                uniform_buffer::<AtmosphereTransform>(true),
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<GpuLights>(true),
                texture_2d(TextureSampleType::Float { filterable: true }), //transmittance lut and sampler
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }), //multiscattering lut and sampler
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }), //sky view lut and sampler
                sampler(SamplerBindingType::Filtering),
                texture_3d(TextureSampleType::Float { filterable: true }), //aerial view lut ans sampler
                sampler(SamplerBindingType::Filtering),
                texture_storage_2d_array(
                    // output 2D array texture
                    TextureFormat::Rgba16Float,
                    StorageTextureAccess::WriteOnly,
                ),
            ),
        ),
    );

    commands.insert_resource(AtmosphereProbeLayouts { environment });
}

pub(super) fn prepare_atmosphere_probe_bind_groups(
    probes: Query<(Entity, &AtmosphereProbeTextures), With<AtmosphereEnvironmentMap>>,
    render_device: Res<RenderDevice>,
    layouts: Res<AtmosphereProbeLayouts>,
    samplers: Res<AtmosphereSamplers>,
    view_uniforms: Res<ViewUniforms>,
    lights_uniforms: Res<LightMeta>,
    atmosphere_transforms: Res<AtmosphereTransforms>,
    atmosphere_uniforms: Res<ComponentUniforms<Atmosphere>>,
    settings_uniforms: Res<ComponentUniforms<AtmosphereSettings>>,
    mut commands: Commands,
) {
    for (entity, textures) in &probes {
        let environment = render_device.create_bind_group(
            "environment_bind_group",
            &layouts.environment,
            &BindGroupEntries::sequential((
                atmosphere_uniforms.binding().unwrap(),
                settings_uniforms.binding().unwrap(),
                atmosphere_transforms.uniforms().binding().unwrap(),
                view_uniforms.uniforms.binding().unwrap(),
                lights_uniforms.view_gpu_lights.binding().unwrap(),
                &textures.transmittance_lut.default_view,
                &samplers.transmittance_lut,
                &textures.multiscattering_lut.default_view,
                &samplers.multiscattering_lut,
                &textures.sky_view_lut.default_view,
                &samplers.sky_view_lut,
                &textures.aerial_view_lut.default_view,
                &samplers.aerial_view_lut,
                &textures.environment,
            )),
        );

        commands
            .entity(entity)
            .insert(AtmosphereProbeBindGroups { environment });
    }
}

pub(super) fn prepare_probe_textures(
    view_textures: Query<&AtmosphereTextures, With<Atmosphere>>,
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
    commands.insert_resource(AtmosphereProbePipeline { environment });
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
        Read<DynamicUniformIndex<Atmosphere>>,
        Read<DynamicUniformIndex<AtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
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
        )),) = (self.main_view_query.get_manual(world, view_entity),)
        else {
            return Ok(());
        };

        for (bind_groups, env_map_light) in self.probe_query.iter_manual(world) {
            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("environment_pass"),
                        timestamp_writes: None,
                    });

            pass.set_pipeline(environment_pipeline);
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

            pass.dispatch_workgroups(
                env_map_light.size.x / 8,
                env_map_light.size.y / 8,
                6, // 6 cubemap faces
            );
        }

        Ok(())
    }
}
