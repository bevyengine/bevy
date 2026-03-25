use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// Defines border insets that shrink a rectangle from its minimum and maximum corners.
///
/// This struct is used to represent thickness or offsets from the four edges
/// of a rectangle, with values increasing inwards.
#[derive(Default, Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(Clone, PartialEq, Default)]
pub struct BorderRect {
    /// Inset applied to the rectangle’s minimum corner
    pub min_inset: Vec2,
    /// Inset applied to the rectangle’s maximum corner
    pub max_inset: Vec2,
}

impl BorderRect {
    /// An empty border with zero thickness along each edge
    pub const ZERO: Self = Self::all(0.);

    /// Creates a border with the same `inset` along each edge
    #[must_use]
    #[inline]
    pub const fn all(inset: f32) -> Self {
        Self {
            min_inset: Vec2::splat(inset),
            max_inset: Vec2::splat(inset),
        }
    }

    /// Creates a new border with the `min.x` and `max.x` insets equal to `horizontal`, and the `min.y` and `max.y` insets equal to `vertical`.
    #[must_use]
    #[inline]
    pub const fn axes(horizontal: f32, vertical: f32) -> Self {
        let insets = Vec2::new(horizontal, vertical);
        Self {
            min_inset: insets,
            max_inset: insets,
        }
    }
}

impl From<f32> for BorderRect {
    fn from(inset: f32) -> Self {
        Self::all(inset)
    }
}

impl From<[f32; 4]> for BorderRect {
    fn from([min_x, max_x, min_y, max_y]: [f32; 4]) -> Self {
        Self {
            min_inset: Vec2::new(min_x, min_y),
            max_inset: Vec2::new(max_x, max_y),
        }
    }
}

impl core::ops::Add for BorderRect {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.min_inset += rhs.min_inset;
        self.max_inset += rhs.max_inset;
        self
    }
}

impl core::ops::Sub for BorderRect {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self.min_inset -= rhs.min_inset;
        self.max_inset -= rhs.max_inset;
        self
    }
}

impl core::ops::Mul<f32> for BorderRect {
    type Output = Self;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self.min_inset *= rhs;
        self.max_inset *= rhs;
        self
    }
}

impl core::ops::Div<f32> for BorderRect {
    type Output = Self;

    fn div(mut self, rhs: f32) -> Self::Output {
        self.min_inset /= rhs;
        self.max_inset /= rhs;
        self
    }
}
