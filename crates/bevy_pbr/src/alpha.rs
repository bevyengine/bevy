use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;

/// Alpha mode.
#[derive(Component, Debug, Default, Reflect, Copy, Clone, PartialEq)]
#[reflect(Component, Default)]
pub enum AlphaMode {
    /// Base color alpha values are overridden to be fully opaque (1.0).
    #[default]
    Opaque,
    /// An alpha cutoff must be supplied where alpha values >= the cutoff
    /// will be fully opaque (1.0) and < will be fully transparent (0.0).
    Mask(f32),
    /// The base color alpha value defines the opacity of the color.
    /// Standard alpha-blending is used to blend the fragment's color
    /// with the color behind it.
    Blend,
}

impl Eq for AlphaMode {}
