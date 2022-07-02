use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;

// FIXME: This should probably be part of bevy_render2!
/// Alpha mode
#[derive(Component, Debug, Default, Reflect, Copy, Clone, PartialEq)]
#[reflect(Component, Default)]
pub enum AlphaMode {
    #[default]
    Opaque,
    /// An alpha cutoff must be supplied where alpha values >= the cutoff
    /// will be fully opaque and < will be fully transparent
    Mask(f32),
    Blend,
}

impl Eq for AlphaMode {}
