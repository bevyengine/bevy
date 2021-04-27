use crate as bevy_reflect;
use crate::ReflectDeserialize;
use bevy_reflect_derive::impl_reflect_value;
use glam::{IVec2, IVec3, IVec4, Mat3, Mat4, Quat, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};

impl_reflect_value!(IVec2(PartialEq, Serialize, Deserialize));
impl_reflect_value!(IVec3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(IVec4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(UVec2(PartialEq, Serialize, Deserialize));
impl_reflect_value!(UVec3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(UVec4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Vec2(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Vec3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Vec4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Mat3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Mat4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Quat(PartialEq, Serialize, Deserialize));
