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
//! Adding the [`Atmosphere`] component to a 3d camera will enable the effect,
//! which by default is set to look similar to Earth's atmosphere. See the
//! documentation on the component itself for information regarding its fields.
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
use bevy_asset::{embedded_asset, AssetId, Handle};
use bevy_camera::Camera3d;
use bevy_core_pipeline::core_3d::graph::Node3d;
use bevy_ecs::{
    component::Component,
    query::{Changed, QueryItem, With},
    schedule::IntoScheduleConfigs,
    system::{lifetimeless::Read, Query},
};
use bevy_math::{UVec2, UVec3, Vec3};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::UniformComponentPlugin,
    render_resource::{DownlevelFlags, ShaderType, SpecializedRenderPipelines},
    view::Hdr,
    RenderStartup,
};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{RenderGraphExt, ViewNodeRunner},
    render_resource::{TextureFormat, TextureUsages},
    renderer::RenderAdapter,
    Render, RenderApp, RenderSystems,
};

use bevy_core_pipeline::core_3d::graph::Core3d;
use bevy_shader::load_shader_library;
use environment::{
    init_atmosphere_probe_layout, init_atmosphere_probe_pipeline,
    prepare_atmosphere_probe_bind_groups, prepare_atmosphere_probe_components,
    prepare_probe_textures, AtmosphereEnvironmentMap, EnvironmentNode,
};
use resources::{
    prepare_atmosphere_transforms, prepare_atmosphere_uniforms, queue_render_sky_pipelines,
    AtmosphereTransforms, GpuAtmosphere, RenderSkyBindGroupLayouts,
};
use tracing::warn;

use crate::{
    medium::ScatteringMedium,
    resources::{init_atmosphere_buffer, write_atmosphere_buffer},
};

use self::{
    node::{AtmosphereLutsNode, AtmosphereNode, RenderSkyNode},
    resources::{
        prepare_atmosphere_bind_groups, prepare_atmosphere_textures, AtmosphereBindGroupLayouts,
        AtmosphereLutPipelines, AtmosphereSampler,
    },
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
            ExtractComponentPlugin::<Atmosphere>::default(),
            ExtractComponentPlugin::<GpuAtmosphereSettings>::default(),
            ExtractComponentPlugin::<AtmosphereEnvironmentMap>::default(),
            UniformComponentPlugin::<GpuAtmosphere>::default(),
            UniformComponentPlugin::<GpuAtmosphereSettings>::default(),
        ))
        .add_systems(Update, prepare_atmosphere_probe_components);
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

        render_app
            .insert_resource(AtmosphereBindGroupLayouts::new())
            .init_resource::<RenderSkyBindGroupLayouts>()
            .init_resource::<AtmosphereSampler>()
            .init_resource::<AtmosphereLutPipelines>()
            .init_resource::<AtmosphereTransforms>()
            .init_resource::<SpecializedRenderPipelines<RenderSkyBindGroupLayouts>>()
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
                    configure_camera_depth_usages.in_set(RenderSystems::ManageViews),
                    queue_render_sky_pipelines.in_set(RenderSystems::Queue),
                    prepare_atmosphere_textures.in_set(RenderSystems::PrepareResources),
                    prepare_probe_textures
                        .in_set(RenderSystems::PrepareResources)
                        .after(prepare_atmosphere_textures),
                    prepare_atmosphere_uniforms
                        .before(RenderSystems::PrepareResources)
                        .after(RenderSystems::PrepareAssets),
                    prepare_atmosphere_probe_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                    prepare_atmosphere_transforms.in_set(RenderSystems::PrepareResources),
                    prepare_atmosphere_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                    write_atmosphere_buffer.in_set(RenderSystems::PrepareResources),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<AtmosphereLutsNode>>(
                Core3d,
                AtmosphereNode::RenderLuts,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    // END_PRE_PASSES -> RENDER_LUTS -> MAIN_PASS
                    Node3d::EndPrepasses,
                    AtmosphereNode::RenderLuts,
                    Node3d::StartMainPass,
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<RenderSkyNode>>(
                Core3d,
                AtmosphereNode::RenderSky,
            )
            .add_render_graph_node::<EnvironmentNode>(Core3d, AtmosphereNode::Environment)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::MainOpaquePass,
                    AtmosphereNode::RenderSky,
                    Node3d::MainTransparentPass,
                ),
            );
    }
}

/// Enables atmospheric scattering for an HDR camera.
#[derive(Clone, Component)]
#[require(AtmosphereSettings, Hdr)]
pub struct Atmosphere {
    /// Radius of the planet
    ///
    /// units: m
    pub bottom_radius: f32,

    /// Radius at which we consider the atmosphere to 'end' for our
    /// calculations (from center of planet)
    ///
    /// units: m
    pub top_radius: f32,

    /// An approximation of the average albedo (or color, roughly) of the
    /// planet's surface. This is used when calculating multiscattering.
    ///
    /// units: N/A
    pub ground_albedo: Vec3,

    /// A handle to a [`ScatteringMedium`], which describes the substance
    /// of the atmosphere and how it scatters light.
    pub medium: Handle<ScatteringMedium>,
}

impl Atmosphere {
    pub fn earthlike(medium: Handle<ScatteringMedium>) -> Self {
        const EARTH_BOTTOM_RADIUS: f32 = 6_360_000.0;
        const EARTH_TOP_RADIUS: f32 = 6_460_000.0;
        const EARTH_ALBEDO: Vec3 = Vec3::splat(0.3);
        Self {
            bottom_radius: EARTH_BOTTOM_RADIUS,
            top_radius: EARTH_TOP_RADIUS,
            ground_albedo: EARTH_ALBEDO,
            medium,
        }
    }
}

impl ExtractComponent for Atmosphere {
    type QueryData = Read<Atmosphere>;

    type QueryFilter = With<Camera3d>;

    type Out = ExtractedAtmosphere;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(ExtractedAtmosphere {
            bottom_radius: item.bottom_radius,
            top_radius: item.top_radius,
            ground_albedo: item.ground_albedo,
            medium: item.medium.id(),
        })
    }
}

/// The render-world representation of an `Atmosphere`, but which
/// hasn't been converted into shader uniforms yet.
#[derive(Clone, Component)]
pub struct ExtractedAtmosphere {
    pub bottom_radius: f32,
    pub top_radius: f32,
    pub ground_albedo: Vec3,
    pub medium: AssetId<ScatteringMedium>,
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

    /// A conversion factor between scene units and meters, used to
    /// ensure correctness at different length scales.
    pub scene_units_to_m: f32,

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
            scene_units_to_m: 1.0,
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
    pub scene_units_to_m: f32,
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
            scene_units_to_m: s.scene_units_to_m,
            sky_max_samples: s.sky_max_samples,
            rendering_method: s.rendering_method as u32,
        }
    }
}

impl ExtractComponent for GpuAtmosphereSettings {
    type QueryData = Read<AtmosphereSettings>;

    type QueryFilter = (With<Camera3d>, With<Atmosphere>);

    type Out = GpuAtmosphereSettings;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone().into())
    }
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
