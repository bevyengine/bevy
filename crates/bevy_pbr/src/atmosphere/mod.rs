//! Procedural Atmospheric Scattering.
//!
//! This plugin implements [Hillaire's 2020 paper](https://sebh.github.io/publications/egsr2020.pdf)
//! on real-time atmospheric scattering. While it *will* work simply as a
//! procedural skybox, it also does much more. It supports dynamic time-of-
//! -day, multiple directional lights, and since it's applied as a post-processing
//! effect *on top* of the existing skybox, a starry skybox would automatically
//! show based on the time of day. Scattering in front of terrain (similar
//! to distance fog, but more complex) is handled as well, and takes into
//! account the directional light color and direction.
//!
//! To add the atmosphere to your scene, spawn an entity with the [`Atmosphere`] component and
//! [`bevy_transform::components::GlobalTransform`], optionally add [`CloudLayer`] for volumetric
//! clouds, and add [`AtmosphereSettings`] to each 3D camera that should render it. Detailed
//! documentation is on the [`Atmosphere`] component.
//!
//! Placement and scene scale come from the entity's transform. With several atmospheres in one
//! scene, each camera picks the atmosphere whose origin is closest in world space.
//!
//! Performance-wise, the effect should be fairly cheap since the LUTs (Look
//! Up Tables) that encode most of the data are small, and take advantage of the
//! fact that the atmosphere is symmetric. Performance is also proportional to
//! the number of directional lights in the scene. In order to tune
//! performance more finely, the [`AtmosphereSettings`] camera component
//! manages the size of each LUT and the sample count for each ray.
//!
//! Given how similar it is to [`crate::volumetric_fog`], it might be expected
//! that these two modules would work together well. However for now using both
//! at once is untested, and might not be physically accurate. These may be
//! integrated into a single module in the future.
//!
//! On web platforms, atmosphere rendering will look slightly different. Specifically, when calculating how light travels
//! through the atmosphere, we use a simpler averaging technique instead of the more
//! complex blending operations. This difference will be resolved for WebGPU in a future release.
//!
//! [Shadertoy]: https://www.shadertoy.com/view/slSXRW
//!
//! [Unreal Engine Implementation]: https://github.com/sebh/UnrealEngineSkyAtmosphere

mod environment;
pub mod fbm_noise;
mod node;
pub mod perlin_worley_noise;
pub mod detail_noise;
pub mod curl_noise;
pub mod resources;

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, AssetId};
use bevy_camera::{Camera3d, Hdr};
use bevy_core_pipeline::{
    core_3d::{main_opaque_pass_3d, main_transparent_pass_3d},
    schedule::{Core3d, Core3dSystems},
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Changed, With},
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
};
use bevy_light::{atmosphere::ScatteringMedium, Atmosphere};
use bevy_math::{Mat4, Quat, UVec2, UVec3, Vec2, Vec3};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponentPlugin, UniformComponentPlugin},
    render_resource::{DownlevelFlags, ShaderType, SpecializedRenderPipelines},
    renderer::RenderDevice,
    sync_component::{SyncComponent, SyncComponentPlugin},
    sync_world::RenderEntity,
    Extract, ExtractSchedule, RenderStartup,
};
use bevy_render::{
    render_resource::{TextureFormat, TextureUsages},
    renderer::RenderAdapter,
    Render, RenderApp, RenderSystems,
};
use bevy_transform::components::GlobalTransform;

use bevy_shader::load_shader_library;
use environment::{
    atmosphere_environment, init_atmosphere_probe_layout, init_atmosphere_probe_pipeline,
    on_insert_atmosphere_environment_map_light, on_remove_atmosphere_environment_map_light,
    prepare_atmosphere_probe_bind_groups, prepare_probe_textures, AtmosphereEnvironmentMap,
};
use fbm_noise::{
    generate_fbm_noise_once, init_fbm_noise_params_buffer, init_fbm_noise_texture,
    prepare_fbm_noise_bind_group, FbmNoiseBindGroupLayout, FbmNoisePipeline, NoiseGenerated,
};
use node::{atmosphere_luts, render_sky};
use perlin_worley_noise::{
    generate_perlin_worley_noise_once, init_perlin_worley_noise_params_buffer,
    init_perlin_worley_noise_texture, prepare_perlin_worley_noise_bind_group,
    PerlinWorleyNoiseBindGroupLayout, PerlinWorleyNoiseGenerated, PerlinWorleyNoisePipeline,
};
use detail_noise::{
    generate_detail_noise_once, init_detail_noise_params_buffer, init_detail_noise_texture,
    prepare_detail_noise_bind_group, DetailNoiseBindGroupLayout, DetailNoiseGenerated,
    DetailNoisePipeline,
};
use curl_noise::{
    generate_curl_noise_once, init_curl_noise_params_buffer, init_curl_noise_texture,
    prepare_curl_noise_bind_group, CurlNoiseBindGroupLayout, CurlNoiseGenerated, CurlNoisePipeline,
};
use perlin_worley_noise::{NoiseMipDownsampleBindGroupLayout, NoiseMipDownsamplePipeline};
use resources::{
    prepare_atmosphere_transforms, prepare_atmosphere_uniforms, queue_render_sky_pipelines,
    AtmosphereTransforms, GpuAtmosphere, RenderSkyBindGroupLayouts,
};
use tracing::warn;

use crate::resources::prepare_atmosphere_buffers;

use self::resources::{
    prepare_atmosphere_bind_groups, prepare_atmosphere_textures,
    prepare_cloud_shadow_temporal_params, prepare_clouds_temporal_params, AtmosphereBindGroupLayouts, AtmosphereBindGroups,
    AtmosphereLutPipelines, AtmosphereSampler, AtmosphereTextures, AtmosphereTransformsOffset,
    CloudNoiseSampler, CloudShadowTemporalParamsBuffer, CloudShadowTemporalState,
    RenderSkyPipelineId,
};

#[doc(hidden)]
pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "types.wesl");
        load_shader_library!(app, "functions.wesl");
        load_shader_library!(app, "bruneton_functions.wesl");
        load_shader_library!(app, "bindings.wesl");
        load_shader_library!(app, "shadows.wesl");
        load_shader_library!(app, "clouds.wesl");

        embedded_asset!(app, "transmittance_lut.wesl");
        embedded_asset!(app, "multiscattering_lut.wesl");
        embedded_asset!(app, "sky_view_lut.wesl");
        embedded_asset!(app, "aerial_view_lut.wesl");
        embedded_asset!(app, "render_sky.wesl");
        embedded_asset!(app, "environment.wesl");
        embedded_asset!(app, "fbm_noise_3d.wesl");
        embedded_asset!(app, "perlin_worley_noise_3d.wesl");
        embedded_asset!(app, "detail_noise_3d.wesl");
        embedded_asset!(app, "curl_noise_2d.wesl");
        embedded_asset!(app, "noise_mip_downsample_3d.wesl");
        embedded_asset!(app, "cloud_shadow_map.wesl");
        embedded_asset!(app, "cloud_shadow_filter.wesl");
        embedded_asset!(app, "cloud_shadow_temporal.wesl");
        embedded_asset!(app, "clouds_raymarch.wesl");
        embedded_asset!(app, "clouds_reconstruct.wesl");

        app.add_plugins((
            ExtractComponentPlugin::<AtmosphereEnvironmentMap>::default(),
            SyncComponentPlugin::<AtmosphereSettings>::default(),
            UniformComponentPlugin::<GpuAtmosphere>::default(),
            UniformComponentPlugin::<GpuAtmosphereSettings>::default(),
            UniformComponentPlugin::<CloudLayer>::default(),
        ))
        .add_observer(on_insert_atmosphere_environment_map_light)
        .add_observer(on_remove_atmosphere_environment_map_light);

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(ExtractSchedule, extract_atmosphere);
        }
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_adapter = render_app.world().resource::<RenderAdapter>();

        if !render_adapter
            .get_downlevel_capabilities()
            .flags
            .contains(DownlevelFlags::COMPUTE_SHADERS)
        {
            warn!("AtmospherePlugin not loaded. GPU lacks support for compute shaders.");
            return;
        }

        if !render_adapter
            .get_texture_format_features(TextureFormat::Rgba16Float)
            .allowed_usages
            .contains(TextureUsages::STORAGE_BINDING)
        {
            warn!("AtmospherePlugin not loaded. GPU lacks support: TextureFormat::Rgba16Float does not support TextureUsages::STORAGE_BINDING.");
            return;
        }

        // Check the `RenderDevice` in addition to the `RenderAdapter`. The
        // former takes the `WGPU_SETTINGS_PRIO` environment variable into
        // account, and the latter doesn't.
        let render_device = render_app.world().resource::<RenderDevice>();
        if render_device.limits().max_storage_textures_per_shader_stage == 0 {
            warn!("AtmospherePlugin not loaded. GPU lacks support: `max_storage_textures_per_shader_stage` is 0");
            return;
        }

        render_app
            .insert_resource(AtmosphereBindGroupLayouts::new())
            .init_resource::<RenderSkyBindGroupLayouts>()
            .init_resource::<AtmosphereSampler>()
            .init_resource::<CloudNoiseSampler>()
            .init_resource::<AtmosphereLutPipelines>()
            .init_resource::<AtmosphereTransforms>()
            .init_resource::<SpecializedRenderPipelines<RenderSkyBindGroupLayouts>>()
            .init_resource::<FbmNoiseBindGroupLayout>()
            .init_resource::<FbmNoisePipeline>()
            .init_resource::<NoiseGenerated>()
            .init_resource::<PerlinWorleyNoiseBindGroupLayout>()
            .init_resource::<PerlinWorleyNoisePipeline>()
            .init_resource::<PerlinWorleyNoiseGenerated>()
            .init_resource::<NoiseMipDownsampleBindGroupLayout>()
            .init_resource::<NoiseMipDownsamplePipeline>()
            .init_resource::<DetailNoiseBindGroupLayout>()
            .init_resource::<DetailNoisePipeline>()
            .init_resource::<DetailNoiseGenerated>()
            .init_resource::<CurlNoiseBindGroupLayout>()
            .init_resource::<CurlNoisePipeline>()
            .init_resource::<CurlNoiseGenerated>()
            .init_resource::<CloudShadowTemporalState>()
            .init_resource::<CloudShadowTemporalParamsBuffer>()
            .add_systems(
                RenderStartup,
                (
                    init_atmosphere_probe_layout,
                    init_atmosphere_probe_pipeline,
                    init_fbm_noise_texture,
                    init_fbm_noise_params_buffer,
                    init_perlin_worley_noise_texture,
                    init_perlin_worley_noise_params_buffer,
                    init_detail_noise_texture,
                    init_detail_noise_params_buffer,
                    init_curl_noise_texture,
                    init_curl_noise_params_buffer,
                )
                    .chain(),
            )
            .add_systems(
                Render,
                (
                    configure_camera_depth_usages.in_set(RenderSystems::PrepareViews),
                    queue_render_sky_pipelines.in_set(RenderSystems::Queue),
                    prepare_atmosphere_textures.in_set(RenderSystems::PrepareResources),
                    prepare_probe_textures
                        .in_set(RenderSystems::PrepareResources)
                        .after(prepare_atmosphere_textures),
                    prepare_atmosphere_uniforms
                        .in_set(RenderSystems::Prepare)
                        .before(RenderSystems::PrepareResources),
                    prepare_atmosphere_probe_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                    prepare_atmosphere_transforms.in_set(RenderSystems::PrepareResources),
                    prepare_cloud_shadow_temporal_params
                        .in_set(RenderSystems::PrepareBindGroups)
                        .before(prepare_atmosphere_bind_groups),
                    prepare_clouds_temporal_params
                        .in_set(RenderSystems::PrepareBindGroups)
                        .before(prepare_atmosphere_bind_groups),
                    prepare_atmosphere_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                    prepare_fbm_noise_bind_group.in_set(RenderSystems::PrepareBindGroups),
                    prepare_perlin_worley_noise_bind_group.in_set(RenderSystems::PrepareBindGroups),
                    prepare_detail_noise_bind_group.in_set(RenderSystems::PrepareBindGroups),
                    prepare_curl_noise_bind_group.in_set(RenderSystems::PrepareBindGroups),
                    generate_fbm_noise_once
                        .in_set(RenderSystems::Render)
                        .after(RenderSystems::PrepareBindGroups),
                    generate_perlin_worley_noise_once
                        .in_set(RenderSystems::Render)
                        .after(RenderSystems::PrepareBindGroups),
                    generate_detail_noise_once
                        .in_set(RenderSystems::Render)
                        .after(RenderSystems::PrepareBindGroups),
                    generate_curl_noise_once
                        .in_set(RenderSystems::Render)
                        .after(RenderSystems::PrepareBindGroups),
                    prepare_atmosphere_buffers.in_set(RenderSystems::PrepareResources),
                ),
            )
            .add_systems(
                Core3d,
                (
                    (atmosphere_luts, atmosphere_environment)
                        .chain()
                        .after(Core3dSystems::Prepass)
                        .before(Core3dSystems::MainPass),
                    render_sky
                        .after(main_opaque_pass_3d)
                        .before(main_transparent_pass_3d),
                ),
            );
    }
}

/// For each camera with [`AtmosphereSettings`], picks the nearest [`Atmosphere`] by world-space
/// distance to its origin, copies it as [`ExtractedAtmosphere`], optionally copies [`CloudLayer`],
/// and builds [`GpuAtmosphereSettings`].
pub fn extract_atmosphere(
    mut commands: Commands,
    atmosphere_entities: Extract<
        Query<(Entity, &Atmosphere, &GlobalTransform, Option<&CloudLayer>)>,
    >,
    cameras: Extract<
        Query<(RenderEntity, Option<&AtmosphereSettings>, &GlobalTransform), With<Camera3d>>,
    >,
) {
    let candidates: Vec<(Entity, &Atmosphere, &GlobalTransform, Option<&CloudLayer>)> =
        atmosphere_entities.iter().collect();

    for (render_entity, settings, cam_global) in &cameras {
        // Remove any stale render-world state when AtmosphereSettings is removed or candidates is empty
        let (Some(settings), false) = (settings, candidates.is_empty()) else {
            commands.entity(render_entity).try_remove::<(
                ExtractedAtmosphere,
                GpuAtmosphereSettings,
                GpuAtmosphere,
                AtmosphereTextures,
                AtmosphereTransformsOffset,
                RenderSkyPipelineId,
                AtmosphereBindGroups,
                CloudLayer,
            )>();
            continue;
        };

        let cam_world = cam_global.translation();
        let selected = candidates
            .iter()
            .min_by(|(ea, _, gt_a, _), (eb, _, gt_b, _)| {
                let da = cam_world.distance(gt_a.translation());
                let db = cam_world.distance(gt_b.translation());
                da.total_cmp(&db).then_with(|| ea.cmp(eb))
            })
            .expect("checked non-empty above");
        let atmo = selected.1;
        let gt = selected.2;
        let cloud_layer = selected.3;
        // Atmospheres are spherically symmetric, so ignore their orientation.
        let (scale, _, translation) = gt.to_scale_rotation_translation();

        let atmosphere_to_world =
            Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, translation);
        let extracted = ExtractedAtmosphere {
            inner_radius: atmo.inner_radius,
            outer_radius: atmo.outer_radius,
            ground_albedo: atmo.ground_albedo,
            medium: atmo.medium.id(),
            world_to_atmosphere: atmosphere_to_world.inverse(),
            atmosphere_to_world,
        };
        commands.entity(render_entity).insert(extracted);
        commands
            .entity(render_entity)
            .insert(GpuAtmosphereSettings::from(settings.clone()));
        if let Some(cloud_layer) = cloud_layer {
            commands.entity(render_entity).insert(cloud_layer.clone());
        } else {
            commands.entity(render_entity).remove::<CloudLayer>();
        }
    }
}

/// The render-world representation of an `Atmosphere`, but which
/// hasn't been converted into shader uniforms yet.
#[derive(Clone, Component)]
pub struct ExtractedAtmosphere {
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub ground_albedo: Vec3,
    pub medium: AssetId<ScatteringMedium>,
    pub world_to_atmosphere: Mat4,
    pub atmosphere_to_world: Mat4,
}

/// This component controls the resolution of the atmosphere LUTs, and
/// how many samples are used when computing them.
///
/// The transmittance LUT stores the transmittance from a point in the
/// atmosphere to the outer edge of the atmosphere in any direction,
/// parametrized by the point's radius and the cosine of the zenith angle
/// of the ray.
///
/// The multiscattering LUT stores the factor representing luminance scattered
/// towards the camera with scattering order >2, parametrized by the point's radius
/// and the cosine of the zenith angle of the sun.
///
/// The sky-view lut is essentially the actual skybox, storing the light scattered
/// towards the camera in every direction with a cubemap.
///
/// The aerial-view lut is a 3d LUT fit to the view frustum, which stores the luminance
/// scattered towards the camera at each point (RGB channels), alongside the average
/// transmittance to that point (A channel).
#[derive(Clone, Component, Reflect)]
#[reflect(Clone, Default)]
#[require(Hdr)]
pub struct AtmosphereSettings {
    /// The size of the transmittance LUT
    pub transmittance_lut_size: UVec2,

    /// The size of the multiscattering LUT
    pub multiscattering_lut_size: UVec2,

    /// The size of the sky-view LUT.
    pub sky_view_lut_size: UVec2,

    /// The size of the aerial-view LUT.
    pub aerial_view_lut_size: UVec3,

    /// The number of points to sample along each ray when
    /// computing the transmittance LUT
    pub transmittance_lut_samples: u32,

    /// The number of rays to sample when computing each
    /// pixel of the multiscattering LUT
    pub multiscattering_lut_dirs: u32,

    /// The number of points to sample when integrating along each
    /// multiscattering ray
    pub multiscattering_lut_samples: u32,

    /// The number of points to sample along each ray when
    /// computing the sky-view LUT.
    pub sky_view_lut_samples: u32,

    /// The number of points to sample for each slice along the z-axis
    /// of the aerial-view LUT.
    pub aerial_view_lut_samples: u32,

    /// The maximum distance from the camera to evaluate the
    /// aerial view LUT. The slices along the z-axis of the
    /// texture will be distributed linearly from the camera
    /// to this value.
    ///
    /// units: m
    pub aerial_view_lut_max_distance: f32,

    /// The number of points to sample for each fragment when the using
    /// ray marching to render the sky
    pub sky_max_samples: u32,

    /// The rendering method to use for the atmosphere.
    pub rendering_method: AtmosphereMode,

    /// Whether to sample shadow maps for volumetric shadow shafts.
    pub shadows_enabled: bool,
    /// Resolution of the cloud shadow map used to shadow directional lights when
    /// rendering in [`AtmosphereMode::Raymarched`].
    ///
    /// Higher resolution improves stability and reduces aliasing at grazing angles,
    /// but increases compute cost.
    pub cloud_shadow_map_size: UVec2,

    /// Half-extent of the orthographic cloud shadow map volume in meters.
    ///
    /// The shadow map covers a square region of size \(2 \cdot extent\) around the view
    /// position, oriented in light space.
    pub cloud_shadow_map_extent: f32,

    /// Number of samples used when building the cloud shadow map.
    pub cloud_shadow_map_samples: u32,

    /// Strength multiplier applied to the cloud shadow map's stored extinction / optical depth.
    pub cloud_shadow_map_strength: f32,

    /// Number of spatial filter iterations applied to the cloud shadow map after tracing.
    ///
    /// This is the main knob to trade performance for stability once the shadow map uses jittered sampling.
    /// Values above 4 are usually not worth it.
    pub cloud_shadow_map_spatial_filter_iterations: u32,

    /// Time-slices the cloud shadow map tracing: each frame only one texel out of every
    /// `N x N` tile is re-traced (in a Bayer order), and the remaining texels are reprojected
    /// from the previous frame's map (see Bauer 2019, "Creating the Atmospheric World of RDR2":
    /// the cloud shadow map updates 1 ray per 4x4 tile per frame).
    ///
    /// `1` traces every texel every frame. Values above `1` cut the tracing cost by roughly
    /// `N * N` and implicitly enable the temporal reprojection pass (blending of the traced texels
    /// with history is still controlled by [`Self::cloud_shadow_temporal_enabled`]). The spatial
    /// filter is skipped while time-slicing, since it would repeatedly blur the reprojected texels.
    /// Supported values: `1`, `2` and `4`.
    pub cloud_shadow_map_update_interleave: u32,

    /// Enable temporal filtering for the cloud shadow map to reduce edge flickering.
    ///
    /// When enabled, the shadow map is blended with the previous frame's result,
    /// stabilizing edges over time. Based on Unreal's approach.
    pub cloud_shadow_temporal_enabled: bool,

    /// Weight of the current frame in the temporal blend (0.0–1.0).
    ///
    /// Lower values (e.g. 0.1–0.2) produce more stable shadows but slower convergence.
    /// Higher values (e.g. 0.5) converge faster but may retain more flicker.
    pub cloud_shadow_temporal_alpha: f32,

    /// Light rotation threshold (degrees) beyond which history is invalidated.
    ///
    /// When the sun moves more than this angle, temporal accumulation is reset
    /// to avoid ghosting. Unreal uses 10°.
    pub cloud_shadow_temporal_light_rotation_cut_deg: f32,
}

impl Default for AtmosphereSettings {
    fn default() -> Self {
        Self {
            transmittance_lut_size: UVec2::new(256, 128),
            transmittance_lut_samples: 40,
            multiscattering_lut_size: UVec2::new(32, 32),
            multiscattering_lut_dirs: 64,
            multiscattering_lut_samples: 20,
            sky_view_lut_size: UVec2::new(400, 200),
            sky_view_lut_samples: 16,
            aerial_view_lut_size: UVec3::new(32, 32, 32),
            aerial_view_lut_samples: 10,
            aerial_view_lut_max_distance: 3.2e4,
            sky_max_samples: 16,
            rendering_method: AtmosphereMode::LookupTexture,
            shadows_enabled: true,

            // Cloud shadow map defaults (only used by Raymarched mode).
            cloud_shadow_map_size: UVec2::new(1024, 1024),
            cloud_shadow_map_extent: 64_000.0,
            cloud_shadow_map_samples: 48,
            cloud_shadow_map_strength: 1.0,
            cloud_shadow_map_spatial_filter_iterations: 0,
            cloud_shadow_map_update_interleave: 4,
            cloud_shadow_temporal_enabled: false,
            cloud_shadow_temporal_alpha: 0.15,
            cloud_shadow_temporal_light_rotation_cut_deg: 10.0,
        }
    }
}

#[derive(Clone, Component, Reflect, ShaderType)]
#[reflect(Default)]
pub struct GpuAtmosphereSettings {
    pub transmittance_lut_size: UVec2,
    pub multiscattering_lut_size: UVec2,
    pub sky_view_lut_size: UVec2,
    pub aerial_view_lut_size: UVec3,
    pub transmittance_lut_samples: u32,
    pub multiscattering_lut_dirs: u32,
    pub multiscattering_lut_samples: u32,
    pub sky_view_lut_samples: u32,
    pub aerial_view_lut_samples: u32,
    pub aerial_view_lut_max_distance: f32,
    pub sky_max_samples: u32,
    pub rendering_method: u32,
    pub shadows_enabled: u32,
    pub cloud_shadow_map_size: UVec2,
    pub cloud_shadow_map_extent: f32,
    pub cloud_shadow_map_samples: u32,
    pub cloud_shadow_map_strength: f32,
    pub cloud_shadow_map_spatial_filter_iterations: u32,
    pub cloud_shadow_map_update_interleave: u32,
    pub cloud_shadow_temporal_enabled: u32,
    pub cloud_shadow_temporal_alpha: f32,
    pub cloud_shadow_temporal_light_rotation_cut_deg: f32,
}

impl GpuAtmosphereSettings {
    /// Whether the cloud shadow map goes through the temporal reprojection pass, either because
    /// temporal filtering is enabled or because the tracing is time-sliced.
    pub fn cloud_shadow_uses_temporal_pass(&self) -> bool {
        self.cloud_shadow_temporal_enabled != 0 || self.cloud_shadow_map_update_interleave > 1
    }
}

impl Default for GpuAtmosphereSettings {
    fn default() -> Self {
        AtmosphereSettings::default().into()
    }
}

impl From<AtmosphereSettings> for GpuAtmosphereSettings {
    fn from(s: AtmosphereSettings) -> Self {
        Self {
            transmittance_lut_size: s.transmittance_lut_size,
            multiscattering_lut_size: s.multiscattering_lut_size,
            sky_view_lut_size: s.sky_view_lut_size,
            aerial_view_lut_size: s.aerial_view_lut_size,
            transmittance_lut_samples: s.transmittance_lut_samples,
            multiscattering_lut_dirs: s.multiscattering_lut_dirs,
            multiscattering_lut_samples: s.multiscattering_lut_samples,
            sky_view_lut_samples: s.sky_view_lut_samples,
            aerial_view_lut_samples: s.aerial_view_lut_samples,
            aerial_view_lut_max_distance: s.aerial_view_lut_max_distance,
            sky_max_samples: s.sky_max_samples,
            rendering_method: s.rendering_method as u32,
            shadows_enabled: s.shadows_enabled as u32,
            cloud_shadow_map_size: s.cloud_shadow_map_size,
            cloud_shadow_map_extent: s.cloud_shadow_map_extent,
            cloud_shadow_map_samples: s.cloud_shadow_map_samples,
            cloud_shadow_map_strength: s.cloud_shadow_map_strength,
            cloud_shadow_map_spatial_filter_iterations: s
                .cloud_shadow_map_spatial_filter_iterations,
            cloud_shadow_map_update_interleave: match s.cloud_shadow_map_update_interleave {
                0 | 1 => 1,
                2 | 3 => 2,
                _ => 4,
            },
            cloud_shadow_temporal_enabled: s.cloud_shadow_temporal_enabled as u32,
            cloud_shadow_temporal_alpha: s.cloud_shadow_temporal_alpha,
            cloud_shadow_temporal_light_rotation_cut_deg: s
                .cloud_shadow_temporal_light_rotation_cut_deg,
        }
    }
}

impl SyncComponent<RenderApp> for AtmosphereSettings {
    type Target = GpuAtmosphereSettings;
}

fn configure_camera_depth_usages(
    mut cameras: Query<&mut Camera3d, (Changed<Camera3d>, With<ExtractedAtmosphere>)>,
) {
    for mut camera in &mut cameras {
        camera.depth_texture_usages.0 |= TextureUsages::TEXTURE_BINDING.bits();
    }
}

/// Selects how the atmosphere is rendered. Choose based on scene scale and
/// volumetric shadow quality, and based on performance needs.
#[repr(u32)]
#[derive(Clone, Default, Reflect, Copy)]
pub enum AtmosphereMode {
    /// High-performance solution tailored to scenes that are mostly inside of the atmosphere.
    /// Uses a set of lookup textures to approximate scattering integration.
    /// Slightly less accurate for very long-distance/space views (lighting precision
    /// tapers as the camera moves far from the scene origin) and for sharp volumetric
    /// (cloud/fog) shadows.
    #[default]
    LookupTexture = 0,
    /// Slower, more accurate rendering method for any type of scene.
    /// Integrates the scattering numerically with raymarching and produces sharp volumetric
    /// (cloud/fog) shadows.
    /// Best for cinematic shots, planets seen from orbit, and scenes requiring
    /// accurate long-distance lighting.
    Raymarched = 1,
}

/// Procedural / painted weather input for volumetric clouds (design.md §3.2).
///
/// Both modes resolve to the same GPU weather map binding (512² RGBA:
/// coverage, type, density scale, spare). Procedural is the default and is
/// generated once on the GPU at startup; a `Texture` override can be wired
/// later without changing the density sampler.
#[derive(Clone, Component, Reflect)]
#[reflect(Clone, Default)]
pub struct CloudWeather {
    /// World-space extent covered by one weather-map tile, in meters.
    pub scale: f32,
    /// Seed for procedural generation (reserved; current generator is fixed).
    pub seed: u32,
    /// Base cloud type in [0, 1] (0 stratus → 1 cumulus), blended with the map's G.
    pub cloud_type: f32,
    /// How much the weather-map type channel can deviate from `cloud_type`.
    pub type_variation: f32,
    /// UV offset applied when sampling the weather map.
    pub offset: Vec2,
    /// Meters/second advection of the weather UV offset (applied on the CPU later).
    pub offset_velocity: Vec2,
}

impl Default for CloudWeather {
    fn default() -> Self {
        Self {
            scale: 32_000.0,
            seed: 0,
            cloud_type: 0.5,
            type_variation: 0.5,
            offset: Vec2::ZERO,
            offset_velocity: Vec2::ZERO,
        }
    }
}

/// Component that adds a volumetric cloud layer to the atmosphere.
///
/// Add this component to an [`Atmosphere`] entity to enable volumetric clouds for cameras
/// that select that atmosphere.
#[derive(Clone, Component, Reflect, ShaderType)]
#[reflect(Clone, Default)]
pub struct CloudLayer {
    /// Altitude at which the cloud layer starts (from planet center)
    /// units: m
    pub cloud_layer_start: f32,

    /// Altitude at which the cloud layer ends (from planet center)
    /// units: m
    pub cloud_layer_end: f32,

    /// Density multiplier for the clouds
    pub cloud_density: f32,

    /// Absorption coefficient for clouds
    pub cloud_absorption: f32,

    /// Scattering coefficient for clouds
    pub cloud_scattering: f32,

    /// Scale of the noise texture in world space
    pub noise_scale: f32,

    /// Offset for animating the noise texture
    pub noise_offset: Vec3,

    /// Scale of the detail noise in world space (smaller = higher frequency detail).
    /// units: m
    pub detail_noise_scale: f32,

    /// Strength of the detail noise modulation in [0,1].
    /// 0 = disabled, 1 = full modulation.
    pub detail_strength: f32,
}

impl Default for CloudLayer {
    fn default() -> Self {
        Self {
            cloud_layer_start: 6_362_000.0, // 1km above Earth's surface
            cloud_layer_end: 6_363_000.0,   // 5km above Earth's surface
            cloud_density: 1.0,
            cloud_absorption: 0.00005, // Physically correct: ~0.00005 m^-1 per unit density
            cloud_scattering: 0.0008,  // Physically correct: ~0.0008 m^-1 per unit density
            noise_scale: 64_000.0,
            noise_offset: Vec3::ZERO,
            detail_noise_scale: 16_000.0,
            detail_strength: 1.0,
        }
    }
}

impl SyncComponent<RenderApp> for CloudLayer {
    type Target = Self;
}
