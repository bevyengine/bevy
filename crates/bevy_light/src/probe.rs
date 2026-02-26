use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_camera::visibility::{self, ViewVisibility, Visibility, VisibilityClass};
use bevy_color::{Color, ColorToComponents, Srgba};
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_math::{Quat, UVec2, Vec3};
use bevy_reflect::prelude::*;
use bevy_transform::components::Transform;
use wgpu_types::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

use crate::cluster::ClusterVisibilityClass;

/// A marker component for a light probe, which is a cuboid region that provides
/// global illumination to all fragments inside it.
///
/// Note that a light probe will have no effect unless the entity contains some
/// kind of illumination, which can either be an [`EnvironmentMapLight`] or an
/// [`IrradianceVolume`].
///
/// The light probe range is conceptually a unit cube (1×1×1) centered on the
/// origin. The [`Transform`] applied to this entity can scale, rotate, or
/// translate that cube so that it contains all fragments that should take this
/// light probe into account.
///
/// Light probes may specify a *falloff* range over which their influence tapers
/// off. The falloff range is expressed as a range from 0, representing
/// infinitely-sharp falloff, to 1, representing the most gradual falloff,
/// *inside* the 1×1×1 cube. So, for example, if you set the falloff to 0.5 on
/// an axis, then any fragments with positions between 0.0 units to 0.25 units
/// on that axis will receive 100% influence from the light probe, while
/// fragments with positions between 0.25 units to 0.5 units on that axis will
/// receive gradually-diminished influence, and fragments more than 0.5 units
/// from the center of the light probe will receive no influence at all.
///
/// When multiple sources of indirect illumination can be applied to a fragment,
/// the highest-quality ones are chosen. Diffuse and specular illumination are
/// considered separately, so, for example, Bevy may decide to sample the
/// diffuse illumination from an irradiance volume and the specular illumination
/// from a reflection probe. From highest priority to lowest priority, the
/// ranking is as follows:
///
/// | Rank | Diffuse              | Specular             |
/// | ---- | -------------------- | -------------------- |
/// | 1    | Lightmap             | Lightmap             |
/// | 2    | Irradiance volume    | Reflection probe     |
/// | 3    | Reflection probe     | View environment map |
/// | 4    | View environment map |                      |
///
/// Note that ambient light is always added to the diffuse component and does
/// not participate in the ranking. That is, ambient light is applied in
/// addition to, not instead of, the light sources above.
///
/// Multiple light probes of the same type can apply to a single fragment. By
/// setting falloff regions appropriately, one can achieve a gradual blend from
/// one reflection probe and/or irradiance volume to another as objects move
/// between them.
///
/// A terminology note: Unfortunately, there is little agreement across game and
/// graphics engines as to what to call the various techniques that Bevy groups
/// under the term *light probe*. In Bevy, a *light probe* is the generic term
/// that encompasses both *reflection probes* and *irradiance volumes*. In
/// object-oriented terms, *light probe* is the superclass, and *reflection
/// probe* and *irradiance volume* are subclasses. In other engines, you may see
/// the term *light probe* refer to an irradiance volume with a single voxel, or
/// perhaps some other technique, while in Bevy *light probe* refers not to a
/// specific technique but rather to a class of techniques. Developers familiar
/// with other engines should be aware of this terminology difference.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(Transform, ViewVisibility, Visibility, VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<ClusterVisibilityClass>)]
pub struct LightProbe {
    /// The distance over which the effect of the light probe becomes weaker, on
    /// each axis.
    ///
    /// This is specified as a ratio of the total distance on each axis. So, for
    /// example, if you specify `Vec3::splat(0.25)` here, then the light probe
    /// will consist of a 0.75×0.75×0.75 unit cube within which fragments
    /// receive the maximum influence from the light probe, contained within a
    /// 1×1×1 cube which influences fragments inside it in a manner that
    /// diminishes as fragments get farther from its center.
    ///
    /// Falloff doesn't affect the influence range of the light probe itself;
    /// it's still conceptually a 1×1×1 cube, regardless of the falloff setting.
    /// In other words, falloff modifies the *interior* of the light probe cube
    /// instead of increasing the *exterior* boundaries of the cube.
    pub falloff: Vec3,
}

impl LightProbe {
    /// Creates a new light probe component.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

/// A pair of cubemap textures that represent the surroundings of a specific
/// area in space.
///
/// See `bevy_pbr::environment_map` for detailed information.
#[derive(Clone, Component, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct EnvironmentMapLight {
    /// The blurry image that represents diffuse radiance surrounding a region.
    pub diffuse_map: Handle<Image>,

    /// The typically-sharper, mipmapped image that represents specular radiance
    /// surrounding a region.
    pub specular_map: Handle<Image>,

    /// Scale factor applied to the diffuse and specular light generated by this component.
    ///
    /// After applying this multiplier, the resulting values should
    /// be in units of [cd/m^2](https://en.wikipedia.org/wiki/Candela_per_square_metre).
    ///
    /// See also <https://google.github.io/filament/Filament.html#lighting/imagebasedlights/iblunit>.
    pub intensity: f32,

    /// World space rotation applied to the environment light cubemaps.
    /// This is useful for users who require a different axis, such as the Z-axis, to serve
    /// as the vertical axis.
    pub rotation: Quat,

    /// Whether the light from this environment map contributes diffuse lighting
    /// to meshes with lightmaps.
    ///
    /// Set this to false if your lightmap baking tool bakes the diffuse light
    /// from this environment light into the lightmaps in order to avoid
    /// counting the radiance from this environment map twice.
    ///
    /// By default, this is set to true.
    pub affects_lightmapped_mesh_diffuse: bool,
}

impl EnvironmentMapLight {
    /// An environment map with a uniform color, useful for uniform ambient lighting.
    pub fn solid_color(assets: &mut Assets<Image>, color: impl Into<Color>) -> Self {
        let color = color.into();
        Self::hemispherical_gradient(assets, color, color, color)
    }

    /// An environment map with a hemispherical gradient, fading between the sky and ground colors
    /// at the horizon. Useful as a very simple 'sky'.
    pub fn hemispherical_gradient(
        assets: &mut Assets<Image>,
        top_color: impl Into<Color>,
        mid_color: impl Into<Color>,
        bottom_color: impl Into<Color>,
    ) -> Self {
        let handle = assets.add(Self::hemispherical_gradient_cubemap(
            top_color.into(),
            mid_color.into(),
            bottom_color.into(),
        ));

        Self {
            diffuse_map: handle.clone(),
            specular_map: handle,
            ..Default::default()
        }
    }

    pub(crate) fn hemispherical_gradient_cubemap(
        top_color: Color,
        mid_color: Color,
        bottom_color: Color,
    ) -> Image {
        let top_color: Srgba = top_color.into();
        let mid_color: Srgba = mid_color.into();
        let bottom_color: Srgba = bottom_color.into();
        Image {
            texture_view_descriptor: Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..Default::default()
            }),
            ..Image::new(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 6,
                },
                TextureDimension::D2,
                [
                    mid_color,
                    mid_color,
                    top_color,
                    bottom_color,
                    mid_color,
                    mid_color,
                ]
                .into_iter()
                .flat_map(|c| c.to_f32_array().map(half::f16::from_f32))
                .flat_map(half::f16::to_le_bytes)
                .collect(),
                TextureFormat::Rgba16Float,
                RenderAssetUsages::RENDER_WORLD,
            )
        }
    }
}

impl Default for EnvironmentMapLight {
    fn default() -> Self {
        EnvironmentMapLight {
            diffuse_map: Handle::default(),
            specular_map: Handle::default(),
            intensity: 0.0,
            rotation: Quat::IDENTITY,
            affects_lightmapped_mesh_diffuse: true,
        }
    }
}

/// Adds a skybox to a 3D camera, based on a cubemap texture.
///
/// Note that this component does not (currently) affect the scene's lighting.
/// To do so, use [`EnvironmentMapLight`] alongside this component.
///
/// See also <https://en.wikipedia.org/wiki/Skybox_(video_games)>.
#[derive(Component, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct Skybox {
    /// The cubemap to use.
    pub image: Handle<Image>,
    /// Scale factor applied to the skybox image.
    /// After applying this multiplier to the image samples, the resulting values should
    /// be in units of [cd/m^2](https://en.wikipedia.org/wiki/Candela_per_square_metre).
    pub brightness: f32,

    /// View space rotation applied to the skybox cubemap.
    /// This is useful for users who require a different axis, such as the Z-axis, to serve
    /// as the vertical axis.
    pub rotation: Quat,
}

impl Default for Skybox {
    fn default() -> Self {
        Skybox {
            image: Handle::default(),
            brightness: 0.0,
            rotation: Quat::IDENTITY,
        }
    }
}

/// A generated environment map that is filtered at runtime.
///
/// See `bevy_pbr::light_probe::generate` for detailed information.
#[derive(Clone, Component, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct GeneratedEnvironmentMapLight {
    /// Source cubemap to be filtered on the GPU, size must be a power of two.
    pub environment_map: Handle<Image>,

    /// Scale factor applied to the diffuse and specular light generated by this
    /// component. Expressed in cd/m² (candela per square meter).
    pub intensity: f32,

    /// World-space rotation applied to the cubemap.
    pub rotation: Quat,

    /// Whether this light contributes diffuse lighting to meshes that already
    /// have baked lightmaps.
    pub affects_lightmapped_mesh_diffuse: bool,
}

impl Default for GeneratedEnvironmentMapLight {
    fn default() -> Self {
        GeneratedEnvironmentMapLight {
            environment_map: Handle::default(),
            intensity: 0.0,
            rotation: Quat::IDENTITY,
            affects_lightmapped_mesh_diffuse: true,
        }
    }
}

/// Lets the atmosphere contribute environment lighting (reflections and ambient diffuse) to your scene.
///
/// Attach this to a [`Camera3d`](bevy_camera::Camera3d) to light the entire view, or to a
/// [`LightProbe`] to light only a specific region.
/// Behind the scenes, this generates an environment map from the atmosphere for image-based lighting
/// and inserts a corresponding [`GeneratedEnvironmentMapLight`].
///
/// For HDRI-based lighting, use a preauthored [`EnvironmentMapLight`] or filter one at runtime with
/// [`GeneratedEnvironmentMapLight`].
#[derive(Component, Clone)]
pub struct AtmosphereEnvironmentMapLight {
    /// Controls how bright the atmosphere's environment lighting is.
    /// Increase this value to brighten reflections and ambient diffuse lighting.
    ///
    /// The default is `1.0` so that the generated environment lighting matches
    /// the light intensity of the atmosphere in the scene.
    pub intensity: f32,
    /// Whether the diffuse contribution should affect meshes that already have lightmaps.
    pub affects_lightmapped_mesh_diffuse: bool,
    /// Cubemap resolution in pixels (must be a power-of-two).
    pub size: UVec2,
}

impl Default for AtmosphereEnvironmentMapLight {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            affects_lightmapped_mesh_diffuse: true,
            size: UVec2::new(512, 512),
        }
    }
}

/// The component that defines an irradiance volume.
///
/// See `bevy_pbr::irradiance_volume` for detailed information.
///
/// This component requires the [`LightProbe`] component, and is typically used with
/// [`bevy_transform::components::Transform`] to place the volume appropriately.
#[derive(Clone, Reflect, Component, Debug)]
#[reflect(Component, Default, Debug, Clone)]
#[require(LightProbe)]
pub struct IrradianceVolume {
    /// The 3D texture that represents the ambient cubes, encoded in the format
    /// described in `bevy_pbr::irradiance_volume`.
    pub voxels: Handle<Image>,

    /// Scale factor applied to the diffuse and specular light generated by this component.
    ///
    /// After applying this multiplier, the resulting values should
    /// be in units of [cd/m^2](https://en.wikipedia.org/wiki/Candela_per_square_metre).
    ///
    /// See also <https://google.github.io/filament/Filament.html#lighting/imagebasedlights/iblunit>.
    pub intensity: f32,

    /// Whether the light from this irradiance volume has an effect on meshes
    /// with lightmaps.
    ///
    /// Set this to false if your lightmap baking tool bakes the light from this
    /// irradiance volume into the lightmaps in order to avoid counting the
    /// irradiance twice. Frequently, applications use irradiance volumes as a
    /// lower-quality alternative to lightmaps for capturing indirect
    /// illumination on dynamic objects, and such applications will want to set
    /// this value to false.
    ///
    /// By default, this is set to true.
    pub affects_lightmapped_meshes: bool,
}

impl Default for IrradianceVolume {
    #[inline]
    fn default() -> Self {
        IrradianceVolume {
            voxels: Handle::default(),
            intensity: 0.0,
            affects_lightmapped_meshes: true,
        }
    }
}

/// Add this component to a reflection probe to customize *parallax correction*.
///
/// For environment maps added directly to a camera, Bevy renders the reflected
/// scene that a cubemap captures as though it were infinitely far away. This is
/// acceptable if the cubemap captures very distant objects, such as distant
/// mountains in outdoor scenes. It's less ideal, however, if the cubemap
/// reflects near objects, such as the interior of a room. Therefore, by default
/// for reflection probes Bevy uses *parallax-corrected cubemaps* (PCCM), which
/// causes Bevy to treat the reflected scene as though it coincided with the
/// boundaries of the light probe.
///
/// As an example, for indoor scenes, it's common to place reflection probes
/// inside each room and to make the boundaries of the reflection probe (as
/// determined by the light probe's [`bevy_transform::components::Transform`])
/// coincide with the walls of the room. That way, the reflection probes will
/// (1) apply to the objects inside the room and (2) take the positions of those
/// objects into account in order to create a realistic reflection.
///
/// Instead of having the simulated boundaries of the reflected area coincide
/// with the boundaries of the light probe, it's also possible to specify
/// *custom* parallax correction boundaries, so that the region of influence of
/// the light probe doesn't correspond with the simulated boundaries used for
/// parallax correction. This is commonly used when the boundaries of the light
/// probe are slightly larger than the room that the light probe contains, for
/// instance in order to avoid artifacts along the edges of the room that occur
/// due to rounding error, or else when the *falloff* feature is used that
/// blends reflection probes into adjacent ones.
///
/// Place this component on an entity that has a [`LightProbe`] and
/// [`EnvironmentMapLight`] component in order to either (1) opt out of parallax
/// correction via [`ParallaxCorrection::None`] or (2) specify custom parallax
/// correction boundaries via [`ParallaxCorrection::Custom`]. If you don't
/// manually place this component on a reflection probe, Bevy will automatically
/// add a [`ParallaxCorrection::Auto`] component so that the boundaries of the
/// light probe will coincide with the simulated boundaries used for parallax
/// correction.
///
/// See the `pccm` example for an example of usage of parallax-corrected
/// cubemaps and the `light_probe_blending` example for an example of use of
/// custom parallax correction boundaries.
#[derive(Clone, Copy, Default, Component, Reflect)]
#[reflect(Clone, Default, Component)]
pub enum ParallaxCorrection {
    /// No parallax correction is used.
    ///
    /// This component causes Bevy to render the reflection as though the
    /// reflected surface were infinitely distant.
    None,

    /// The parallax correction boundaries correspond with the boundaries of the
    /// light probe.
    ///
    /// This is the default value. Bevy automatically adds this component value
    /// to reflection probes that don't have a [`ParallaxCorrection`] component.
    /// It's equivalent to `ParallaxCorrection::Custom(Vec3::splat(0.5))`.
    #[default]
    Auto,

    /// The parallax correction boundaries are specified manually.
    ///
    /// The simulated reflection boundaries are specified as an axis-aligned
    /// cube *in light probe space* with the given *half* extents. Thus, for
    /// example, if you set the parallax correction boundaries to `vec3(0.5,
    /// 1.0, 2.0)` and the scale of the light probe is `vec3(3.0, 3.0, 3.0)`,
    /// then the simulated boundaries of the reflected area used for parallax
    /// correction will be centered on the reflection probe with a width of 3.0
    /// m, a height of 6.0 m, and a depth of 12.0 m.
    Custom(Vec3),
}

/// A system that automatically adds a [`ParallaxCorrection::Auto`] component to
/// any reflection probe that doesn't already have a [`ParallaxCorrection`]
/// component.
///
/// A reflection probe is any entity with both an [`EnvironmentMapLight`] and a
/// [`LightProbe`] component.
pub fn automatically_add_parallax_correction_components(
    mut commands: Commands,
    query: Query<
        Entity,
        (
            With<EnvironmentMapLight>,
            With<LightProbe>,
            Without<ParallaxCorrection>,
        ),
    >,
) {
    for entity in &query {
        commands
            .entity(entity)
            .insert(ParallaxCorrection::default());
    }
}
