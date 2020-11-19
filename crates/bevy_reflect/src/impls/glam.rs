use crate::ReflectDeserialize;
use bevy_reflect_derive::impl_reflect_value;
use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4};

impl_reflect_value!(Vec2(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Vec3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Vec4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Mat3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Mat4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Quat(PartialEq, Serialize, Deserialize));
