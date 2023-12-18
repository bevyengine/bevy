//! Provides [`PhysicalSize`] and [`LogicalSize`] structs to help resolution calulations.

use std::ops::Div;

use bevy_math::{UVec2, Vec2};
use bevy_reflect::Reflect;

/// A [`PhysicalSize`] is the width and height using physical pixels.
#[derive(Reflect, Default, Debug, Copy, Clone, PartialEq)]
pub struct PhysicalSize {
    /// The width in physical pixels.
    pub x: u32,
    /// The height in physical pixels.
    pub y: u32,
}

impl PhysicalSize {
    /// Create a new [`PhysicalSize`] from a width and height in physical units.
    #[inline]
    pub fn new(physical_width: u32, physical_height: u32) -> Self {
        Self {
            x: physical_width,
            y: physical_height,
        }
    }

    /// Converts [`PhysicalSize`] to [`LogicalSize`] using some scale factor.
    #[inline]
    pub fn to_logical(&self, scale_factor: f32) -> LogicalSize {
        LogicalSize::new(self.x as f32 / scale_factor, self.y as f32 / scale_factor)
    }
}

impl From<PhysicalSize> for UVec2 {
    fn from(value: PhysicalSize) -> Self {
        UVec2::new(value.x, value.y)
    }
}

impl From<PhysicalSize> for Vec2 {
    fn from(value: PhysicalSize) -> Self {
        Vec2::new(value.x as f32, value.y as f32)
    }
}

/// A [`LogicalSize`] is the width and height using logical pixels
#[derive(Reflect, Default, Debug, Copy, Clone, PartialEq)]
pub struct LogicalSize {
    /// The width in logical pixels
    pub x: f32,
    /// The height in logical pixels
    pub y: f32,
}

impl LogicalSize {
    /// Create a new [`LogicalSize`] from a width and height in logical units.
    #[inline]
    pub fn new(logical_width: f32, logical_height: f32) -> Self {
        Self {
            x: logical_width,
            y: logical_height,
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

    /// Returns the minimum between either width or height.
    #[inline]
    pub fn min_element(self) -> f32 {
        self.x.min(self.y)
    }

    /// Returns the maximum between either width or height.
    #[inline]
    pub fn max_element(self) -> f32 {
        self.x.max(self.y)
    }
}

impl From<LogicalSize> for Vec2 {
    fn from(value: LogicalSize) -> Self {
        Vec2::new(value.x, value.y)
    }
}

impl Div<f32> for LogicalSize {
    type Output = LogicalSize;
    fn div(self, rhs: f32) -> Self::Output {
        Self {
            x: self.x.div(rhs),
            y: self.y.div(rhs),
        }
    }
}
