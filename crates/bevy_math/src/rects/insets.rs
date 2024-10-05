use bevy_reflect::Reflect;

/// Struct defining insets relative to a rect, commonly used for defining visual components.
#[derive(Default, Copy, Clone, PartialEq, Debug, Reflect)]
pub struct Insets {
    /// Left inset
    pub left: f32,
    /// Right inset
    pub right: f32,
    /// Top inset
    pub top: f32,
    /// Bottom inset
    pub bottom: f32,
}

impl Insets {
    /// An empty inset
    pub const ZERO: Self = Self::square(0.);

    /// Creates identical insets in every direction
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

    /// Creates insets with:
    /// - `horizontal` for left and right insets
    /// - `vertical` for top and bottom insets
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

impl From<f32> for Insets {
    fn from(v: f32) -> Self {
        Self::square(v)
    }
}

impl From<[f32; 4]> for Insets {
    fn from([left, right, top, bottom]: [f32; 4]) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }
}
