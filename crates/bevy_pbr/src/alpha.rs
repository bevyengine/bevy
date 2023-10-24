use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_render::render_resource::BlendState;
use bitbybit::bitenum;

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
impl AlphaMode {
    pub fn may_discard(self) -> bool {
        matches!(self, Self::Mask(_))
    }
}

#[bitenum(u2, exhaustive: true)]
#[derive(PartialEq)]
pub enum BlendMode {
    Opaque = 0,
    PremultipliedAlpha = 1,
    Multiply = 2,
    Alpha = 3,
}
impl From<AlphaMode> for BlendMode {
    fn from(value: AlphaMode) -> Self {
        match value {
            AlphaMode::Premultiplied | AlphaMode::Add => BlendMode::PremultipliedAlpha,
            AlphaMode::Blend => BlendMode::Alpha,
            AlphaMode::Multiply => BlendMode::Multiply,
            _ => BlendMode::Opaque,
        }
    }
}
impl BlendMode {
    pub fn is_opaque(self) -> bool {
        matches!(self, Self::Opaque)
    }
    pub fn state(self) -> Option<BlendState> {
        use bevy_render::render_resource::*;
        match self {
            BlendMode::PremultipliedAlpha => Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
            BlendMode::Multiply => Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::Dst,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent::OVER,
            }),
            BlendMode::Alpha => Some(BlendState::ALPHA_BLENDING),
            BlendMode::Opaque => None,
        }
    }
    pub const fn defines(self) -> Option<[&'static str; 2]> {
        match self {
            BlendMode::Alpha | BlendMode::Opaque => None,
            BlendMode::PremultipliedAlpha => {
                Some(["PREMULTIPLY_ALPHA", "BLEND_PREMULTIPLIED_ALPHA"])
            }
            BlendMode::Multiply => Some(["PREMULTIPLY_ALPHA", "BLEND_MULTIPLY"]),
        }
    }
}

impl Eq for AlphaMode {}
