use crate::Val;
use bevy_reflect::Reflect;
use std::ops::{Div, DivAssign, Mul, MulAssign};

/// A type which is commonly used to define margins, paddings and borders.
///
/// # Examples

///
/// ## Margin
///
/// A margin is used to create space around UI elements, outside of any defined borders.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let margin = UiRect::all(Val::Auto); // Centers the UI element
/// ```
///
/// ## Padding
///
/// A padding is used to create space around UI elements, inside of any defined borders.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let padding = UiRect {
///     left: Val::Px(10.0),
///     right: Val::Px(20.0),
///     top: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
/// ```
///
/// ## Borders
///
/// A border is used to define the width of the border of a UI element.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let border = UiRect {
///     left: Val::Px(10.0),
///     right: Val::Px(20.0),
///     top: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
/// ```
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct UiRect {
    /// The value corresponding to the left side of the UI rect.
    pub left: Val,
    /// The value corresponding to the right side of the UI rect.
    pub right: Val,
    /// The value corresponding to the top side of the UI rect.
    pub top: Val,
    /// The value corresponding to the bottom side of the UI rect.
    pub bottom: Val,
}

impl UiRect {
    pub const DEFAULT: Self = Self {
        left: Val::Px(0.),
        right: Val::Px(0.),
        top: Val::Px(0.),
        bottom: Val::Px(0.),
    };

    /// Creates a new [`UiRect`] from the values specified.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::new(
    ///     Val::Px(10.0),
    ///     Val::Px(20.0),
    ///     Val::Px(30.0),
    ///     Val::Px(40.0),
    /// );
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(20.0));
    /// assert_eq!(ui_rect.top, Val::Px(30.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(40.0));
    /// ```
    pub const fn new(left: Val, right: Val, top: Val, bottom: Val) -> Self {
        UiRect {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Creates a new [`UiRect`] where all sides have the same value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub const fn all(value: Val) -> Self {
        UiRect {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    /// Creates a new [`UiRect`] where `left` and `right` take the given value,
    /// and `top` and `bottom` set to zero `Val::Px(0.)`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::horizontal(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Px(0.));
    /// assert_eq!(ui_rect.bottom, Val::Px(0.));
    /// ```
    pub fn horizontal(value: Val) -> Self {
        UiRect {
            left: value,
            right: value,
            ..Default::default()
        }
    }

    /// Creates a new [`UiRect`] where `top` and `bottom` take the given value,
    /// and `left` and `right` are set to `Val::Px(0.)`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::vertical(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(0.));
    /// assert_eq!(ui_rect.right, Val::Px(0.));
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub fn vertical(value: Val) -> Self {
        UiRect {
            top: value,
            bottom: value,
            ..Default::default()
        }
    }

    /// Creates a new [`UiRect`] where `left` takes the given value, and
    /// the other fields are set to `Val::Px(0.)`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::left(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(0.));
    /// assert_eq!(ui_rect.top, Val::Px(0.));
    /// assert_eq!(ui_rect.bottom, Val::Px(0.));
    /// ```
    pub fn left(value: Val) -> Self {
        UiRect {
            left: value,
            ..Default::default()
        }
    }

    /// Creates a new [`UiRect`] where `right` takes the given value,
    /// and the other fields are set to `Val::Px(0.)`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::right(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(0.));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Px(0.));
    /// assert_eq!(ui_rect.bottom, Val::Px(0.));
    /// ```
    pub fn right(value: Val) -> Self {
        UiRect {
            right: value,
            ..Default::default()
        }
    }

    /// Creates a new [`UiRect`] where `top` takes the given value,
    /// and the other fields are set to `Val::Px(0.)`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::top(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(0.));
    /// assert_eq!(ui_rect.right, Val::Px(0.));
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(0.));
    /// ```
    pub fn top(value: Val) -> Self {
        UiRect {
            top: value,
            ..Default::default()
        }
    }

    /// Creates a new [`UiRect`] where `bottom` takes the given value,
    /// and the other fields are set to `Val::Px(0.)`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::bottom(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(0.));
    /// assert_eq!(ui_rect.right, Val::Px(0.));
    /// assert_eq!(ui_rect.top, Val::Px(0.));
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub fn bottom(value: Val) -> Self {
        UiRect {
            bottom: value,
            ..Default::default()
        }
    }
}

impl Default for UiRect {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A 2-dimensional area defined by a width and height.
///
/// It is commonly used to define the size of a text or UI element.
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Size {
    /// The width of the 2-dimensional area.
    pub width: Val,
    /// The height of the 2-dimensional area.
    pub height: Val,
}

impl Size {
    pub const DEFAULT: Self = Self::AUTO;

    /// Creates a new [`Size`] from a width and a height.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Size, Val};
    /// #
    /// let size = Size::new(Val::Px(100.0), Val::Px(200.0));
    ///
    /// assert_eq!(size.width, Val::Px(100.0));
    /// assert_eq!(size.height, Val::Px(200.0));
    /// ```
    pub const fn new(width: Val, height: Val) -> Self {
        Size { width, height }
    }

    /// Creates a new [`Size`] where both sides take the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Size, Val};
    /// #
    /// let size = Size::all(Val::Px(10.));
    ///
    /// assert_eq!(size.width, Val::Px(10.0));
    /// assert_eq!(size.height, Val::Px(10.0));
    /// ```
    pub const fn all(value: Val) -> Self {
        Self {
            width: value,
            height: value,
        }
    }

    /// Creates a new [`Size`] where `width` takes the given value,
    /// and `height` is set to [`Val::Auto`].

    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Size, Val};
    /// #
    /// let size = Size::width(Val::Px(10.));
    ///
    /// assert_eq!(size.width, Val::Px(10.0));
    /// assert_eq!(size.height, Val::Auto);
    /// ```
    pub const fn width(width: Val) -> Self {
        Self {
            width,
            height: Val::Auto,
        }
    }

    /// Creates a new [`Size`] where `height` takes the given value,
    /// and `width` is set to [`Val::Auto`].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Size, Val};
    /// #
    /// let size = Size::height(Val::Px(10.));
    ///
    /// assert_eq!(size.width, Val::Auto);
    /// assert_eq!(size.height, Val::Px(10.));
    /// ```
    pub const fn height(height: Val) -> Self {
        Self {
            width: Val::Auto,
            height,
        }
    }

    /// Creates a Size where both values are [`Val::Auto`].
    pub const AUTO: Self = Self::all(Val::Auto);
}

impl Default for Size {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<(Val, Val)> for Size {
    fn from(vals: (Val, Val)) -> Self {
        Self {
            width: vals.0,
            height: vals.1,
        }
    }
}

impl Mul<f32> for Size {
    type Output = Size;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::Output {
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}

impl MulAssign<f32> for Size {
    fn mul_assign(&mut self, rhs: f32) {
        self.width *= rhs;
        self.height *= rhs;
    }
}

impl Div<f32> for Size {
    type Output = Size;

    fn div(self, rhs: f32) -> Self::Output {
        Self::Output {
            width: self.width / rhs,
            height: self.height / rhs,
        }
    }
}

impl DivAssign<f32> for Size {
    fn div_assign(&mut self, rhs: f32) {
        self.width /= rhs;
        self.height /= rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uirect_default_equals_const_default() {
        assert_eq!(
            UiRect::default(),
            UiRect {
                left: Val::Px(0.),
                right: Val::Px(0.),
                top: Val::Px(0.),
                bottom: Val::Px(0.)
            }
        );
        assert_eq!(UiRect::default(), UiRect::DEFAULT);
    }

    #[test]
    fn test_size_from() {
        let size: Size = (Val::Px(20.), Val::Px(30.)).into();

        assert_eq!(
            size,
            Size {
                width: Val::Px(20.),
                height: Val::Px(30.),
            }
        );
    }

    #[test]
    fn test_size_mul() {
        assert_eq!(Size::all(Val::Px(10.)) * 2., Size::all(Val::Px(20.)));

        let mut size = Size::all(Val::Px(10.));
        size *= 2.;
        assert_eq!(size, Size::all(Val::Px(20.)));
    }

    #[test]
    fn test_size_div() {
        assert_eq!(
            Size::new(Val::Px(20.), Val::Px(20.)) / 2.,
            Size::new(Val::Px(10.), Val::Px(10.))
        );

        let mut size = Size::new(Val::Px(20.), Val::Px(20.));
        size /= 2.;
        assert_eq!(size, Size::new(Val::Px(10.), Val::Px(10.)));
    }

    #[test]
    fn test_size_all() {
        let length = Val::Px(10.);

        assert_eq!(
            Size::all(length),
            Size {
                width: length,
                height: length
            }
        );
    }

    #[test]
    fn test_size_width() {
        let width = Val::Px(10.);

        assert_eq!(
            Size::width(width),
            Size {
                width,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_size_height() {
        let height = Val::Px(7.);

        assert_eq!(
            Size::height(height),
            Size {
                height,
                ..Default::default()
            }
        );
    }

    #[test]
    fn size_default_equals_const_default() {
        assert_eq!(
            Size::default(),
            Size {
                width: Val::Auto,
                height: Val::Auto
            }
        );
        assert_eq!(Size::default(), Size::DEFAULT);
    }
}
