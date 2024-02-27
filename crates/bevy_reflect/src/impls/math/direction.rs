use crate as bevy_reflect;
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::bevy_math::Direction2d(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::bevy_math::Direction3d(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::bevy_math::Direction3dA(
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));
