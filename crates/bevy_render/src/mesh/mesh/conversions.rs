//! These implementations allow you to
//! convert `std::vec::Vec<T>` to `VertexAttributeValues::T` and back.
//!
//! # Examples
//!
//! ```
//! use bevy_render::mesh::VertexAttributeValues;
//!
//! // creating std::vec::Vec
//! let buffer = vec![[0_u32; 4]; 10];
//!
//! // converting std::vec::Vec to bevy_render::mesh::VertexAttributeValues
//! let values = VertexAttributeValues::from(buffer.clone());
//!
//! // converting bevy_render::mesh::VertexAttributeValues to std::vec::Vec with two ways
//! let result_into: Vec<[u32; 4]> = values.clone().try_into().unwrap();
//! let result_from: Vec<[u32; 4]> = Vec::try_from(values.clone()).unwrap();
//!
//! // getting an error when trying to convert incorrectly
//! let error: Result<Vec<u32>, _> = values.try_into();
//!
//! assert_eq!(buffer, result_into);
//! assert_eq!(buffer, result_from);
//! assert!(error.is_err());
//! ```

use crate::mesh::VertexAttributeValues;
use bevy_math::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec3A, Vec4};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
#[error("cannot convert VertexAttributeValues::{variant} to {into}")]
pub struct FromVertexAttributeError {
    from: VertexAttributeValues,
    variant: &'static str,
    into: &'static str,
}

impl FromVertexAttributeError {
    fn new<T: 'static>(from: VertexAttributeValues) -> Self {
        Self {
            variant: from.enum_variant_name(),
            into: std::any::type_name::<T>(),
            from,
        }
    }
}

macro_rules! impl_from {
    ($from:ty, $variant:tt) => {
        impl From<Vec<$from>> for VertexAttributeValues {
            fn from(vec: Vec<$from>) -> Self {
                VertexAttributeValues::$variant(vec)
            }
        }
    };
}

macro_rules! impl_from_into {
    ($from:ty, $variant:tt) => {
        impl From<Vec<$from>> for VertexAttributeValues {
            fn from(vec: Vec<$from>) -> Self {
                let vec: Vec<_> = vec.into_iter().map(|t| t.into()).collect();
                VertexAttributeValues::$variant(vec)
            }
        }
    };
}

impl_from!(f32, Float32);
impl_from!([f32; 2], Float32x2);
impl_from_into!(Vec2, Float32x2);
impl_from!([f32; 3], Float32x3);
impl_from_into!(Vec3, Float32x3);
impl_from_into!(Vec3A, Float32x3);
impl_from!([f32; 4], Float32x4);
impl_from_into!(Vec4, Float32x4);

impl_from!(i32, Sint32);
impl_from!([i32; 2], Sint32x2);
impl_from_into!(IVec2, Sint32x2);
impl_from!([i32; 3], Sint32x3);
impl_from_into!(IVec3, Sint32x3);
impl_from!([i32; 4], Sint32x4);
impl_from_into!(IVec4, Sint32x4);

impl_from!(u32, Uint32);
impl_from!([u32; 2], Uint32x2);
impl_from_into!(UVec2, Uint32x2);
impl_from!([u32; 3], Uint32x3);
impl_from_into!(UVec3, Uint32x3);
impl_from!([u32; 4], Uint32x4);
impl_from_into!(UVec4, Uint32x4);

macro_rules! impl_try_from {
    ($into:ty, $($variant:tt), +) => {
        impl TryFrom<VertexAttributeValues> for Vec<$into> {
            type Error = FromVertexAttributeError;

            fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
                match value {
                    $(VertexAttributeValues::$variant(value)) |+ => Ok(value),
                    _ => Err(FromVertexAttributeError::new::<Self>(value)),
                }
            }
        }
    };
}

macro_rules! impl_try_from_into {
    ($into:ty, $($variant:tt), +) => {
        impl TryFrom<VertexAttributeValues> for Vec<$into> {
            type Error = FromVertexAttributeError;

            fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
                match value {
                    $(VertexAttributeValues::$variant(value)) |+ => {
                        Ok(value.into_iter().map(|t| t.into()).collect())
                    }
                    _ => Err(FromVertexAttributeError::new::<Self>(value)),
                }
            }
        }
    };
}

impl_try_from!(f32, Float32);
impl_try_from!([f32; 2], Float32x2);
impl_try_from_into!(Vec2, Float32x2);
impl_try_from!([f32; 3], Float32x3);
impl_try_from_into!(Vec3, Float32x3);
impl_try_from_into!(Vec3A, Float32x3);
impl_try_from!([f32; 4], Float32x4);
impl_try_from_into!(Vec4, Float32x4);

impl_try_from!(i32, Sint32);
impl_try_from!([i32; 2], Sint32x2);
impl_try_from_into!(IVec2, Sint32x2);
impl_try_from!([i32; 3], Sint32x3);
impl_try_from_into!(IVec3, Sint32x3);
impl_try_from!([i32; 4], Sint32x4);
impl_try_from_into!(IVec4, Sint32x4);

impl_try_from!(u32, Uint32);
impl_try_from!([u32; 2], Uint32x2);
impl_try_from_into!(UVec2, Uint32x2);
impl_try_from!([u32; 3], Uint32x3);
impl_try_from_into!(UVec3, Uint32x3);
impl_try_from!([u32; 4], Uint32x4);
impl_try_from_into!(UVec4, Uint32x4);

impl_try_from!([i8; 2], Sint8x2, Snorm8x2);
impl_try_from!([i8; 4], Sint8x4, Snorm8x4);

impl_try_from!([u8; 2], Uint8x2, Unorm8x2);
impl_try_from!([u8; 4], Uint8x4, Unorm8x4);

impl_try_from!([i16; 2], Sint16x2, Snorm16x2);
impl_try_from!([i16; 4], Sint16x4, Snorm16x4);

impl_try_from!([u16; 2], Uint16x2, Unorm16x2);
impl_try_from!([u16; 4], Uint16x4, Unorm16x4);

#[cfg(test)]
mod tests {
    use bevy_math::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec3A, Vec4};

    use super::VertexAttributeValues;
    #[test]
    fn f32() {
        let buffer = vec![0.0; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<f32> = values.clone().try_into().unwrap();
        let result_from: Vec<f32> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn i32() {
        let buffer = vec![0; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<i32> = values.clone().try_into().unwrap();
        let result_from: Vec<i32> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn u32() {
        let buffer = vec![0_u32; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<u32> = values.clone().try_into().unwrap();
        let result_from: Vec<u32> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<f32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn f32_2() {
        let buffer = vec![[0.0; 2]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<[f32; 2]> = values.clone().try_into().unwrap();
        let result_from: Vec<[f32; 2]> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn vec2() {
        let buffer = vec![Vec2::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert!(matches!(values, VertexAttributeValues::Float32x2(_)));
        let result_into: Vec<Vec2> = values.clone().try_into().unwrap();
        let result_from: Vec<Vec2> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn i32_2() {
        let buffer = vec![[0; 2]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<[i32; 2]> = values.clone().try_into().unwrap();
        let result_from: Vec<[i32; 2]> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn ivec2() {
        let buffer = vec![IVec2::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert!(matches!(values, VertexAttributeValues::Sint32x2(_)));
        let result_into: Vec<IVec2> = values.clone().try_into().unwrap();
        let result_from: Vec<IVec2> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn u32_2() {
        let buffer = vec![[0_u32; 2]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<[u32; 2]> = values.clone().try_into().unwrap();
        let result_from: Vec<[u32; 2]> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn uvec2() {
        let buffer = vec![UVec2::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert!(matches!(values, VertexAttributeValues::Uint32x2(_)));
        let result_into: Vec<UVec2> = values.clone().try_into().unwrap();
        let result_from: Vec<UVec2> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn f32_3() {
        let buffer = vec![[0.0; 3]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<[f32; 3]> = values.clone().try_into().unwrap();
        let result_from: Vec<[f32; 3]> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn vec3() {
        let buffer = vec![Vec3::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert!(matches!(values, VertexAttributeValues::Float32x3(_)));
        let result_into: Vec<Vec3> = values.clone().try_into().unwrap();
        let result_from: Vec<Vec3> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn vec3a() {
        let buffer = vec![Vec3A::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert!(matches!(values, VertexAttributeValues::Float32x3(_)));
        let result_into: Vec<Vec3A> = values.clone().try_into().unwrap();
        let result_from: Vec<Vec3A> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn i32_3() {
        let buffer = vec![[0; 3]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<[i32; 3]> = values.clone().try_into().unwrap();
        let result_from: Vec<[i32; 3]> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn ivec3() {
        let buffer = vec![IVec3::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert!(matches!(values, VertexAttributeValues::Sint32x3(_)));
        let result_into: Vec<IVec3> = values.clone().try_into().unwrap();
        let result_from: Vec<IVec3> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn u32_3() {
        let buffer = vec![[0_u32; 3]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<[u32; 3]> = values.clone().try_into().unwrap();
        let result_from: Vec<[u32; 3]> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn uvec3() {
        let buffer = vec![UVec3::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert!(matches!(values, VertexAttributeValues::Uint32x3(_)));
        let result_into: Vec<UVec3> = values.clone().try_into().unwrap();
        let result_from: Vec<UVec3> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn f32_4() {
        let buffer = vec![[0.0; 4]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<[f32; 4]> = values.clone().try_into().unwrap();
        let result_from: Vec<[f32; 4]> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn vec4() {
        let buffer = vec![Vec4::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert!(matches!(values, VertexAttributeValues::Float32x4(_)));
        let result_into: Vec<Vec4> = values.clone().try_into().unwrap();
        let result_from: Vec<Vec4> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn i32_4() {
        let buffer = vec![[0; 4]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<[i32; 4]> = values.clone().try_into().unwrap();
        let result_from: Vec<[i32; 4]> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn ivec4() {
        let buffer = vec![IVec4::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert!(matches!(values, VertexAttributeValues::Sint32x4(_)));
        let result_into: Vec<IVec4> = values.clone().try_into().unwrap();
        let result_from: Vec<IVec4> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn u32_4() {
        let buffer = vec![[0_u32; 4]; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        let result_into: Vec<[u32; 4]> = values.clone().try_into().unwrap();
        let result_from: Vec<[u32; 4]> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn uvec4() {
        let buffer = vec![UVec4::ZERO; 10];
        let values = VertexAttributeValues::from(buffer.clone());
        assert!(matches!(values, VertexAttributeValues::Uint32x4(_)));
        let result_into: Vec<UVec4> = values.clone().try_into().unwrap();
        let result_from: Vec<UVec4> = Vec::try_from(values.clone()).unwrap();
        let error: Result<Vec<u32>, _> = values.try_into();
        assert_eq!(buffer, result_into);
        assert_eq!(buffer, result_from);
        assert!(error.is_err());
    }

    #[test]
    fn correct_message() {
        let buffer = vec![[0_u32; 4]; 3];
        let values = VertexAttributeValues::from(buffer);
        let error_result: Result<Vec<u32>, _> = values.try_into();
        let error = match error_result {
            Ok(..) => unreachable!(),
            Err(error) => error,
        };
        assert_eq!(
            error.to_string(),
            "cannot convert VertexAttributeValues::Uint32x4 to alloc::vec::Vec<u32>"
        );
        assert_eq!(format!("{error:?}"),
               "FromVertexAttributeError { from: Uint32x4([[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]), variant: \"Uint32x4\", into: \"alloc::vec::Vec<u32>\" }");
    }
}
