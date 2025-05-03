//! Volumetric fog and volumetric lighting, also known as light shafts or god
//! rays.
//!
//! This module implements a more physically-accurate, but slower, form of fog
//! than the [`crate::fog`] module does. Notably, this *volumetric fog* allows
//! for light beams from directional lights to shine through, creating what is
//! known as *light shafts* or *god rays*.
//!
//! To add volumetric fog to a scene, add [`VolumetricFog`] to the
//! camera, and add [`VolumetricLight`] to directional lights that you wish to
//! be volumetric. [`VolumetricFog`] feature numerous settings that
//! allow you to define the accuracy of the simulation, as well as the look of
//! the fog. Currently, only interaction with directional lights that have
//! shadow maps is supported. Note that the overhead of the effect scales
//! directly with the number of directional lights in use, so apply
//! [`VolumetricLight`] sparingly for the best results.
//!
//! The overall algorithm, which is implemented as a postprocessing effect, is a
//! combination of the techniques described in [Scratchapixel] and [this blog
//! post]. It uses raymarching in screen space, transformed into shadow map
//! space for sampling and combined with physically-based modeling of absorption
//! and scattering. Bevy employs the widely-used [Henyey-Greenstein phase
//! function] to model asymmetry; this essentially allows light shafts to fade
//! into and out of existence as the user views them.
//!
//! [Scratchapixel]: https://www.scratchapixel.com/lessons/3d-basic-rendering/volume-rendering-for-developers/intro-volume-rendering.html
//!
//! [this blog post]: https://www.alexandre-pestana.com/volumetric-lights/
//!
//! [Henyey-Greenstein phase function]: https://www.pbr-book.org/4ed/Volume_Scattering/Phase_Functions#TheHenyeyndashGreensteinPhaseFunction

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle};
use bevy_color::Color;
use bevy_core_pipeline::core_3d::{
    graph::{Core3d, Node3d},
    prepare_core_3d_depth_textures,
};
use bevy_ecs::{
    component::Component, reflect::ReflectComponent, schedule::IntoScheduleConfigs as _,
};
use bevy_image::Image;
use bevy_math::{
    primitives::{Cuboid, Plane3d},
    Vec2, Vec3,
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    mesh::{Mesh, Meshable},
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{Shader, SpecializedRenderPipelines},
    sync_component::SyncComponentPlugin,
    view::Visibility,
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::components::Transform;
use render::{
    VolumetricFogNode, VolumetricFogPipeline, VolumetricFogUniformBuffer, CUBE_MESH, PLANE_MESH,
    VOLUMETRIC_FOG_HANDLE,
};

use crate::graph::NodePbr;

pub mod render;

/// A plugin that implements volumetric fog.
pub struct VolumetricFogPlugin;

/// Add this component to a [`DirectionalLight`](crate::DirectionalLight) with a shadow map
/// (`shadows_enabled: true`) to make volumetric fog interact with it.
///
/// This allows the light to generate light shafts/god rays.
#[derive(Clone, Copy, Component, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct VolumetricLight;

/// When placed on a [`bevy_core_pipeline::core_3d::Camera3d`], enables
/// volumetric fog and volumetric lighting, also known as light shafts or god
/// rays.
#[derive(Clone, Copy, Component, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct VolumetricFog {
    /// Color of the ambient light.
    ///
    /// This is separate from Bevy's [`AmbientLight`](crate::light::AmbientLight) because an
    /// [`EnvironmentMapLight`](crate::environment_map::EnvironmentMapLight) is
    /// still considered an ambient light for the purposes of volumetric fog. If you're using a
    /// [`EnvironmentMapLight`](crate::environment_map::EnvironmentMapLight), for best results,
    /// this should be a good approximation of the average color of the environment map.
    ///
    /// Defaults to white.
    pub ambient_color: Color,

    /// The brightness of the ambient light.
    ///
    /// If there's no [`EnvironmentMapLight`](crate::environment_map::EnvironmentMapLight),
    /// set this to 0.
    ///
    /// Defaults to 0.1.
    pub ambient_intensity: f32,

    /// The maximum distance to offset the ray origin randomly by, in meters.
    ///
    /// This is intended for use with temporal antialiasing. It helps fog look
    /// less blocky by varying the start position of the ray, using interleaved
    /// gradient noise.
    pub jitter: f32,

    /// The number of raymarching steps to perform.
    ///
    /// Higher values produce higher-quality results with less banding, but
    /// reduce performance.
    ///
    /// The default value is 64.
    pub step_count: u32,
}

#[derive(Clone, Component, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(Transform, Visibility)]
pub struct FogVolume {
    /// The color of the fog.
    ///
    /// Note that the fog must be lit by a [`VolumetricLight`] or ambient light
    /// in order for this color to appear.
    ///
    /// Defaults to white.
    pub fog_color: Color,

    /// The density of fog, which measures how dark the fog is.
    ///
    /// The default value is 0.1.
    pub density_factor: f32,

    /// Optional 3D voxel density texture for the fog.
    pub density_texture: Option<Handle<Image>>,

    /// Configurable offset of the density texture in UVW coordinates.
    ///
    /// This can be used to scroll a repeating density texture in a direction over time
    /// to create effects like fog moving in the wind. Make sure to configure the texture
    /// to use `ImageAddressMode::Repeat` if this is your intention.
    ///
    /// Has no effect when no density texture is present.
    ///
    /// The default value is (0, 0, 0).
    pub density_texture_offset: Vec3,

    /// The absorption coefficient, which measures what fraction of light is
    /// absorbed by the fog at each step.
    ///
    /// Increasing this value makes the fog darker.
    ///
    /// The default value is 0.3.
    pub absorption: f32,

    /// The scattering coefficient, which measures the fraction of light that's
    /// scattered toward, and away from, the viewer.
    ///
    /// The default value is 0.3.
    pub scattering: f32,

    /// Measures the fraction of light that's scattered *toward* the camera, as
    /// opposed to *away* from the camera.
    ///
    /// Increasing this value makes light shafts become more prominent when the
    /// camera is facing toward their source and less prominent when the camera
    /// is facing away. Essentially, a high value here means the light shafts
    /// will fade into view as the camera focuses on them and fade away when the
    /// camera is pointing away.
    ///
    /// The default value is 0.8.
    pub scattering_asymmetry: f32,

    /// Applies a nonphysical color to the light.
    ///
    /// This can be useful for artistic purposes but is nonphysical.
    ///
    /// The default value is white.
    pub light_tint: Color,

    /// Scales the light by a fixed fraction.
    ///
    /// This can be useful for artistic purposes but is nonphysical.
    ///
    /// The default value is 1.0, which results in no adjustment.
    pub light_intensity: f32,
}

impl Plugin for VolumetricFogPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            VOLUMETRIC_FOG_HANDLE,
            "volumetric_fog.wgsl",
            Shader::from_wgsl
        );

        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        meshes.insert(&PLANE_MESH, Plane3d::new(Vec3::Z, Vec2::ONE).mesh().into());
        meshes.insert(&CUBE_MESH, Cuboid::new(1.0, 1.0, 1.0).mesh().into());

        app.register_type::<VolumetricFog>()
            .register_type::<VolumetricLight>();

        app.add_plugins(SyncComponentPlugin::<FogVolume>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<VolumetricFogPipeline>>()
            .init_resource::<VolumetricFogUniformBuffer>()
            .add_systems(ExtractSchedule, render::extract_volumetric_fog)
            .add_systems(
                Render,
                (
                    render::prepare_volumetric_fog_pipelines.in_set(RenderSet::Prepare),
                    render::prepare_volumetric_fog_uniforms.in_set(RenderSet::Prepare),
                    render::prepare_view_depth_textures_for_volumetric_fog
                        .in_set(RenderSet::Prepare)
                        .before(prepare_core_3d_depth_textures),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<VolumetricFogPipeline>()
            .add_render_graph_node::<ViewNodeRunner<VolumetricFogNode>>(
                Core3d,
                NodePbr::VolumetricFog,
            )
            .add_render_graph_edges(
                Core3d,
                // Volumetric fog is a postprocessing effect. Run it after the
                // main pass but before bloom.
                (Node3d::EndMainPass, NodePbr::VolumetricFog, Node3d::Bloom),
            );
    }
}

impl Default for VolumetricFog {
    fn default() -> Self {
        Self {
            step_count: 64,
            // Matches `AmbientLight` defaults.
            ambient_color: Color::WHITE,
            ambient_intensity: 0.1,
            jitter: 0.0,
        }
    }
}

impl Default for FogVolume {
    fn default() -> Self {
        Self {
            absorption: 0.3,
            scattering: 0.3,
            density_factor: 0.1,
            density_texture: None,
            density_texture_offset: Vec3::ZERO,
            scattering_asymmetry: 0.5,
            fog_color: Color::WHITE,
            light_tint: Color::WHITE,
            light_intensity: 1.0,
        }
    }
}
