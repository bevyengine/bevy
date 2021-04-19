//! This implementations allows you
//! convert std::vec::Vec<T> to VertexAttributeValues::T and back.
//!
//! # Examples
//!
//! ```rust
//! use std::convert::TryInto;
//! use bevy_render::mesh::VertexAttributeValues;
//!
//! // creating vector of values
//! let before = vec![[0_u32; 4]; 10];
//! let values = VertexAttributeValues::from(before.clone());
//! let after: Vec<[u32; 4]> = values.try_into().unwrap();
//!
//! assert_eq!(before, after);
//! ```

use crate::mesh::VertexAttributeValues;
use std::convert::TryInto;

const CANT_CONVERT: &str = "can't convert to ";

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

impl TryInto<Vec<[u8; 4]>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<[u8; 4]>, Self::Error> {
        match self {
            VertexAttributeValues::Uchar4Norm(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<[u8; 4]>")
        }
    }
}

impl TryInto<Vec<[u32; 4]>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<[u32; 4]>, Self::Error> {
        match self {
            VertexAttributeValues::Uint4(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<[u32; 4]>")
        }
    }
}

impl TryInto<Vec<[i32; 4]>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<[i32; 4]>, Self::Error> {
        match self {
            VertexAttributeValues::Int4(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<[i32; 4]>")
        }
    }
}

impl TryInto<Vec<[f32; 4]>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<[f32; 4]>, Self::Error> {
        match self {
            VertexAttributeValues::Float4(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<[f32; 4]>")
        }
    }
}

impl TryInto<Vec<[u32; 3]>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<[u32; 3]>, Self::Error> {
        match self {
            VertexAttributeValues::Uint3(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<[u32; 3]>")
        }
    }
}

impl TryInto<Vec<[i32; 3]>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<[i32; 3]>, Self::Error> {
        match self {
            VertexAttributeValues::Int3(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<[i32; 3]>")
        }
    }
}

impl TryInto<Vec<[f32; 3]>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<[f32; 3]>, Self::Error> {
        match self {
            VertexAttributeValues::Float3(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<[f32; 3]>")
        }
    }
}

impl TryInto<Vec<[u32; 2]>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<[u32; 2]>, Self::Error> {
        match self {
            VertexAttributeValues::Uint2(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<[u32; 2]>")
        }
    }
}

impl TryInto<Vec<[i32; 2]>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<[i32; 2]>, Self::Error> {
        match self {
            VertexAttributeValues::Int2(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<[i32; 2]>")
        }
    }
}

impl TryInto<Vec<[f32; 2]>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<[f32; 2]>, Self::Error> {
        match self {
            VertexAttributeValues::Float2(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<[f32; 2]>")
        }
    }
}

impl TryInto<Vec<u32>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<u32>, Self::Error> {
        match self {
            VertexAttributeValues::Uint(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<u32>")
        }
    }
}

impl TryInto<Vec<i32>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<i32>, Self::Error> {
        match self {
            VertexAttributeValues::Int(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<i32>")
        }
    }
}

impl TryInto<Vec<f32>> for VertexAttributeValues {
    type Error = String;

    fn try_into(self) -> Result<Vec<f32>, Self::Error> {
        match self {
            VertexAttributeValues::Float(value) => Ok(value),
            _ => Err(CANT_CONVERT.to_string() + "Vec<f32>")
        }
    }
}

#[test]
fn f32()
{
    let buffer = vec![0.0; 10];
    let result: Vec<f32> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn i32()
{
    let buffer = vec![0; 10];
    let result: Vec<i32> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn u32()
{
    let buffer = vec![0_u32; 10];
    let result: Vec<u32> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<f32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn f32_2()
{
    let buffer = vec![[0.0; 2]; 10];
    let result: Vec<[f32; 2]> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn i32_2()
{
    let buffer = vec![[0; 2]; 10];
    let result: Vec<[i32; 2]> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn u32_2()
{
    let buffer = vec![[0_u32; 2]; 10];
    let result: Vec<[u32; 2]> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn f32_3()
{
    let buffer = vec![[0.0; 3]; 10];
    let result: Vec<[f32; 3]> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn i32_3()
{
    let buffer = vec![[0; 3]; 10];
    let result: Vec<[i32; 3]> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn u32_3()
{
    let buffer = vec![[0_u32; 3]; 10];
    let result: Vec<[u32; 3]> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn f32_4()
{
    let buffer = vec![[0.0; 4]; 10];
    let result: Vec<[f32; 4]> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn i32_4()
{
    let buffer = vec![[0; 4]; 10];
    let result: Vec<[i32; 4]> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}

#[test]
fn u32_4()
{
    let buffer = vec![[0_u32; 4]; 10];
    let result: Vec<[u32; 4]> = VertexAttributeValues::from(buffer.clone()).try_into().unwrap();
    let error: Result<Vec<u32>, _> = VertexAttributeValues::from(buffer.clone()).try_into();
    assert_eq!(buffer, result);
    assert!(error.is_err());
}
