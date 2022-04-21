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
use bevy_math::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};
use bevy_utils::EnumVariantMeta;
use thiserror::Error;

macro_rules! convert_vec_of_vec {
    ($x:expr, $t:ty, $s:literal) => {
        unsafe {
            // SAFETY: x is only used in this file. It is called only with x being a value of type `Vec<Vec3>` or something similar.
            // Vec3 is a repr(C) struct containing 3 f32s, hence it is transmutable to a [f32; 3].
            let mut v = std::mem::ManuallyDrop::new($x);
            Vec::from_raw_parts(v.as_mut_ptr() as *mut [$t; $s], v.len(), v.capacity())
        }
    };
}

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

impl From<Vec<f32>> for VertexAttributeValues {
    fn from(vec: Vec<f32>) -> Self {
        VertexAttributeValues::Float32(vec)
    }
}

impl From<Vec<i32>> for VertexAttributeValues {
    fn from(vec: Vec<i32>) -> Self {
        VertexAttributeValues::Sint32(vec)
    }
}

impl From<Vec<u32>> for VertexAttributeValues {
    fn from(vec: Vec<u32>) -> Self {
        VertexAttributeValues::Uint32(vec)
    }
}

impl From<Vec<[f32; 2]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 2]>) -> Self {
        VertexAttributeValues::Float32x2(vec)
    }
}

impl From<Vec<[i32; 2]>> for VertexAttributeValues {
    fn from(vec: Vec<[i32; 2]>) -> Self {
        VertexAttributeValues::Sint32x2(vec)
    }
}

impl From<Vec<[u32; 2]>> for VertexAttributeValues {
    fn from(vec: Vec<[u32; 2]>) -> Self {
        VertexAttributeValues::Uint32x2(vec)
    }
}

impl From<Vec<[f32; 3]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 3]>) -> Self {
        VertexAttributeValues::Float32x3(vec)
    }
}

impl From<Vec<[i32; 3]>> for VertexAttributeValues {
    fn from(vec: Vec<[i32; 3]>) -> Self {
        VertexAttributeValues::Sint32x3(vec)
    }
}

impl From<Vec<[u32; 3]>> for VertexAttributeValues {
    fn from(vec: Vec<[u32; 3]>) -> Self {
        VertexAttributeValues::Uint32x3(vec)
    }
}

impl From<Vec<[f32; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 4]>) -> Self {
        VertexAttributeValues::Float32x4(vec)
    }
}

impl From<Vec<[i32; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[i32; 4]>) -> Self {
        VertexAttributeValues::Sint32x4(vec)
    }
}

impl From<Vec<[u32; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[u32; 4]>) -> Self {
        VertexAttributeValues::Uint32x4(vec)
    }
}

impl From<Vec<[u16; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[u16; 4]>) -> Self {
        VertexAttributeValues::Uint16x4(vec)
    }
}

impl From<Vec<[u8; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[u8; 4]>) -> Self {
        VertexAttributeValues::Unorm8x4(vec)
    }
}

impl From<Vec<Vec2>> for VertexAttributeValues {
    fn from(vec: Vec<Vec2>) -> Self {
        VertexAttributeValues::Float32x2(convert_vec_of_vec!(vec, f32, 2))
    }
}

impl From<Vec<IVec2>> for VertexAttributeValues {
    fn from(vec: Vec<IVec2>) -> Self {
        VertexAttributeValues::Sint32x2(convert_vec_of_vec!(vec, i32, 2))
    }
}

impl From<Vec<UVec2>> for VertexAttributeValues {
    fn from(vec: Vec<UVec2>) -> Self {
        VertexAttributeValues::Uint32x2(convert_vec_of_vec!(vec, u32, 2))
    }
}

impl From<Vec<Vec3>> for VertexAttributeValues {
    fn from(vec: Vec<Vec3>) -> Self {
        VertexAttributeValues::Float32x3(convert_vec_of_vec!(vec, f32, 3))
    }
}

impl From<Vec<IVec3>> for VertexAttributeValues {
    fn from(vec: Vec<IVec3>) -> Self {
        VertexAttributeValues::Sint32x3(convert_vec_of_vec!(vec, i32, 3))
    }
}

impl From<Vec<UVec3>> for VertexAttributeValues {
    fn from(vec: Vec<UVec3>) -> Self {
        VertexAttributeValues::Uint32x3(convert_vec_of_vec!(vec, u32, 3))
    }
}

impl From<Vec<Vec4>> for VertexAttributeValues {
    fn from(vec: Vec<Vec4>) -> Self {
        VertexAttributeValues::Float32x4(convert_vec_of_vec!(vec, f32, 4))
    }
}

impl From<Vec<IVec4>> for VertexAttributeValues {
    fn from(vec: Vec<IVec4>) -> Self {
        VertexAttributeValues::Sint32x4(convert_vec_of_vec!(vec, i32, 4))
    }
}

impl From<Vec<UVec4>> for VertexAttributeValues {
    fn from(vec: Vec<UVec4>) -> Self {
        VertexAttributeValues::Uint32x4(convert_vec_of_vec!(vec, u32, 4))
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u8; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint8x4(value) => Ok(value),
            VertexAttributeValues::Unorm8x4(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i8; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Sint8x4(value) => Ok(value),
            VertexAttributeValues::Snorm8x4(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u8; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint8x2(value) => Ok(value),
            VertexAttributeValues::Unorm8x2(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i8; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Sint8x2(value) => Ok(value),
            VertexAttributeValues::Snorm8x2(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i16; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Sint16x4(value) => Ok(value),
            VertexAttributeValues::Snorm16x4(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u16; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint16x4(value) => Ok(value),
            VertexAttributeValues::Unorm16x4(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u16; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint16x2(value) => Ok(value),
            VertexAttributeValues::Unorm16x2(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i16; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Sint16x2(value) => Ok(value),
            VertexAttributeValues::Snorm16x2(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u32; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint32x4(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i32; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Sint32x4(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[f32; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Float32x4(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u32; 3]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint32x3(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i32; 3]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Sint32x3(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[f32; 3]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Float32x3(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u32; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint32x2(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i32; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Sint32x2(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[f32; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Float32x2(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<u32> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint32(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<i32> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Sint32(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<f32> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Float32(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use bevy_math::{IVec2, IVec3, IVec4, UVec2, UVec3, UVec4, Vec2, Vec3, Vec4};

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

        let buffer = vec![Vec2::new(0., PI); 2];
        let values = VertexAttributeValues::from(buffer);
        let result_into: Vec<[f32; 2]> = values.try_into().unwrap();
        assert_eq!(result_into[1][1], PI);
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

        let buffer = vec![IVec2::new(0, -1); 2];
        let values = VertexAttributeValues::from(buffer);
        let result_into: Vec<[i32; 2]> = values.try_into().unwrap();
        assert_eq!(result_into[1][1], -1);
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

        let buffer = vec![UVec2::new(0, u32::MAX); 2];
        let values = VertexAttributeValues::from(buffer);
        let result_into: Vec<[u32; 2]> = values.try_into().unwrap();
        assert_eq!(result_into[1][1], u32::MAX);
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

        let buffer = vec![Vec3::new(0., 0., PI); 2];
        let values = VertexAttributeValues::from(buffer);
        let result_into: Vec<[f32; 3]> = values.try_into().unwrap();
        assert_eq!(result_into[1][2], PI);
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

        let buffer = vec![IVec3::new(0, 0, -1); 2];
        let values = VertexAttributeValues::from(buffer);
        let result_into: Vec<[i32; 3]> = values.try_into().unwrap();
        assert_eq!(result_into[1][2], -1);
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

        let buffer = vec![UVec3::new(0, 0, u32::MAX); 2];
        let values = VertexAttributeValues::from(buffer);
        let result_into: Vec<[u32; 3]> = values.try_into().unwrap();
        assert_eq!(result_into[1][2], u32::MAX);
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

        let buffer = vec![Vec4::new(0., 0., 0., PI); 2];
        let values = VertexAttributeValues::from(buffer);
        let result_into: Vec<[f32; 4]> = values.try_into().unwrap();
        assert_eq!(result_into[1][3], PI);
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

        let buffer = vec![IVec4::new(0, 0, 0, -1); 2];
        let values = VertexAttributeValues::from(buffer);
        let result_into: Vec<[i32; 4]> = values.try_into().unwrap();
        assert_eq!(result_into[1][3], -1);
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

        let buffer = vec![UVec4::new(0, 0, 0, u32::MAX); 2];
        let values = VertexAttributeValues::from(buffer);
        let result_into: Vec<[u32; 4]> = values.try_into().unwrap();
        assert_eq!(result_into[1][3], u32::MAX);
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
            format!("{}", error),
            "cannot convert VertexAttributeValues::Uint32x4 to alloc::vec::Vec<u32>"
        );
        assert_eq!(format!("{:?}", error),
               "FromVertexAttributeError { from: Uint32x4([[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]), variant: \"Uint32x4\", into: \"alloc::vec::Vec<u32>\" }");
    }
}
