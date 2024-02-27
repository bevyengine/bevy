use bevy_reflect::Reflect;

/// Struct defining a [`Sprite`](crate::Sprite) border with padding values
#[derive(Default, Copy, Clone, PartialEq, Debug, Reflect)]
pub struct BorderRect {
    /// Pixel padding to the left
    pub left: f32,
    /// Pixel padding to the right
    pub right: f32,
    /// Pixel padding to the top
    pub top: f32,
    /// Pixel padding to the bottom
    pub bottom: f32,
}

impl BorderRect {
    /// Creates a new border as a square, with identical pixel padding values on every direction
    #[must_use]
    #[inline]
    pub const fn square(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    /// Creates a new border as a rectangle, with:
    /// - `horizontal` for left and right pixel padding
    /// - `vertical` for top and bottom pixel padding
    #[must_use]
    #[inline]
    pub const fn rectangle(horizontal: f32, vertical: f32) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }
}

impl From<f32> for BorderRect {
    fn from(v: f32) -> Self {
        Self::square(v)
    }
}

impl From<[f32; 4]> for BorderRect {
    fn from([left, right, top, bottom]: [f32; 4]) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }
}
