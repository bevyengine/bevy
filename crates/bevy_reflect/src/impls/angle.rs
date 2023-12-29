use crate as bevy_reflect;
use crate::prelude::ReflectDefault;
use bevy_math::float::Float;
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::bevy_math::Angle<T: Float + std::marker::Sync + std::marker::Send>(Debug, Default));
