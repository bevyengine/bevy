use crate::Val;
use bevy_reflect::Reflect;
use std::ops::{Div, DivAssign, Mul, MulAssign};

/// A type which is commonly used to define insets, margins, paddings and borders.
///
/// # Examples
///
/// ## Inset
///
/// A inset is used to determine where to place a UI element.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// # use bevy_utils::default;
/// #
/// let inset = UiRect {
///     left: Val::Px(100.0),
///     top: Val::Px(50.0),
///     ..default()
/// };
/// ```
///
/// If you define opposite sides of the inset, the size of the UI element will automatically be calculated
/// if not explicitly specified. This means that if you have a [`Size`] that uses [`Val::Auto`](crate::Val::Auto)
/// as a width and height, the size would be determined by the window size and the values specified in the inset.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let inset = UiRect {
///     left: Val::Px(100.0),
///     right: Val::Px(200.0),
///     top: Val::Px(300.0),
///     bottom: Val::Px(400.0),
/// };
/// ```
///
/// To determine the width of the UI element you have to take the width of the window and subtract it by the
/// left and right values of the inset. To determine the height of the UI element you have to take the height
/// of the window and subtract it by the top and bottom values of the inset. If we had a window with a width
/// and height of 1000px, the UI element declared above would have a width of 700px and a height of 300px.
///
/// ```
/// // Size of the window
/// let window_width = 1000.0;
/// let window_height = 1000.0;
///
/// // Values of the inset
/// let left = 100.0;
/// let right = 200.0;
/// let top = 300.0;
/// let bottom = 400.0;
///
/// // Calculation to get the size of the UI element
/// let ui_element_width = window_width - left - right;
/// let ui_element_height = window_height - top - bottom;
///
/// assert_eq!(ui_element_width, 700.0);
/// assert_eq!(ui_element_height, 300.0);
/// ```
///
/// If you define a [`Size`] and also all four sides of the inset, the top and left values of the inset
/// are used to determine where to place the UI element. The size will not be calculated using the bottom and
/// right values of the inset because the size of the UI element is already explicitly specified.
///
/// ```
/// # use bevy_ui::{UiRect, Size, Val, Style};
/// # use bevy_utils::default;
/// #
/// let style = Style {
///     inset: Inset { // Defining all four sides
///         left: Val::Px(100.0),
///         right: Val::Px(200.0),
///         top: Val::Px(300.0),
///         bottom: Val::Px(400.0),
///     },
///     size: Size::new(Val::Percent(100.0), Val::Percent(50.0)), // but also explicitly specifying a size
///     ..default()
/// };
/// ```
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

    /// Creates a new [`UiRect`] where `left` and `right` take the given value.
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

    /// Creates a new [`UiRect`] where `top` and `bottom` take the given value.
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

    /// Creates a new [`UiRect`] where `left` takes the given value.
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

    /// Creates a new [`UiRect`] where `right` takes the given value.
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

    /// Creates a new [`UiRect`] where `top` takes the given value.
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

    /// Creates a new [`UiRect`] where `bottom` takes the given value.
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

/// A [`Inset`] is used to determine where to place a UI element.
///
/// ```
/// # use bevy_ui::{Inset, Val};
/// # use bevy_utils::default;
/// #
/// let inset = Inset {
///     left: Val::Px(100.0),
///     top: Val::Px(50.0),
///     ..default()
/// };
/// ```
///
/// If you define opposite sides of the inset, the size of the UI element will automatically be calculated
/// if not explicitly specified. This means that if you have a [`Size`] that uses [`Val::Auto`](crate::Val::Auto)
/// as a width and height, the size would be determined by the window size and the values specified in the inset.
///
/// ```
/// # use bevy_ui::{Inset, Val};
/// #
/// let inset = Inset {
///     left: Val::Px(100.0),
///     right: Val::Px(200.0),
///     top: Val::Px(300.0),
///     bottom: Val::Px(400.0),
/// };
/// ```
///
/// To determine the width of the UI element you have to take the width of the window and subtract it by the
/// left and right values of the inset. To determine the height of the UI element you have to take the height
/// of the window and subtract it by the top and bottom values of the inset. If we had a window with a width
/// and height of 1000px, the UI element declared above would have a width of 700px and a height of 300px.
///
/// ```
/// // Size of the window
/// let window_width = 1000.0;
/// let window_height = 1000.0;
///
/// // Values of the inset
/// let left = 100.0;
/// let right = 200.0;
/// let top = 300.0;
/// let bottom = 400.0;
///
/// // Calculation to get the size of the UI element
/// let ui_element_width = window_width - left - right;
/// let ui_element_height = window_height - top - bottom;
///
/// assert_eq!(ui_element_width, 700.0);
/// assert_eq!(ui_element_height, 300.0);
/// ```
///
/// If you define a [`Size`] and also all four sides of the inset, the top and left values of the inset
/// are used to determine where to place the UI element. The size will not be calculated using the bottom and
/// right values of the inset because the size of the UI element is already explicitly specified.
///
/// ```
/// # use bevy_ui::{Inset, Size, Val, Style};
/// # use bevy_utils::default;
/// #
/// let style = Style {
///     inset: Inset { // Defining all four sides
///         left: Val::Px(100.0),
///         right: Val::Px(200.0),
///         top: Val::Px(300.0),
///         bottom: Val::Px(400.0),
///     },
///     size: Size::new(Val::Percent(100.0), Val::Percent(50.0)), // but also explicitly specifying a size
///     ..default()
/// };
/// ```
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Inset {
    /// Distance inset from the left.
    pub left: Val,
    /// Distance inset from the right.
    pub right: Val,
    /// Distance inset from the top.
    pub top: Val,
    /// Distance inset from the bottom.
    pub bottom: Val,
}

impl Inset {
    pub const DEFAULT: Self = Self::all(Val::Auto);

    /// Creates a new [`Inset`] from the values specified.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Inset, Val};
    /// #
    /// let inset = Inset::new(
    ///     Val::Px(10.0),
    ///     Val::Px(20.0),
    ///     Val::Px(30.0),
    ///     Val::Px(40.0),
    /// );
    ///
    /// assert_eq!(inset.left, Val::Px(10.0));
    /// assert_eq!(inset.right, Val::Px(20.0));
    /// assert_eq!(inset.top, Val::Px(30.0));
    /// assert_eq!(inset.bottom, Val::Px(40.0));
    /// ```
    pub const fn new(left: Val, right: Val, top: Val, bottom: Val) -> Self {
        Inset {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Creates a new [`Inset`] where all sides have the same value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Inset, Val};
    /// #
    /// let inset = Inset::all(Val::Px(10.0));
    ///
    /// assert_eq!(inset.left, Val::Px(10.0));
    /// assert_eq!(inset.right, Val::Px(10.0));
    /// assert_eq!(inset.top, Val::Px(10.0));
    /// assert_eq!(inset.bottom, Val::Px(10.0));
    /// ```
    pub const fn all(value: Val) -> Self {
        Inset {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    /// Creates a new [`Inset`] where `left` takes the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Inset, Val};
    /// #
    /// let inset = Inset::left(Val::Px(10.0));
    ///
    /// assert_eq!(inset.left, Val::Px(10.0));
    /// assert_eq!(inset.right, Val::Auto);
    /// assert_eq!(inset.top, Val::Auto);
    /// assert_eq!(inset.bottom, Val::Auto);
    /// ```
    pub fn left(value: Val) -> Self {
        Inset {
            left: value,
            ..Default::default()
        }
    }

    /// Creates a new [`Inset`] where `right` takes the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Inset, Val};
    /// #
    /// let inset = Inset::right(Val::Px(10.0));
    ///
    /// assert_eq!(inset.left, Val::Auto);
    /// assert_eq!(inset.right, Val::Px(10.0));
    /// assert_eq!(inset.top, Val::Auto);
    /// assert_eq!(inset.bottom, Val::Auto);
    /// ```
    pub fn right(value: Val) -> Self {
        Inset {
            right: value,
            ..Default::default()
        }
    }

    /// Creates a new [`Inset`] where `top` takes the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Inset, Val};
    /// #
    /// let inset = Inset::top(Val::Px(10.0));
    ///
    /// assert_eq!(inset.left, Val::Auto);
    /// assert_eq!(inset.right, Val::Auto);
    /// assert_eq!(inset.top, Val::Px(10.0));
    /// assert_eq!(inset.bottom, Val::Auto);
    /// ```
    pub fn top(value: Val) -> Self {
        Inset {
            top: value,
            ..Default::default()
        }
    }

    /// Creates a new [`Inset`] where `bottom` takes the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Inset, Val};
    /// #
    /// let inset = Inset::bottom(Val::Px(10.0));
    ///
    /// assert_eq!(inset.left, Val::Auto);
    /// assert_eq!(inset.right, Val::Auto);
    /// assert_eq!(inset.top, Val::Auto);
    /// assert_eq!(inset.bottom, Val::Px(10.0));
    /// ```
    pub fn bottom(value: Val) -> Self {
        Inset {
            bottom: value,
            ..Default::default()
        }
    }

    /// Creates a new [`Inset`] where `left` and `right` take the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Inset, Val};
    /// #
    /// let inset = Inset::horizontal(Val::Px(10.0));
    ///
    /// assert_eq!(inset.left, Val::Px(10.0));
    /// assert_eq!(inset.right, Val::Px(10.0));
    /// assert_eq!(inset.top, Val::Auto);
    /// assert_eq!(inset.bottom, Val::Auto);
    /// ```
    pub fn horizontal(value: Val) -> Self {
        Inset {
            left: value,
            right: value,
            ..Default::default()
        }
    }

    /// Creates a new [`Inset`] where `top` and `bottom` take the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Inset, Val};
    /// #
    /// let inset = Inset::vertical(Val::Px(10.0));
    ///
    /// assert_eq!(inset.left, Val::Auto);
    /// assert_eq!(inset.right, Val::Auto);
    /// assert_eq!(inset.top, Val::Px(10.0));
    /// assert_eq!(inset.bottom, Val::Px(10.0));
    /// ```
    pub fn vertical(value: Val) -> Self {
        Inset {
            top: value,
            bottom: value,
            ..Default::default()
        }
    }
}

impl Default for Inset {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<Inset> for UiRect {
    fn from(inset: Inset) -> Self {
        Self {
            left: inset.left,
            right: inset.right,
            top: inset.top,
            bottom: inset.bottom,
        }
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
    pub const DEFAULT: Self = Self::all(Val::DEFAULT);

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
    pub const fn all(value: Val) -> Self {
        Self {
            width: value,
            height: value,
        }
    }

    /// Creates a new [`Size`] where `width` takes the given value.
    pub const fn width(width: Val) -> Self {
        Self {
            width,
            height: Val::DEFAULT,
        }
    }

    /// Creates a new [`Size`] where `height` takes the given value.
    pub const fn height(height: Val) -> Self {
        Self {
            width: Val::DEFAULT,
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
}
