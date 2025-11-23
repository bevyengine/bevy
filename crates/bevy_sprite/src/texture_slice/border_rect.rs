use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// Defines the extents of the border of a rectangle.
///
/// This struct is used to represent thickness or offsets from the four edges
/// of a rectangle, with values increasing inwards.
#[derive(Default, Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq, Default)]
pub struct BorderRect {
    /// Extents of the border from the minimum corner of the rectangle
    pub min: Vec2,
    /// Extents of the border from the maximum corner of the rectangle
    pub max: Vec2,
}

impl BorderRect {
    /// An empty border with zero thickness along each edge
    pub const ZERO: Self = Self::all(0.);

    /// Creates a border with the same `extent` along each edge
    #[must_use]
    #[inline]
    pub const fn all(extent: f32) -> Self {
        Self {
            min: Vec2::splat(extent),
            max: Vec2::splat(extent),
        }
    }

    /// Creates a new border with the `min.x` and `max.x` extents equal to `horizontal`, and the `min.y` and `max.y` extents equal to `vertical`.
    #[must_use]
    #[inline]
    pub const fn axes(horizontal: f32, vertical: f32) -> Self {
        let extents = Vec2::new(horizontal, vertical);
        Self {
            min: extents,
            max: extents,
        }
    }
}

impl From<f32> for BorderRect {
    fn from(extent: f32) -> Self {
        Self::all(extent)
    }
}

impl From<[f32; 4]> for BorderRect {
    fn from([min_x, max_x, min_y, max_y]: [f32; 4]) -> Self {
        Self {
            min: Vec2::new(min_x, min_y),
            max: Vec2::new(max_x, max_y),
        }
    }
}

impl core::ops::Add for BorderRect {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.min += rhs.min;
        self.max += rhs.max;
        self
    }
}

impl core::ops::Sub for BorderRect {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self.min -= rhs.min;
        self.max -= rhs.max;
        self
    }
}

impl core::ops::Mul<f32> for BorderRect {
    type Output = Self;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self.min *= rhs;
        self.max *= rhs;
        self
    }
}

impl core::ops::Div<f32> for BorderRect {
    type Output = Self;

    fn div(mut self, rhs: f32) -> Self::Output {
        self.min /= rhs;
        self.max /= rhs;
        self
    }
}
