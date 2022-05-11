use crate as bevy_reflect;
use crate::prelude::ReflectDefault;
use crate::reflect::Reflect;
use crate::ReflectDeserialize;
use bevy_reflect_derive::{impl_from_reflect_value, impl_reflect_struct, impl_reflect_value};
use glam::*;

impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct IVec2 {
        x: i32,
        y: i32,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct IVec3 {
        x: i32,
        y: i32,
        z: i32,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct IVec4 {
        x: i32,
        y: i32,
        z: i32,
        w: i32,
    }
);

impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct UVec2 {
        x: u32,
        y: u32,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct UVec3 {
        x: u32,
        y: u32,
        z: u32,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct UVec4 {
        x: u32,
        y: u32,
        z: u32,
        w: u32,
    }
);

impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct Vec2 {
        x: f32,
        y: f32,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct Vec3A {
        x: f32,
        y: f32,
        z: f32,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct Vec4 {
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    }
);

impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct DVec2 {
        x: f64,
        y: f64,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct DVec3 {
        x: f64,
        y: f64,
        z: f64,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct DVec4 {
        x: f64,
        y: f64,
        z: f64,
        w: f64,
    }
);

impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct Mat3 {
        x_axis: Vec3,
        y_axis: Vec3,
        z_axis: Vec3,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct Mat4 {
        x_axis: Vec4,
        y_axis: Vec4,
        z_axis: Vec4,
        w_axis: Vec4,
    }
);

impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct DMat3 {
        x_axis: DVec3,
        y_axis: DVec3,
        z_axis: DVec3,
    }
);
impl_reflect_struct!(
    #[reflect(PartialEq, Serialize, Deserialize, Default)]
    struct DMat4 {
        x_axis: DVec4,
        y_axis: DVec4,
        z_axis: DVec4,
        w_axis: DVec4,
    }
);

// Quat fields are read-only (as of now), and reflection is currently missing
// mechanisms for read-only fields. I doubt those mechanisms would be added,
// so for now quaternions will remain as values. They are represented identically
// to Vec4 and DVec4, so you may use those instead and convert between.
impl_reflect_value!(Quat(PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(DQuat(PartialEq, Serialize, Deserialize, Default));

impl_from_reflect_value!(Quat);
impl_from_reflect_value!(DQuat);
