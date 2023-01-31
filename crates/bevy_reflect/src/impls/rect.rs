use crate as bevy_reflect;
use crate::prelude::ReflectDefault;
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_math::{Aabb, Rect, Vec2, Vec3A};
use bevy_reflect_derive::impl_reflect_struct;

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    struct Rect {
        min: Vec2,
        max: Vec2,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    struct Aabb {
        pub center: Vec3A,
        pub half_extents: Vec3A,
    }
);
