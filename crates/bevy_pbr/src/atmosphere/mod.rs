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
//! [`bevy_transform::components::GlobalTransform`], and add [`AtmosphereSettings`] to each
//! 3D camera that should render it. Detailed documentation is on the [`Atmosphere`] component.
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
mod node;
pub mod resources;

use bevy_app::{App, Plugin, Update};
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
use bevy_math::{Mat4, UVec2, UVec3, Vec3};
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
    GpuResourceAppExt, Render, RenderApp, RenderSystems,
};
use bevy_transform::components::GlobalTransform;

use bevy_shader::load_shader_library;
use environment::{
    atmosphere_environment, init_atmosphere_probe_layout, init_atmosphere_probe_pipeline,
    prepare_atmosphere_probe_bind_groups, prepare_atmosphere_probe_components,
    prepare_probe_textures, AtmosphereEnvironmentMap,
};
use node::{atmosphere_luts, render_sky};
use resources::{
    prepare_atmosphere_transforms, prepare_atmosphere_uniforms, queue_render_sky_pipelines,
    AtmosphereTransforms, GpuAtmosphere, RenderSkyBindGroupLayouts,
};
use tracing::warn;

use crate::resources::{init_atmosphere_buffer, write_atmosphere_buffer};

use self::resources::{
    prepare_atmosphere_bind_groups, prepare_atmosphere_textures, AtmosphereBindGroupLayouts,
    AtmosphereLutPipelines, AtmosphereSampler,
};

#[doc(hidden)]
pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "types.wgsl");
        load_shader_library!(app, "functions.wgsl");
        load_shader_library!(app, "bruneton_functions.wgsl");
        load_shader_library!(app, "bindings.wgsl");

        embedded_asset!(app, "transmittance_lut.wgsl");
        embedded_asset!(app, "multiscattering_lut.wgsl");
        embedded_asset!(app, "sky_view_lut.wgsl");
        embedded_asset!(app, "aerial_view_lut.wgsl");
        embedded_asset!(app, "render_sky.wgsl");
        embedded_asset!(app, "environment.wgsl");

        app.add_plugins((
            ExtractComponentPlugin::<AtmosphereEnvironmentMap>::default(),
            SyncComponentPlugin::<AtmosphereSettings>::default(),
            UniformComponentPlugin::<GpuAtmosphere>::default(),
            UniformComponentPlugin::<GpuAtmosphereSettings>::default(),
        ))
        .add_systems(Update, prepare_atmosphere_probe_components);

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
            .init_gpu_resource::<RenderSkyBindGroupLayouts>()
            .init_gpu_resource::<AtmosphereSampler>()
            .init_gpu_resource::<AtmosphereLutPipelines>()
            .init_gpu_resource::<AtmosphereTransforms>()
            .init_gpu_resource::<SpecializedRenderPipelines<RenderSkyBindGroupLayouts>>()
            .add_systems(
                RenderStartup,
                (
                    init_atmosphere_probe_layout,
                    init_atmosphere_probe_pipeline,
                    init_atmosphere_buffer,
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
                    prepare_atmosphere_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                    write_atmosphere_buffer.in_set(RenderSystems::PrepareResources),
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
/// distance to its origin, copies it as [`ExtractedAtmosphere`], and builds [`GpuAtmosphereSettings`].
pub fn extract_atmosphere(
    mut commands: Commands,
    atmosphere_entities: Extract<Query<(Entity, &Atmosphere, &GlobalTransform)>>,
    cameras: Extract<Query<(RenderEntity, &AtmosphereSettings, &GlobalTransform), With<Camera3d>>>,
) {
    let candidates: Vec<(Entity, &Atmosphere, &GlobalTransform)> =
        atmosphere_entities.iter().collect();

    if candidates.is_empty() {
        for (render_entity, ..) in &cameras {
            commands
                .entity(render_entity)
                .remove::<ExtractedAtmosphere>();
            commands
                .entity(render_entity)
                .remove::<GpuAtmosphereSettings>();
        }
        return;
    }

    for (render_entity, settings, cam_global) in &cameras {
        let cam_world = cam_global.translation();
        let selected = candidates
            .iter()
            .min_by(|(ea, _, gt_a), (eb, _, gt_b)| {
                let da = cam_world.distance(gt_a.translation());
                let db = cam_world.distance(gt_b.translation());
                da.total_cmp(&db).then_with(|| ea.cmp(eb))
            })
            .expect("checked non-empty above");
        let atmo = selected.1;
        let gt = selected.2;

        let extracted = ExtractedAtmosphere {
            inner_radius: atmo.inner_radius,
            outer_radius: atmo.outer_radius,
            ground_albedo: atmo.ground_albedo,
            medium: atmo.medium.id(),
            world_to_atmosphere: gt.to_matrix().inverse(),
        };
        commands.entity(render_entity).insert(extracted);
        commands
            .entity(render_entity)
            .insert(GpuAtmosphereSettings::from(settings.clone()));
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
        }
    }
}

impl SyncComponent for AtmosphereSettings {
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
