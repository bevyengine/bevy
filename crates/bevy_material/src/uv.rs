use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// An enum to define which UV attribute to use for a texture.
///
/// It only supports two UV attributes, [`bevy_mesh::Mesh::ATTRIBUTE_UV_0`] and
/// [`bevy_mesh::Mesh::ATTRIBUTE_UV_1`].
/// The default is [`UvChannel::Uv0`].
#[derive(Reflect, Default, Debug, Clone, PartialEq, Eq)]
#[reflect(Default, Debug, Clone, PartialEq)]
pub enum UvChannel {
    #[default]
    Uv0,
    Uv1,
}
