use crate::{std_traits::ReflectDefault, ReflectDeserialize, ReflectSerialize};
use assert_type_match::assert_type_match;
use bevy_reflect_derive::{impl_reflect, impl_reflect_opaque};
use glam::*;

/// Reflects the given foreign type as an enum and asserts that the variants/fields match up.
macro_rules! reflect_enum {
    ($(#[$meta:meta])* enum $ident:ident { $($ty:tt)* } ) => {
        impl_reflect!($(#[$meta])* enum $ident { $($ty)* });

        #[assert_type_match($ident, test_only)]
        #[expect(
            clippy::upper_case_acronyms,
            reason = "The variants used are not acronyms."
        )]
        enum $ident { $($ty)* }
    };
}

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct IVec2 {
        x: i32,
        y: i32,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct IVec3 {
        x: i32,
        y: i32,
        z: i32,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct IVec4 {
        x: i32,
        y: i32,
        z: i32,
        w: i32,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct I8Vec2 {
        x: i8,
        y: i8,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct I8Vec3 {
        x: i8,
        y: i8,
        z: i8,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct I8Vec4 {
        x: i8,
        y: i8,
        z: i8,
        w: i8,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct I16Vec2 {
        x: i16,
        y: i16,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct I16Vec3 {
        x: i16,
        y: i16,
        z: i16,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct I16Vec4 {
        x: i16,
        y: i16,
        z: i16,
        w: i16,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct I64Vec2 {
        x: i64,
        y: i64,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct I64Vec3 {
        x: i64,
        y: i64,
        z: i64,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct I64Vec4 {
        x: i64,
        y: i64,
        z: i64,
        w: i64,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct UVec2 {
        x: u32,
        y: u32,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct UVec3 {
        x: u32,
        y: u32,
        z: u32,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct UVec4 {
        x: u32,
        y: u32,
        z: u32,
        w: u32,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct U8Vec2 {
        x: u8,
        y: u8,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct U8Vec3 {
        x: u8,
        y: u8,
        z: u8,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct U8Vec4 {
        x: u8,
        y: u8,
        z: u8,
        w: u8,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct U16Vec2 {
        x: u16,
        y: u16,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct U16Vec3 {
        x: u16,
        y: u16,
        z: u16,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct U16Vec4 {
        x: u16,
        y: u16,
        z: u16,
        w: u16,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct U64Vec2 {
        x: u64,
        y: u64,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct U64Vec3 {
        x: u64,
        y: u64,
        z: u64,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, Hash, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct U64Vec4 {
        x: u64,
        y: u64,
        z: u64,
        w: u64,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Vec2 {
        x: f32,
        y: f32,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Vec3A {
        x: f32,
        y: f32,
        z: f32,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Vec4 {
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct BVec2 {
        x: bool,
        y: bool,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct BVec3 {
        x: bool,
        y: bool,
        z: bool,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct BVec4 {
        x: bool,
        y: bool,
        z: bool,
        w: bool,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct DVec2 {
        x: f64,
        y: f64,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct DVec3 {
        x: f64,
        y: f64,
        z: f64,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct DVec4 {
        x: f64,
        y: f64,
        z: f64,
        w: f64,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Mat2 {
        x_axis: Vec2,
        y_axis: Vec2,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Mat3 {
        x_axis: Vec3,
        y_axis: Vec3,
        z_axis: Vec3,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Mat3A {
        x_axis: Vec3A,
        y_axis: Vec3A,
        z_axis: Vec3A,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Mat4 {
        x_axis: Vec4,
        y_axis: Vec4,
        z_axis: Vec4,
        w_axis: Vec4,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct DMat2 {
        x_axis: DVec2,
        y_axis: DVec2,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct DMat3 {
        x_axis: DVec3,
        y_axis: DVec3,
        z_axis: DVec3,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct DMat4 {
        x_axis: DVec4,
        y_axis: DVec4,
        z_axis: DVec4,
        w_axis: DVec4,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Affine2 {
        matrix2: Mat2,
        translation: Vec2,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Affine3A {
        matrix3: Mat3A,
        translation: Vec3A,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct DAffine2 {
        matrix2: DMat2,
        translation: DVec2,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct DAffine3 {
        matrix3: DMat3,
        translation: DVec3,
    }
);

impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct Quat {
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    }
);
impl_reflect!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    struct DQuat {
        x: f64,
        y: f64,
        z: f64,
        w: f64,
    }
);

reflect_enum!(
    #[reflect(Clone, Debug, PartialEq, Default, Deserialize, Serialize)]
    #[type_path = "glam"]
    enum EulerRot {
        ZYX,
        ZXY,
        YXZ,
        YZX,
        XYZ,
        XZY,
        ZYZ,
        ZXZ,
        YXY,
        YZY,
        XYX,
        XZX,
        ZYXEx,
        ZXYEx,
        YXZEx,
        YZXEx,
        XYZEx,
        XZYEx,
        ZYZEx,
        ZXZEx,
        YXYEx,
        YZYEx,
        XYXEx,
        XZXEx,
    }
);

impl_reflect_opaque!(::glam::BVec3A(
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize
));
impl_reflect_opaque!(::glam::BVec4A(
    Clone,
    Debug,
    Default,
    Deserialize,
    Serialize
));

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};
    use ron::{
        ser::{to_string_pretty, PrettyConfig},
        Deserializer,
    };
    use serde::de::DeserializeSeed;
    use static_assertions::assert_impl_all;

    use crate::{
        prelude::*,
        serde::{ReflectDeserializer, ReflectSerializer},
        Enum, GetTypeRegistration, TypeRegistry,
    };

    use super::*;

    assert_impl_all!(EulerRot: Enum);

    #[test]
    fn euler_rot_serialization() {
        let v = EulerRot::YXZ;

        let mut registry = TypeRegistry::default();
        registry.register::<EulerRot>();

        let ser = ReflectSerializer::new(&v, &registry);

        let config = PrettyConfig::default()
            .new_line(String::from("\n"))
            .indentor(String::from("    "));
        let output = to_string_pretty(&ser, config).unwrap();
        let expected = r#"
{
    "glam::EulerRot": YXZ,
}"#;

        assert_eq!(expected, format!("\n{output}"));
    }

    #[test]
    fn euler_rot_deserialization() {
        let data = r#"
{
    "glam::EulerRot": XZY,
}"#;

        let mut registry = TypeRegistry::default();
        registry.add_registration(EulerRot::get_type_registration());

        let de = ReflectDeserializer::new(&registry);

        let mut deserializer =
            Deserializer::from_str(data).expect("Failed to acquire deserializer");

        let dynamic_struct = de
            .deserialize(&mut deserializer)
            .expect("Failed to deserialize");

        let mut result = EulerRot::default();

        result.apply(dynamic_struct.as_partial_reflect());

        assert_eq!(result, EulerRot::XZY);
    }
}
