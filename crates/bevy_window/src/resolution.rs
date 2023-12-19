//! Provides [`PhysicalSize`] and [`LogicalSize`] structs to help resolution calulations.

use std::ops::{Deref, DerefMut};

use bevy_math::{UVec2, Vec2};
use bevy_reflect::prelude::{Reflect, ReflectDefault};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A width and height in physical pixels.
#[derive(Reflect, Default, Debug, Copy, Clone, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Default)]
pub struct PhysicalSize {
    /// The extents in physical pixels.
    pub extents: UVec2,
}

impl PhysicalSize {
    /// Create a new [`PhysicalSize`] from a width and height in physical units.
    #[inline]
    pub fn new(physical_width: u32, physical_height: u32) -> Self {
        Self {
            extents: UVec2::new(physical_width, physical_height),
        }
    }

    /// Converts [`PhysicalSize`] to [`LogicalSize`] using some scale factor.
    #[inline]
    pub fn to_logical(&self, scale_factor: f32) -> LogicalSize {
        LogicalSize::new(self.x as f32 / scale_factor, self.y as f32 / scale_factor)
    }
}

impl Deref for PhysicalSize {
    type Target = UVec2;

    fn deref(&self) -> &Self::Target {
        &self.extents
    }
}

impl DerefMut for PhysicalSize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.extents
    }
}

impl From<(u32, u32)> for PhysicalSize {
    fn from(value: (u32, u32)) -> Self {
        PhysicalSize::new(value.0, value.1)
    }
}

/// A width and height in logical pixels.
#[derive(Reflect, Default, Debug, Copy, Clone, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[reflect(Debug, PartialEq, Default)]
pub struct LogicalSize {
    /// The extents in logical pixels.
    pub extents: Vec2,
}

impl Deref for LogicalSize {
    type Target = Vec2;

    fn deref(&self) -> &Self::Target {
        &self.extents
    }
}

impl DerefMut for LogicalSize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.extents
    }
}

impl LogicalSize {
    /// Create a new [`LogicalSize`] from a width and height in logical units.
    #[inline]
    pub fn new(logical_width: f32, logical_height: f32) -> Self {
        Self {
            extents: Vec2::new(logical_width, logical_height),
        }
    }

    /// Converts [`LogicalSize`] to [`PhysicalSize`] using some scale factor.
    #[inline]
    pub fn to_physical(&self, scale_factor: f32) -> PhysicalSize {
        PhysicalSize::new(
            (self.x * scale_factor) as u32,
            (self.y * scale_factor) as u32,
        )
    }
}

impl From<(f32, f32)> for LogicalSize {
    fn from(value: (f32, f32)) -> Self {
        LogicalSize::new(value.0, value.1)
    }
}
