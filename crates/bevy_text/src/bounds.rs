use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// The maximum width and height of text. The text will wrap according to the specified size.
///
/// Characters out of the bounds after wrapping will be truncated. Text is aligned according to the
/// specified [`Justify`](crate::text::Justify).
///
/// Note: only characters that are completely out of the bounds will be truncated, so this is not a
/// reliable limit if it is necessary to contain the text strictly in the bounds. Currently this
/// component is mainly useful for text wrapping only.
#[derive(Component, Copy, Clone, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct TextBounds {
    /// The maximum width of text in logical pixels.
    /// If `None`, the width is unbounded.
    pub width: Option<f32>,
    /// The maximum height of text in logical pixels.
    /// If `None`, the height is unbounded.
    pub height: Option<f32>,
}

impl Default for TextBounds {
    #[inline]
    fn default() -> Self {
        Self::UNBOUNDED
    }
}

impl TextBounds {
    /// Unbounded text will not be truncated or wrapped.
    pub const UNBOUNDED: Self = Self {
        width: None,
        height: None,
    };

    /// Creates a new `TextBounds`, bounded with the specified width and height values.
    #[inline]
    pub const fn new(width: f32, height: f32) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
        }
    }

    /// Creates a new `TextBounds`, bounded with the specified width value and unbounded on height.
    #[inline]
    pub const fn new_horizontal(width: f32) -> Self {
        Self {
            width: Some(width),
            height: None,
        }
    }

    /// Creates a new `TextBounds`, bounded with the specified height value and unbounded on width.
    #[inline]
    pub const fn new_vertical(height: f32) -> Self {
        Self {
            width: None,
            height: Some(height),
        }
    }
}

impl From<Vec2> for TextBounds {
    #[inline]
    fn from(v: Vec2) -> Self {
        Self::new(v.x, v.y)
    }
}
