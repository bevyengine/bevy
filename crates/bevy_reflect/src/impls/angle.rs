use crate as bevy_reflect;
use crate::prelude::ReflectDefault;
use bevy_reflect_derive::impl_reflect_value;

impl_reflect_value!(::bevy_math::Angle(Debug, Default));
