use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;

// TODO: add discussion about performance.
/// Sets how a material's base color alpha channel is used for transparency.
#[derive(Debug, Default, Reflect, Copy, Clone, PartialEq)]
#[reflect(Default, Debug)]
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
    /// Spreads the fragment out over a hardware-dependent number of sample
    /// locations proportional to the alpha value. This requires multisample
    /// antialiasing; if MSAA isn't on, this is identical to
    /// [`AlphaMode::Mask`] with a value of 0.5.
    ///
    /// Alpha to coverage provides improved performance and better visual
    /// fidelity over [`AlphaMode::Blend`], as Bevy doesn't have to sort objects
    /// when it's in use. It's especially useful for complex transparent objects
    /// like foliage.
    ///
    /// [alpha to coverage]: https://en.wikipedia.org/wiki/Alpha_to_coverage
    AlphaToCoverage,
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
