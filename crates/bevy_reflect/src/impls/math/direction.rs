use crate as bevy_reflect;
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::bevy_math::Dir2(Debug, PartialEq, Serialize, Deserialize));
impl_reflect_value!(::bevy_math::Dir3(Debug, PartialEq, Serialize, Deserialize));
impl_reflect_value!(::bevy_math::Dir3A(Debug, PartialEq, Serialize, Deserialize));
