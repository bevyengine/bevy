use crate as bevy_reflect;
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_math::Rotation2d;
use bevy_reflect_derive::impl_reflect;

impl_reflect!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize)]
    #[type_path = "bevy_math"]
    struct Rotation2d {
        cos: f32,
        sin: f32,
    }
);
