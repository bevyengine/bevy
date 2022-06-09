use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;

// FIXME: This should probably be part of bevy_render2!
/// Alpha mode
#[derive(Component, Debug, Reflect, Copy, Clone, PartialEq)]
#[reflect(Component, Default)]
pub enum AlphaMode {
    /// Completely opaque texture/color; alpha values disregarded.
    Opaque,
    /// An alpha cutoff must be supplied where alpha values >= the cutoff
    /// will be fully opaque and < will be fully transparent.
    Mask(f32),
    /// A middle ground between [AlphaMode::Mask] and [AlphaMode::Blend].
    /// Dithers between texture/color and background by hashing object space coordinates.
    /// Pixels where alpha values >= hash will be fully opaque and < will be fully transparent.
    /// Ideal for textures with noisy alpha values (ex: hair and foliage).
    Hashed,
    /// Alpha values mix texture/color with background.
    Blend,
}

impl Eq for AlphaMode {}

impl Default for AlphaMode {
    fn default() -> Self {
        AlphaMode::Opaque
    }
}
