use crate::{
    environment_map::EnvironmentMapExposures,
    resources::{
        AtmosphereTextures, AtmosphereTransform, AtmosphereTransforms, AtmosphereTransformsOffset,
        GpuAtmosphere,
    },
    ExtractedAtmosphere, GpuAtmosphereSettings, GpuLights, LightMeta, LinearSampler,
    ViewLightsUniformOffset,
};
use bevy_asset::{load_embedded_asset, AssetServer, Assets, Handle, RenderAssetUsages};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    lifecycle::{Insert, Remove},
    observer::On,
    query::{Has, With, Without},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
    template::FromTemplate,
};
use bevy_image::Image;
use bevy_light::{
    AtmosphereEnvironmentMapLight, EnvironmentMapLight, GeneratedEnvironmentMapLight,
};
use bevy_math::UVec2;
use bevy_render::{
    camera::{ExtractedCamera, SortedCameras},
    diagnostic::RecordDiagnostics,
    extract_component::{ComponentUniforms, DynamicUniformIndex, ExtractComponent},
    render_asset::RenderAssets,
    render_resource::{binding_types::*, *},
    renderer::{RenderContext, RenderDevice, ViewQuery},
    texture::GpuImage,
    view::{ViewUniform, ViewUniformOffset, ViewUniforms},
    RenderApp,
};
use bevy_utils::default;
use tracing::warn;

// Render world representation of an environment map light for the atmosphere
#[derive(Component, ExtractComponent, Clone, FromTemplate)]
#[extract_app(RenderApp)]
#[extract_component_sync_target((
    Self,
    AtmosphereProbeSource,
    AtmosphereProbeTextures,
    AtmosphereProbeBindGroups
))]
pub struct AtmosphereEnvironmentMap {
    pub environment_map: Handle<Image>,
    pub size: UVec2,
}

/// The atmosphere view that renders a probe's environment map this frame, from its own LUTs and
/// uniforms.
#[derive(Component)]
pub struct AtmosphereProbeSource(pub Entity);

#[derive(Component)]
pub struct AtmosphereProbeTextures {
    pub environment: TextureView,
}

#[derive(Component)]
pub struct AtmosphereProbeBindGroups {
    pub environment: BindGroup,
}

#[derive(Resource)]
pub struct AtmosphereProbeLayouts {
    pub environment: BindGroupLayoutDescriptor,
}

#[derive(Resource)]
pub struct AtmosphereProbePipeline {
    pub environment: CachedComputePipelineId,
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

    commands.insert_resource(AtmosphereProbeLayouts { environment });
}

/// Picks the atmosphere view that renders each probe this frame and records the exposure the
/// probe's environment map is stored with: the sky-view LUT it samples holds luminance pre-exposed
/// by that view.
///
/// A probe on an atmosphere view renders from that view. Any other probe renders from the view that
/// renders first, so that later views sample this frame's environment map.
pub(super) fn prepare_atmosphere_probe_sources(
    views: Query<(Entity, &ExtractedCamera), With<ExtractedAtmosphere>>,
    sorted_cameras: Res<SortedCameras>,
    probes: Query<(Entity, &AtmosphereEnvironmentMap)>,
    mut exposures: ResMut<EnvironmentMapExposures>,
    mut commands: Commands,
) {
    let first_view = sorted_cameras
        .0
        .iter()
        .find_map(|camera| views.get(camera.entity).ok());

    for (probe, render_env_map) in &probes {
        match views.get(probe).ok().or(first_view) {
            Some((view, camera)) => {
                exposures.insert(render_env_map.environment_map.id(), camera.exposure);
                commands.entity(probe).insert(AtmosphereProbeSource(view));
            }
            None => {
                commands.entity(probe).remove::<AtmosphereProbeSource>();
            }
        }
    }
}

pub(super) fn prepare_atmosphere_probe_bind_groups(
    probes: Query<(Entity, &AtmosphereProbeTextures, &AtmosphereProbeSource)>,
    view_textures: Query<&AtmosphereTextures>,
    render_device: Res<RenderDevice>,
    layouts: Res<AtmosphereProbeLayouts>,
    atmosphere_sampler: Res<LinearSampler>,
    view_uniforms: Res<ViewUniforms>,
    lights_uniforms: Res<LightMeta>,
    atmosphere_transforms: Res<AtmosphereTransforms>,
    atmosphere_uniforms: Res<ComponentUniforms<GpuAtmosphere>>,
    settings_uniforms: Res<ComponentUniforms<GpuAtmosphereSettings>>,
    pipeline_cache: Res<PipelineCache>,
    mut commands: Commands,
) {
    for (entity, textures, source) in &probes {
        let Ok(luts) = view_textures.get(source.0) else {
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
                // atmosphere luts and sampler
                (8, &luts.transmittance_lut.default_view),
                (9, &luts.multiscattering_lut.default_view),
                (10, &luts.sky_view_lut.default_view),
                (11, &luts.aerial_view_lut.default_view),
                (12, &atmosphere_sampler.0),
                // output 2D array texture
                (13, &textures.environment),
            )),
        );

        commands
            .entity(entity)
            .insert(AtmosphereProbeBindGroups { environment });
    }
}

pub(super) fn prepare_probe_textures(
    probes: Query<(Entity, &AtmosphereEnvironmentMap), Without<AtmosphereProbeTextures>>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut commands: Commands,
) {
    for (probe, render_env_map) in &probes {
        let Some(environment) = gpu_images.get(&render_env_map.environment_map) else {
            continue;
        };

        let environment_view = environment.texture.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });

        commands.entity(probe).insert(AtmosphereProbeTextures {
            environment: environment_view,
        });
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
        shader: load_embedded_asset!(asset_server.as_ref(), "environment.wesl"),
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

/// Allocates a cubemap and inserts filtered or unfiltered environment lighting.
pub fn on_insert_atmosphere_environment_map_light(
    insert: On<Insert<AtmosphereEnvironmentMapLight>>,
    lights: Query<(
        &AtmosphereEnvironmentMapLight,
        Has<AtmosphereEnvironmentMap>,
    )>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok((env_map_light, already_added_env_map)) = lights.get(insert.entity) else {
        return;
    };
    let Ok(mut entity) = commands.get_entity(insert.entity) else {
        return;
    };

    // Handle case where someone inserts a new AtmosphereEnvironmentMapLight component
    // on an entity that already has an AtmosphereEnvironmentMapLight component
    if already_added_env_map {
        entity.try_remove::<(
            AtmosphereEnvironmentMap,
            GeneratedEnvironmentMapLight,
            EnvironmentMapLight,
        )>();
    }

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
    environment_image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC;

    let environment_handle = images.add(environment_image);

    entity.insert(AtmosphereEnvironmentMap {
        environment_map: environment_handle.clone(),
        size: new_size,
    });

    if env_map_light.filtered {
        entity.insert(GeneratedEnvironmentMapLight {
            environment_map: environment_handle,
            intensity: env_map_light.intensity,
            affects_lightmapped_mesh_diffuse: env_map_light.affects_lightmapped_mesh_diffuse,
            ..default()
        });
    } else {
        entity.insert(EnvironmentMapLight {
            diffuse_map: environment_handle.clone(),
            specular_map: environment_handle,
            intensity: env_map_light.intensity,
            affects_lightmapped_mesh_diffuse: env_map_light.affects_lightmapped_mesh_diffuse,
            ..default()
        });
    }
}

/// Removes the cubemap and environment lighting inserted with it.
pub fn on_remove_atmosphere_environment_map_light(
    remove: On<Remove<AtmosphereEnvironmentMapLight>>,
    mut commands: Commands,
) {
    if let Ok(mut entity) = commands.get_entity(remove.entity) {
        entity.try_remove::<(
            AtmosphereEnvironmentMap,
            GeneratedEnvironmentMapLight,
            EnvironmentMapLight,
        )>();
    }
}

pub fn atmosphere_environment(
    view: ViewQuery<(
        Entity,
        &DynamicUniformIndex<GpuAtmosphere>,
        &DynamicUniformIndex<GpuAtmosphereSettings>,
        &AtmosphereTransformsOffset,
        &ViewUniformOffset,
        &ViewLightsUniformOffset,
    )>,
    probe_query: Query<(
        &AtmosphereProbeBindGroups,
        &AtmosphereEnvironmentMap,
        &AtmosphereProbeSource,
    )>,
    pipeline_cache: Res<PipelineCache>,
    pipelines: Res<AtmosphereProbePipeline>,
    mut ctx: RenderContext,
) {
    let Some(environment_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.environment)
    else {
        return;
    };

    let (
        view_entity,
        atmosphere_uniforms_offset,
        settings_uniforms_offset,
        atmosphere_transforms_offset,
        view_uniforms_offset,
        lights_uniforms_offset,
    ) = view.into_inner();

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "atmosphere_environment");

    for (bind_groups, env_map_light, source) in &probe_query {
        if source.0 != view_entity {
            continue;
        }
        let command_encoder = ctx.command_encoder();
        let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
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

    time_span.end(ctx.command_encoder());
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::Update;
    use bevy_asset::{uuid_handle, AssetId};
    use bevy_camera::{CameraOutputMode, ClearColorConfig, MsaaWriteback};
    use bevy_ecs::{schedule::ScheduleLabel, world::World};
    use bevy_math::{Mat4, Vec3};
    use bevy_render::camera::SortedCamera;

    fn atmosphere_view(order: isize, exposure: f32) -> (ExtractedCamera, ExtractedAtmosphere) {
        (
            ExtractedCamera {
                target: None,
                physical_viewport_size: None,
                physical_target_size: None,
                viewport: None,
                schedule: Update.intern(),
                order,
                output_mode: CameraOutputMode::default(),
                msaa_writeback: MsaaWriteback::default(),
                clear_color: ClearColorConfig::default(),
                sorted_camera_index_for_target: 0,
                exposure,
                hdr: true,
                tonemap_in_shader: false,
            },
            ExtractedAtmosphere {
                inner_radius: 6_360_000.0,
                outer_radius: 6_460_000.0,
                ground_albedo: Vec3::splat(0.3),
                medium: AssetId::default(),
                world_to_atmosphere: Mat4::IDENTITY,
            },
        )
    }

    fn probe(environment_map: Handle<Image>) -> AtmosphereEnvironmentMap {
        AtmosphereEnvironmentMap {
            environment_map,
            size: UVec2::splat(128),
        }
    }

    #[test]
    fn probes_record_the_exposure_of_the_view_that_renders_them() {
        let own_map = uuid_handle!("6d2f86a4-3c1b-4b9e-9a51-0c6f1e0b7a11");
        let standalone_map = uuid_handle!("0b9c7f3e-58d2-4e8a-b3a4-71d5c2e9f622");

        let mut world = World::new();
        world.init_resource::<EnvironmentMapExposures>();
        let late_view = world
            .spawn((atmosphere_view(1, 0.25), probe(own_map.clone())))
            .id();
        let first_view = world.spawn(atmosphere_view(0, 4.0)).id();
        let standalone = world.spawn(probe(standalone_map.clone())).id();
        let sorted = |entity, order| SortedCamera {
            entity,
            order,
            target: None,
            output_mode: CameraOutputMode::default(),
        };
        world.insert_resource(SortedCameras(vec![
            sorted(first_view, 0),
            sorted(late_view, 1),
        ]));

        world
            .run_system_cached(prepare_atmosphere_probe_sources)
            .unwrap();

        let source = |entity| world.get::<AtmosphereProbeSource>(entity).unwrap().0;
        assert_eq!(source(late_view), late_view);
        assert_eq!(source(standalone), first_view);
        let exposures = world.resource::<EnvironmentMapExposures>();
        assert_eq!(exposures.get(own_map.id()), 0.25);
        assert_eq!(exposures.get(standalone_map.id()), 4.0);
    }
}
