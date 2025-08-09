use bevy_asset::Handle;
use bevy_camera::{
    primitives::{CascadesFrusta, Frustum},
    visibility::{self, CascadesVisibleEntities, ViewVisibility, Visibility, VisibilityClass},
    Camera,
};
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_reflect::prelude::*;
use bevy_transform::components::Transform;
use tracing::warn;

use super::{
    cascade::CascadeShadowConfig, cluster::ClusterVisibilityClass, light_consts, Cascades,
};

/// A Directional light.
///
/// Directional lights don't exist in reality but they are a good
/// approximation for light sources VERY far away, like the sun or
/// the moon.
///
/// The light shines along the forward direction of the entity's transform. With a default transform
/// this would be along the negative-Z axis.
///
/// Valid values for `illuminance` are:
///
/// | Illuminance (lux) | Surfaces illuminated by                        |
/// |-------------------|------------------------------------------------|
/// | 0.0001            | Moonless, overcast night sky (starlight)       |
/// | 0.002             | Moonless clear night sky with airglow          |
/// | 0.05–0.3          | Full moon on a clear night                     |
/// | 3.4               | Dark limit of civil twilight under a clear sky |
/// | 20–50             | Public areas with dark surroundings            |
/// | 50                | Family living room lights                      |
/// | 80                | Office building hallway/toilet lighting        |
/// | 100               | Very dark overcast day                         |
/// | 150               | Train station platforms                        |
/// | 320–500           | Office lighting                                |
/// | 400               | Sunrise or sunset on a clear day.              |
/// | 1000              | Overcast day; typical TV studio lighting       |
/// | 10,000–25,000     | Full daylight (not direct sun)                 |
/// | 32,000–100,000    | Direct sunlight                                |
///
/// Source: [Wikipedia](https://en.wikipedia.org/wiki/Lux)
///
/// ## Shadows
///
/// To enable shadows, set the `shadows_enabled` property to `true`.
///
/// Shadows are produced via [cascaded shadow maps](https://developer.download.nvidia.com/SDK/10.5/opengl/src/cascaded_shadow_maps/doc/cascaded_shadow_maps.pdf).
///
/// To modify the cascade setup, such as the number of cascades or the maximum shadow distance,
/// change the [`CascadeShadowConfig`] component of the entity with the [`DirectionalLight`].
///
/// To control the resolution of the shadow maps, use the [`DirectionalLightShadowMap`] resource.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(
    Cascades,
    CascadesFrusta,
    CascadeShadowConfig,
    CascadesVisibleEntities,
    Transform,
    Visibility,
    VisibilityClass
)]
#[component(on_add = visibility::add_visibility_class::<ClusterVisibilityClass>)]
pub struct DirectionalLight {
    /// The color of the light.
    ///
    /// By default, this is white.
    pub color: Color,

    /// Illuminance in lux (lumens per square meter), representing the amount of
    /// light projected onto surfaces by this light source. Lux is used here
    /// instead of lumens because a directional light illuminates all surfaces
    /// more-or-less the same way (depending on the angle of incidence). Lumens
    /// can only be specified for light sources which emit light from a specific
    /// area.
    pub illuminance: f32,

    /// Whether this light casts shadows.
    ///
    /// Note that shadows are rather expensive and become more so with every
    /// light that casts them. In general, it's best to aggressively limit the
    /// number of lights with shadows enabled to one or two at most.
    pub shadows_enabled: bool,

    /// Whether soft shadows are enabled, and if so, the size of the light.
    ///
    /// Soft shadows, also known as *percentage-closer soft shadows* or PCSS,
    /// cause shadows to become blurrier (i.e. their penumbra increases in
    /// radius) as they extend away from objects. The blurriness of the shadow
    /// depends on the size of the light; larger lights result in larger
    /// penumbras and therefore blurrier shadows.
    ///
    /// Currently, soft shadows are rather noisy if not using the temporal mode.
    /// If you enable soft shadows, consider choosing
    /// [`ShadowFilteringMethod::Temporal`] and enabling temporal antialiasing
    /// (TAA) to smooth the noise out over time.
    ///
    /// Note that soft shadows are significantly more expensive to render than
    /// hard shadows.
    ///
    /// [`ShadowFilteringMethod::Temporal`]: crate::ShadowFilteringMethod::Temporal
    #[cfg(feature = "experimental_pbr_pcss")]
    pub soft_shadow_size: Option<f32>,

    /// Whether this directional light contributes diffuse lighting to meshes
    /// with lightmaps.
    ///
    /// Set this to false if your lightmap baking tool bakes the direct diffuse
    /// light from this directional light into the lightmaps in order to avoid
    /// counting the radiance from this light twice. Note that the specular
    /// portion of the light is always considered, because Bevy currently has no
    /// means to bake specular light.
    ///
    /// By default, this is set to true.
    pub affects_lightmapped_mesh_diffuse: bool,

    /// A value that adjusts the tradeoff between self-shadowing artifacts and
    /// proximity of shadows to their casters.
    ///
    /// This value frequently must be tuned to the specific scene; this is
    /// normal and a well-known part of the shadow mapping workflow. If set too
    /// low, unsightly shadow patterns appear on objects not in shadow as
    /// objects incorrectly cast shadows on themselves, known as *shadow acne*.
    /// If set too high, shadows detach from the objects casting them and seem
    /// to "fly" off the objects, known as *Peter Panning*.
    pub shadow_depth_bias: f32,

    /// A bias applied along the direction of the fragment's surface normal. It
    /// is scaled to the shadow map's texel size so that it is automatically
    /// adjusted to the orthographic projection.
    pub shadow_normal_bias: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        DirectionalLight {
            color: Color::WHITE,
            illuminance: light_consts::lux::AMBIENT_DAYLIGHT,
            shadows_enabled: false,
            shadow_depth_bias: Self::DEFAULT_SHADOW_DEPTH_BIAS,
            shadow_normal_bias: Self::DEFAULT_SHADOW_NORMAL_BIAS,
            affects_lightmapped_mesh_diffuse: true,
            #[cfg(feature = "experimental_pbr_pcss")]
            soft_shadow_size: None,
        }
    }
}

impl DirectionalLight {
    pub const DEFAULT_SHADOW_DEPTH_BIAS: f32 = 0.02;
    pub const DEFAULT_SHADOW_NORMAL_BIAS: f32 = 1.8;
}

/// Add to a [`DirectionalLight`] to add a light texture effect.
/// A texture mask is applied to the light source to modulate its intensity,  
/// simulating patterns like window shadows, gobo/cookie effects, or soft falloffs.
#[derive(Clone, Component, Debug, Reflect)]
#[reflect(Component, Debug)]
#[require(DirectionalLight)]
pub struct DirectionalLightTexture {
    /// The texture image. Only the R channel is read.
    pub image: Handle<Image>,
    /// Whether to tile the image infinitely, or use only a single tile centered at the light's translation
    pub tiled: bool,
}

/// Controls the resolution of [`DirectionalLight`] and [`SpotLight`](crate::SpotLight) shadow maps.
///
/// ```
/// # use bevy_app::prelude::*;
/// # use bevy_light::DirectionalLightShadowMap;
/// App::new()
///     .insert_resource(DirectionalLightShadowMap { size: 4096 });
/// ```
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource, Debug, Default, Clone)]
pub struct DirectionalLightShadowMap {
    // The width and height of each cascade.
    ///
    /// Must be a power of two to avoid unstable cascade positioning.
    ///
    /// Defaults to `2048`.
    pub size: usize,
}

impl Default for DirectionalLightShadowMap {
    fn default() -> Self {
        Self { size: 2048 }
    }
}

pub fn validate_shadow_map_size(mut shadow_map: ResMut<DirectionalLightShadowMap>) {
    if shadow_map.is_changed() && !shadow_map.size.is_power_of_two() {
        let new_size = shadow_map.size.next_power_of_two();
        warn!("Non-power-of-two DirectionalLightShadowMap sizes are not supported, correcting {} to {new_size}", shadow_map.size);
        shadow_map.size = new_size;
    }
}

pub fn update_directional_light_frusta(
    mut views: Query<
        (
            &Cascades,
            &DirectionalLight,
            &ViewVisibility,
            &mut CascadesFrusta,
        ),
        (
            // Prevents this query from conflicting with camera queries.
            Without<Camera>,
        ),
    >,
) {
    for (cascades, directional_light, visibility, mut frusta) in &mut views {
        // The frustum is used for culling meshes to the light for shadow mapping
        // so if shadow mapping is disabled for this light, then the frustum is
        // not needed.
        if !directional_light.shadows_enabled || !visibility.get() {
            continue;
        }

        frusta.frusta = cascades
            .cascades
            .iter()
            .map(|(view, cascades)| {
                (
                    *view,
                    cascades
                        .iter()
                        .map(|c| Frustum::from_clip_from_world(&c.clip_from_world))
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
    }
}
