use crate as bevy_reflect;
use crate::ReflectDeserialize;
use crate::reflect::Reflect;
use bevy_reflect_derive::{impl_from_reflect_value, impl_reflect_value, impl_reflect_struct_and_from_reflect_struct};
use glam::*;

// impl_reflect_value!(IVec2(PartialEq, Serialize, Deserialize));
impl_reflect_value!(IVec3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(IVec4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(UVec2(PartialEq, Serialize, Deserialize));
impl_reflect_value!(UVec3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(UVec4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Vec2(PartialEq, Serialize, Deserialize));
// impl_reflect_value!(Vec3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Vec3A(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Vec4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(DVec2(PartialEq, Serialize, Deserialize));
impl_reflect_value!(DVec3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(DVec4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Mat3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Mat4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(Quat(PartialEq, Serialize, Deserialize));
impl_reflect_value!(DMat3(PartialEq, Serialize, Deserialize));
impl_reflect_value!(DMat4(PartialEq, Serialize, Deserialize));
impl_reflect_value!(DQuat(PartialEq, Serialize, Deserialize));

/*
==========================
|     Updated impls      |
==========================
*/

impl_reflect_struct_and_from_reflect_struct!(
    Constructor(Default::default())
    #[reflect(PartialEq, Serialize, Deserialize)]
    struct IVec2 {
        x: i32,
        y: i32
    }
);

impl_reflect_struct_and_from_reflect_struct!(
    Constructor(Default::default())
    #[reflect(PartialEq, Serialize, Deserialize)]
    struct Vec3 {
        x: f32,
        y: f32,
        z: f32
    }
);

// impl_from_reflect_value!(IVec2);
impl_from_reflect_value!(IVec3);
impl_from_reflect_value!(IVec4);
impl_from_reflect_value!(UVec2);
impl_from_reflect_value!(UVec3);
impl_from_reflect_value!(UVec4);
impl_from_reflect_value!(Vec2);
// impl_from_reflect_value!(Vec3);
impl_from_reflect_value!(Vec4);
impl_from_reflect_value!(Vec3A);
impl_from_reflect_value!(DVec2);
impl_from_reflect_value!(DVec3);
impl_from_reflect_value!(DVec4);
impl_from_reflect_value!(Mat3);
impl_from_reflect_value!(Mat4);
impl_from_reflect_value!(Quat);
impl_from_reflect_value!(DMat3);
impl_from_reflect_value!(DMat4);
impl_from_reflect_value!(DQuat);

#[test]
fn temp_test() {
    let v = vec3(12.0, 0.0, 0.0);

    let refl: &dyn Reflect = &v;

    assert!(matches!(refl.reflect_ref(), bevy_reflect::ReflectRef::Struct(_)));

    assert!(match refl.reflect_ref() {
        bevy_reflect::ReflectRef::Struct(s) => s.field("x").is_some(),
        _ => false
    });

    assert!(match refl.reflect_ref() {
        bevy_reflect::ReflectRef::Struct(s) => s.field("x").unwrap().downcast_ref::<f32>().unwrap() == &12.0,
        _ => false
    });

    use bevy_reflect::FromReflect;
    let v2 = Vec3::from_reflect(refl).unwrap();

    assert_eq!(v2.x, 12.0);

    assert!(refl.reflect_partial_eq(&v2).is_some() && refl.reflect_partial_eq(&v2).unwrap())
}