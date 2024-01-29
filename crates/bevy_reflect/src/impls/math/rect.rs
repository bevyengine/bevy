use crate as bevy_reflect;
use crate::prelude::ReflectDefault;
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_math::{IRect, IVec2, Rect, URect, UVec2, Vec2};
use bevy_reflect_derive::impl_reflect_struct;

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Hash, Serialize, Deserialize, Default)]
    #[type_path = "bevy_math"]
    struct IRect {
        min: IVec2,
        max: IVec2,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path = "bevy_math"]
    struct Rect {
        min: Vec2,
        max: Vec2,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Hash, Serialize, Deserialize, Default)]
    #[type_path = "bevy_math"]
    struct URect {
        min: UVec2,
        max: UVec2,
    }
);
