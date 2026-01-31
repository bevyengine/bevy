use bevy_asset::Handle;
use bevy_camera::Camera;
use bevy_ecs::{
    component::Component,
    query::{QueryItem, With},
    reflect::ReflectComponent,
    resource::Resource,
    system::lifetimeless::Read,
};
use bevy_image::Image;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{extract_component::ExtractComponent, render_resource::ShaderType};

/// The placeholder data for the default film grain texture.
///
/// Not used for the actual effect, but to signal the shader if a texture was provided.
pub(super) static DEFAULT_FILM_GRAIN_TEXTURE_DATA: [u8; 4] = [255, 255, 255, 255];

/// The default film grain intensity amount.
const DEFAULT_FILM_GRAIN_INTENSITY: f32 = 0.05;
/// The default film grain shadows intensity amount.
const DEFAULT_FILM_GRAIN_SHADOWS_INTENSITY: f32 = 1.0;
/// The default film grain midtones intensity amount.
const DEFAULT_FILM_GRAIN_MIDTONES_INTENSITY: f32 = 0.5;
/// The default film grain highlight intensity amount.
const DEFAULT_FILM_GRAIN_HIGHLIGHTS_INTENSITY: f32 = 0.1;
/// The default film grain shadows threshold amount.
const DEFAULT_FILM_GRAIN_SHADOWS_THRESHOLD: f32 = 0.25;
/// The default film grain highlights threshold amount.
const DEFAULT_FILM_GRAIN_HIGHLIGHT_THRESHOLD: f32 = 0.75;
/// The default film grain grain size amount.
const DEFAULT_FILM_GRAIN_GRAIN_SIZE: f32 = 1.0;

#[derive(Resource)]
pub(crate) struct DefaultFilmGrainTexture(pub(super) Handle<Image>);

/// Add a film grain overlay to the rendered image.
///
/// [Film grain] simulates the random optical texture of photographic film
/// caused by the presence of small silver particles. It adds a gritty, noisy
/// look to the image, which is especially visible in flat color areas. Film
/// grain is commonly used to increase perceived realism and aesthetic style,
/// especially in retro or cinematic games.
///
/// Bevy’s implementation provides two methods for generating grain:
///
/// 1. **Texture-based grain (primary method)**:
/// Samples from a pre-computed noise texture that is tiled across the screen in repeat mode.
/// The texture is offset each frame using a hash function to prevent static patterns.
/// This is the default and recommended approach for performance.
///
/// 2. **Procedural grid noise (fallback)**:
/// Used if no grain texture is available or the texture size is invalid(1x1). It works by:
///     - Creating a virtual grid where each cell represents a grain chunk.
///     - Sampling pseudo-random RGB noise at grid cell corners using a hash function.
///     - Applying bilinear interpolation with smoothstep for smooth transitions.
///     - Animating the pattern by offsetting grid coordinates with the frame count.
#[derive(Reflect, Component, Clone)]
#[reflect(Component, Default, Clone)]
pub struct FilmGrain {
    /// The overall intensity of the film grain effect.
    ///
    /// The recommended range is 0.0 to 0.20.
    ///
    /// Range: `0.0` to `1.0`
    /// The default value is 0.05.
    pub intensity: f32,

    /// The intensity of the film grain in shadow areas.
    ///
    /// Range: `0.0` to `1.0`.
    /// The default value is 1.0.
    pub shadows_intensity: f32,

    /// The intensity of the film grain in midtone areas.
    ///
    /// Range: `0.0` to `1.0`.
    /// The default value is 0.5.
    pub midtones_intensity: f32,

    /// The intensity of the film grain in highlight areas.
    ///
    /// Range: `0.0` to `1.0`.
    /// The default value is 0.1.
    pub highlights_intensity: f32,

    /// The threshold separating shadows from midtones.
    ///
    /// Pixels below this value are considered shadows. This value should be
    /// lower than `highlights_threshold`.
    ///
    /// Range: `0.0` to `1.0`.
    /// The default value is 0.25.
    pub shadows_threshold: f32,

    /// The threshold separating highlights from midtones.
    ///
    /// Pixels above this value are considered highlights. This value should be
    /// higher than `shadows_threshold`.
    ///
    /// Range: `0.0` to `1.0`
    /// The default value is 0.75
    pub highlights_threshold: f32,

    /// The size of the film grain particles.
    ///
    /// The default value is 1.0
    pub grain_size: f32,

    /// A user-provided texture to use for the film grain.
    ///
    /// By default (if None), a default 1x1 placeholder texture is used.
    /// This signals the shader to generate film grain procedurally instead of sampling from a texture.
    ///
    /// Note: User should not pass a 1x1 texture manually,
    /// as it will be treated as invalid and trigger the same procedural fallback.
    pub texture: Option<Handle<Image>>,
}

impl Default for FilmGrain {
    fn default() -> Self {
        Self {
            intensity: DEFAULT_FILM_GRAIN_INTENSITY,
            shadows_intensity: DEFAULT_FILM_GRAIN_SHADOWS_INTENSITY,
            midtones_intensity: DEFAULT_FILM_GRAIN_MIDTONES_INTENSITY,
            highlights_intensity: DEFAULT_FILM_GRAIN_HIGHLIGHTS_INTENSITY,
            shadows_threshold: DEFAULT_FILM_GRAIN_SHADOWS_THRESHOLD,
            highlights_threshold: DEFAULT_FILM_GRAIN_HIGHLIGHT_THRESHOLD,
            grain_size: DEFAULT_FILM_GRAIN_GRAIN_SIZE,
            texture: None,
        }
    }
}

impl ExtractComponent for FilmGrain {
    type QueryData = Read<FilmGrain>;

    type QueryFilter = With<Camera>;

    type Out = FilmGrain;

    fn extract_component(film_grain: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        if film_grain.intensity > 0.0 {
            Some(film_grain.clone())
        } else {
            None
        }
    }
}

#[derive(ShaderType, Default)]
pub struct FilmGrainUniform {
    /// The overall intensity of the film grain effect.
    pub(super) intensity: f32,
    /// The intensity of the film grain in shadow areas.
    pub(super) shadows_intensity: f32,
    /// The intensity of the film grain in midtone areas.
    pub(super) midtones_intensity: f32,
    /// The intensity of the film grain in highlight areas.
    pub(super) highlights_intensity: f32,
    /// The threshold separating shadows from midtones.
    pub(super) shadows_threshold: f32,
    /// The threshold separating highlights from midtones.
    pub(super) highlights_threshold: f32,
    /// The size of the film grain particles.
    pub(super) grain_size: f32,
    /// The current frame number, used to animate the noise pattern over time.
    pub(super) frame: u32,
}
