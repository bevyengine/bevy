use crate::prelude::ReflectDefault;
use crate::{self as bevy_reflect, ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::bevy_math::Angle(
    Debug,
    Default,
    PartialEq,
    Serialize,
    Deserialize,
));
