//! Allows configuring a material's transparency behavior.

use bevy_reflect::{std_traits::ReflectDefault, Reflect};

// TODO: add discussion about performance.
/// Sets how a material's base color alpha channel is used for transparency.
#[derive(Debug, Default, Reflect, Copy, Clone, PartialEq)]
#[reflect(Default, Debug, Clone)]
pub enum AlphaMode {
    /// Base color alpha values are overridden to be fully opaque (1.0).
    #[default]
    Opaque,
    /// Reduce transparency to fully opaque or fully transparent
    /// based on a threshold.
    ///
    /// Compares the base color alpha value to the specified threshold.
    /// If the value is below the threshold,
    /// considers the color to be fully transparent (alpha is set to 0.0).
    /// If it is equal to or above the threshold,
    /// considers the color to be fully opaque (alpha is set to 1.0).
    Mask(f32),
    /// The base color alpha value defines the opacity of the color.
    /// Standard alpha-blending is used to blend the fragment's color
    /// with the color behind it.
    Blend,
    /// Similar to [`AlphaMode::Blend`], however assumes RGB channel values are
    /// [premultiplied](https://en.wikipedia.org/wiki/Alpha_compositing#Straight_versus_premultiplied).
    ///
    /// For otherwise constant RGB values, behaves more like [`AlphaMode::Blend`] for
    /// alpha values closer to 1.0, and more like [`AlphaMode::Add`] for
    /// alpha values closer to 0.0.
    ///
    /// Can be used to avoid “border” or “outline” artifacts that can occur
    /// when using plain alpha-blended textures.
    Premultiplied,
    /// Uses [alpha to coverage] to render anti-aliased cutouts: an
    /// anti-aliased version of [`AlphaMode::Mask`] with a threshold of 0.5.
    ///
    /// The fragment is spread out over a hardware-dependent number of sample
    /// locations proportional to the alpha value. This requires multisample
    /// antialiasing; if MSAA isn't on, this is identical to
    /// [`AlphaMode::Mask`] with a value of 0.5.
    ///
    /// This mode is designed for textures whose alpha channel describes a
    /// shape, like leaves, grass or fences. When the material has a base color
    /// texture, [`StandardMaterial`] "sharpens" the alpha value around 0.5 so
    /// that the transition from fully transparent to fully opaque spans roughly
    /// one pixel, producing crisp edges that are smoothed by MSAA. As a
    /// consequence, partial transparency in the texture is *not* preserved: use
    /// [`AlphaMode::Dither`] or [`AlphaMode::Blend`] for that instead. See
    /// ["Anti-aliased Alpha Test: The Esoteric Alpha To Coverage"] for details.
    ///
    /// Alpha to coverage provides improved performance and better visual
    /// fidelity over [`AlphaMode::Blend`], as Bevy doesn't have to sort objects
    /// when it's in use.
    ///
    /// [alpha to coverage]: https://en.wikipedia.org/wiki/Alpha_to_coverage
    /// [`StandardMaterial`]: https://docs.rs/bevy/latest/bevy/pbr/struct.StandardMaterial.html
    /// ["Anti-aliased Alpha Test: The Esoteric Alpha To Coverage"]: https://bgolus.medium.com/anti-aliased-alpha-test-the-esoteric-alpha-to-coverage-8b177335ae4f
    AlphaToCoverage,
    /// Approximates transparency by discarding a fraction of the fragments
    /// proportional to the alpha value, following a 4x4 [ordered dithering]
    /// pattern in screen space (also known as "screen-door transparency").
    ///
    /// Dithered objects are rendered in the opaque pass: they write depth,
    /// cast (dithered) shadows, and work with the depth prepass and deferred
    /// rendering. Unlike [`AlphaMode::AlphaToCoverage`], it doesn't require MSAA.
    ///
    /// Each mesh instance uses one of 16 variations of the pattern, so
    /// overlapping dithered objects don't discard the same pixels, and can be
    /// seen through one another. The pattern is fixed in screen space and
    /// doesn't change between frames.
    ///
    /// Useful for cheaply fading objects in and out, e.g. when they occlude the
    /// camera or when they spawn and despawn.
    ///
    /// [ordered dithering]: https://en.wikipedia.org/wiki/Ordered_dithering
    Dither,
    /// Combines the color of the fragments with the colors behind them in an
    /// additive process, (i.e. like light) producing lighter results.
    ///
    /// Black produces no effect. Alpha values can be used to modulate the result.
    ///
    /// Useful for effects like holograms, ghosts, lasers and other energy beams.
    Add,
    /// Combines the color of the fragments with the colors behind them in a
    /// multiplicative process, (i.e. like pigments) producing darker results.
    ///
    /// White produces no effect. Alpha values can be used to modulate the result.
    ///
    /// Useful for effects like stained glass, window tint film and some colored liquids.
    Multiply,
}

impl Eq for AlphaMode {}
