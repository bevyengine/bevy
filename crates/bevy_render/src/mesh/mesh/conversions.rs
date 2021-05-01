//! These implementations allow you to
//! convert std::vec::Vec<T> to VertexAttributeValues::T and back.
//!
//! # Examples
//!
//! ```rust
//! use bevy_render::mesh::VertexAttributeValues;
//! use std::convert::{ TryInto, TryFrom };
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
use bevy_utils::EnumVariantMeta;
use std::convert::TryFrom;
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

impl From<Vec<f32>> for VertexAttributeValues {
    fn from(vec: Vec<f32>) -> Self {
        VertexAttributeValues::Float(vec)
    }
}

impl From<Vec<i32>> for VertexAttributeValues {
    fn from(vec: Vec<i32>) -> Self {
        VertexAttributeValues::Int(vec)
    }
}

impl From<Vec<u32>> for VertexAttributeValues {
    fn from(vec: Vec<u32>) -> Self {
        VertexAttributeValues::Uint(vec)
    }
}

impl From<Vec<[f32; 2]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 2]>) -> Self {
        VertexAttributeValues::Float2(vec)
    }
}

impl From<Vec<[i32; 2]>> for VertexAttributeValues {
    fn from(vec: Vec<[i32; 2]>) -> Self {
        VertexAttributeValues::Int2(vec)
    }
}

impl From<Vec<[u32; 2]>> for VertexAttributeValues {
    fn from(vec: Vec<[u32; 2]>) -> Self {
        VertexAttributeValues::Uint2(vec)
    }
}

impl From<Vec<[f32; 3]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 3]>) -> Self {
        VertexAttributeValues::Float3(vec)
    }
}

impl From<Vec<[i32; 3]>> for VertexAttributeValues {
    fn from(vec: Vec<[i32; 3]>) -> Self {
        VertexAttributeValues::Int3(vec)
    }
}

impl From<Vec<[u32; 3]>> for VertexAttributeValues {
    fn from(vec: Vec<[u32; 3]>) -> Self {
        VertexAttributeValues::Uint3(vec)
    }
}

impl From<Vec<[f32; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[f32; 4]>) -> Self {
        VertexAttributeValues::Float4(vec)
    }
}

impl From<Vec<[i32; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[i32; 4]>) -> Self {
        VertexAttributeValues::Int4(vec)
    }
}

impl From<Vec<[u32; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[u32; 4]>) -> Self {
        VertexAttributeValues::Uint4(vec)
    }
}

impl From<Vec<[u8; 4]>> for VertexAttributeValues {
    fn from(vec: Vec<[u8; 4]>) -> Self {
        VertexAttributeValues::Uchar4Norm(vec)
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u8; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uchar4(value) => Ok(value),
            VertexAttributeValues::Uchar4Norm(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i8; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Char4(value) => Ok(value),
            VertexAttributeValues::Char4Norm(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u8; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uchar2(value) => Ok(value),
            VertexAttributeValues::Uchar2Norm(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i8; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Char2(value) => Ok(value),
            VertexAttributeValues::Char2Norm(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i16; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Short4(value) => Ok(value),
            VertexAttributeValues::Short4Norm(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u16; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Ushort4(value) => Ok(value),
            VertexAttributeValues::Ushort4Norm(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u16; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Ushort2(value) => Ok(value),
            VertexAttributeValues::Ushort2Norm(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i16; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Short2(value) => Ok(value),
            VertexAttributeValues::Short2Norm(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u32; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint4(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i32; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Int4(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[f32; 4]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Float4(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u32; 3]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint3(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i32; 3]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Int3(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[f32; 3]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Float3(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[u32; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint2(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[i32; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Int2(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<[f32; 2]> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Float2(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<u32> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Uint(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<i32> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Int(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

impl TryFrom<VertexAttributeValues> for Vec<f32> {
    type Error = FromVertexAttributeError;

    fn try_from(value: VertexAttributeValues) -> Result<Self, Self::Error> {
        match value {
            VertexAttributeValues::Float(value) => Ok(value),
            _ => Err(FromVertexAttributeError::new::<Self>(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VertexAttributeValues;
    use std::convert::{TryFrom, TryInto};
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
            "cannot convert VertexAttributeValues::Uint4 to alloc::vec::Vec<u32>"
        );
        assert_eq!(format!("{:?}", error),
               "FromVertexAttributeError { from: Uint4([[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]), variant: \"Uint4\", into: \"alloc::vec::Vec<u32>\" }");
    }
}
