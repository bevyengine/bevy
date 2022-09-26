use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;

// TODO: add discussion about performance.
/// Sets how a material's base color alpha channel is used for transparency.
#[derive(Component, Debug, Default, Reflect, Copy, Clone, PartialEq)]
#[reflect(Component, Default)]
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
}

impl Eq for AlphaMode {}
