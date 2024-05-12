use crate as bevy_reflect;
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::bevy_math::Dir2(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::bevy_math::Dir3(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::bevy_math::Dir3A(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize
));
