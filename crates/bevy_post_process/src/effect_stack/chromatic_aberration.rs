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

/// The raw RGBA data for the default chromatic aberration gradient.
///
/// This consists of one red pixel, one green pixel, and one blue pixel, in that
/// order.
pub(super) static DEFAULT_CHROMATIC_ABERRATION_LUT_DATA: [u8; 12] =
    [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];

/// The default chromatic aberration intensity amount, in a fraction of the
/// window size.
const DEFAULT_CHROMATIC_ABERRATION_INTENSITY: f32 = 0.02;

/// The default maximum number of samples for chromatic aberration.
const DEFAULT_CHROMATIC_ABERRATION_MAX_SAMPLES: u32 = 8;

#[derive(Resource)]
pub(crate) struct DefaultChromaticAberrationLut(pub(crate) Handle<Image>);

/// Adds colored fringes to the edges of objects in the scene.
///
/// [Chromatic aberration] simulates the effect when lenses fail to focus all
/// colors of light toward a single point. It causes rainbow-colored streaks to
/// appear, which are especially apparent on the edges of objects. Chromatic
/// aberration is commonly used for collision effects, especially in horror
/// games.
///
/// Bevy's implementation is based on that of *Inside* ([Gjøl & Svendsen 2016]).
/// It's based on a customizable lookup texture, which allows for changing the
/// color pattern. By default, the color pattern is simply a 3×1 pixel texture
/// consisting of red, green, and blue, in that order, but you can change it to
/// any image in order to achieve different effects.
///
/// [Chromatic aberration]: https://en.wikipedia.org/wiki/Chromatic_aberration
///
/// [Gjøl & Svendsen 2016]: https://github.com/playdeadgames/publications/blob/master/INSIDE/rendering_inside_gdc2016.pdf
#[derive(Reflect, Component, Clone)]
#[reflect(Component, Default, Clone)]
pub struct ChromaticAberration {
    /// The lookup texture that determines the color gradient.
    ///
    /// By default (if None), this is a 3×1 texel texture consisting of one red
    /// pixel, one green pixel, and one blue texel, in that order. This
    /// recreates the most typical chromatic aberration pattern. However, you
    /// can change it to achieve different artistic effects.
    ///
    /// The texture is always sampled in its vertical center, so it should
    /// ordinarily have a height of 1 texel.
    pub color_lut: Option<Handle<Image>>,

    /// The size of the streaks around the edges of objects, as a fraction of
    /// the window size.
    ///
    /// The default value is 0.02.
    pub intensity: f32,

    /// A cap on the number of texture samples that will be performed.
    ///
    /// Higher values result in smoother-looking streaks but are slower.
    ///
    /// The default value is 8.
    pub max_samples: u32,
}

impl Default for ChromaticAberration {
    fn default() -> Self {
        Self {
            color_lut: None,
            intensity: DEFAULT_CHROMATIC_ABERRATION_INTENSITY,
            max_samples: DEFAULT_CHROMATIC_ABERRATION_MAX_SAMPLES,
        }
    }
}

impl ExtractComponent for ChromaticAberration {
    type QueryData = Read<ChromaticAberration>;

    type QueryFilter = With<Camera>;

    type Out = ChromaticAberration;

    fn extract_component(
        chromatic_aberration: QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self::Out> {
        // Skip the postprocessing phase entirely if the intensity is zero.
        if chromatic_aberration.intensity > 0.0 {
            Some(chromatic_aberration.clone())
        } else {
            None
        }
    }
}

/// The on-GPU version of the [`ChromaticAberration`] settings.
///
/// See the documentation for [`ChromaticAberration`] for more information on
/// each of these fields.
#[derive(ShaderType, Default)]
pub struct ChromaticAberrationUniform {
    /// The intensity of the effect, in a fraction of the screen.
    pub(super) intensity: f32,
    /// A cap on the number of samples of the source texture that the shader
    /// will perform.
    pub(super) max_samples: u32,
    /// Padding data.
    pub(super) unused_1: u32,
    /// Padding data.
    pub(super) unused_2: u32,
}
