use crate as bevy_reflect;
use crate::prelude::ReflectDefault;
use crate::reflect::Reflect;
use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::{impl_from_reflect_value, impl_reflect_struct, impl_reflect_value};
use glam::*;

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::IVec2")]
    struct IVec2 {
        x: i32,
        y: i32,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::IVec3")]
    struct IVec3 {
        x: i32,
        y: i32,
        z: i32,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::IVec4")]
    struct IVec4 {
        x: i32,
        y: i32,
        z: i32,
        w: i32,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::UVec2")]
    struct UVec2 {
        x: u32,
        y: u32,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::UVec3")]
    struct UVec3 {
        x: u32,
        y: u32,
        z: u32,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::UVec4")]
    struct UVec4 {
        x: u32,
        y: u32,
        z: u32,
        w: u32,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::Vec2")]
    struct Vec2 {
        x: f32,
        y: f32,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::Vec3")]
    struct Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::Vec3A")]
    struct Vec3A {
        x: f32,
        y: f32,
        z: f32,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::Vec4")]
    struct Vec4 {
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Default)]
    #[type_path("glam::BVec2")]
    struct BVec2 {
        x: bool,
        y: bool,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Default)]
    #[type_path("glam::BVec3")]
    struct BVec3 {
        x: bool,
        y: bool,
        z: bool,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Default)]
    #[type_path("glam::BVec4")]
    struct BVec4 {
        x: bool,
        y: bool,
        z: bool,
        w: bool,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::DVec2")]
    struct DVec2 {
        x: f64,
        y: f64,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::DVec3")]
    struct DVec3 {
        x: f64,
        y: f64,
        z: f64,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::DVec4")]
    struct DVec4 {
        x: f64,
        y: f64,
        z: f64,
        w: f64,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::Mat2")]
    struct Mat2 {
        x_axis: Vec2,
        y_axis: Vec2,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::Mat3")]
    struct Mat3 {
        x_axis: Vec3,
        y_axis: Vec3,
        z_axis: Vec3,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::Mat3A")]
    struct Mat3A {
        x_axis: Vec3A,
        y_axis: Vec3A,
        z_axis: Vec3A,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::Mat4")]
    struct Mat4 {
        x_axis: Vec4,
        y_axis: Vec4,
        z_axis: Vec4,
        w_axis: Vec4,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::DMat2")]
    struct DMat2 {
        x_axis: DVec2,
        y_axis: DVec2,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::DMat3")]
    struct DMat3 {
        x_axis: DVec3,
        y_axis: DVec3,
        z_axis: DVec3,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::DMat4")]
    struct DMat4 {
        x_axis: DVec4,
        y_axis: DVec4,
        z_axis: DVec4,
        w_axis: DVec4,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::Affine2")]
    struct Affine2 {
        matrix2: Mat2,
        translation: Vec2,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::Affine3A")]
    struct Affine3A {
        matrix3: Mat3A,
        translation: Vec3A,
    }
);

impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::DAffine2")]
    struct DAffine2 {
        matrix2: DMat2,
        translation: DVec2,
    }
);
impl_reflect_struct!(
    #[reflect(Debug, PartialEq, Serialize, Deserialize, Default)]
    #[type_path("glam::DAffine3")]
    struct DAffine3 {
        matrix3: DMat3,
        translation: DVec3,
    }
);

// Quat fields are read-only (as of now), and reflection is currently missing
// mechanisms for read-only fields. I doubt those mechanisms would be added,
// so for now quaternions will remain as values. They are represented identically
// to Vec4 and DVec4, so you may use those instead and convert between.
impl_reflect_value!(@"glam::Quat" Quat(Debug, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(@"glam::DQuat" DQuat(Debug, PartialEq, Serialize, Deserialize, Default));

impl_from_reflect_value!(Quat);
impl_from_reflect_value!(DQuat);

impl_reflect_value!(@"glam::EulerRot" EulerRot(Debug, Default));

// glam type aliases these to the non simd versions when there is no support (this breaks wasm builds for example)
// ideally it shouldn't do that and there's an issue on glam for this
// https://github.com/bitshifter/glam-rs/issues/306
#[cfg(any(target_feature = "sse2", target_feature = "simd128"))]
impl_reflect_value!(@"glam::BVec3A" BVec3A(Debug, PartialEq, Default));
#[cfg(any(target_feature = "sse2", target_feature = "simd128"))]
impl_reflect_value!(@"glam::BVec4A" BVec4A(Debug, PartialEq, Default));

#[cfg(test)]
mod tests {
    use glam::*;

    use crate::TypePath;

    #[test]
    fn type_name_should_be_prefixed_with_glam() {
        macro_rules! assert_name {
            ($t:ty) => {
                assert_eq!(
                    <$t as TypePath>::type_path(),
                    concat!("glam::", stringify!($t))
                )
            };
        }

        assert_name!(IVec2);
        assert_name!(IVec3);
        assert_name!(IVec4);
        assert_name!(UVec2);
        assert_name!(UVec3);
        assert_name!(UVec4);
        assert_name!(Vec2);
        assert_name!(Vec3);
        assert_name!(Vec3A);
        assert_name!(Vec4);
        assert_name!(BVec2);
        assert_name!(BVec3);
        assert_name!(BVec4);
        assert_name!(DVec2);
        assert_name!(DVec3);
        assert_name!(DVec4);
        assert_name!(Mat2);
        assert_name!(Mat3);
        assert_name!(Mat3A);
        assert_name!(Mat4);
        assert_name!(DMat2);
        assert_name!(DMat3);
        assert_name!(DMat4);
        assert_name!(Affine2);
        assert_name!(Affine3A);
        assert_name!(DAffine2);
        assert_name!(DAffine2);
        assert_name!(Quat);
        assert_name!(DQuat);
        assert_name!(EulerRot);
        #[cfg(any(target_feature = "sse2", target_feature = "simd128"))]
        assert_name!(BVec3A);
        #[cfg(any(target_feature = "sse2", target_feature = "simd128"))]
        assert_name!(BVec4A);
    }
}
