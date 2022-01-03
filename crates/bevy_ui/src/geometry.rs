use crate::Val;
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

/// A 2-dimensional area defined by a width and height.
///
/// It is commonly used to define the size of a text or UI element.
///
/// # Example
///
/// ```rust
/// # use bevy_ui::{Size, Val};
/// #
/// let size = Size {
///     width: Val::Px(100.0),
///     height: Val::Px(200.0),
/// };
///
/// assert_eq!(size.width, Val::Px(100.0));
/// assert_eq!(size.height, Val::Px(200.0));
/// ```
#[derive(Default, Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Size {
    /// The width of the 2-dimensional area.
    pub width: Val,
    /// The height of the 2-dimensional area.
    pub height: Val,
}

impl Size {
    /// Creates a new [`Size`] from a width and a height.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bevy_ui::{Size, Val};
    /// #
    /// let size = Size::new(Val::Px(100.0), Val::Px(200.0));
    ///
    /// assert_eq!(size.width, Val::Px(100.0));
    /// assert_eq!(size.height, Val::Px(200.0));
    /// ```
    pub fn new(width: Val, height: Val) -> Self {
        Size { width, height }
    }
}

impl Add<Vec2> for Size {
    type Output = Size;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            width: self.width + rhs.x,
            height: self.height + rhs.y,
        }
    }
}

impl AddAssign<Vec2> for Size {
    fn add_assign(&mut self, rhs: Vec2) {
        self.width += rhs.x;
        self.height += rhs.y;
    }
}

impl Sub<Vec2> for Size {
    type Output = Size;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Self {
            width: self.width - rhs.x,
            height: self.height - rhs.y,
        }
    }
}

impl SubAssign<Vec2> for Size {
    fn sub_assign(&mut self, rhs: Vec2) {
        self.width -= rhs.x;
        self.height -= rhs.y;
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

/// A type which is commonly used to define positions, margins, paddings and borders.
///
/// # Examples
///
/// ## Position
///
/// A position is used to determine where to place a UI element.
///
/// In this example we are creating a UI position. It has a left value of 100px and a top value of 50px.
/// If positioned absolutely this would correspond to a UI element which is positioned 100px to the right
/// from the left side of the window and 50px down from the top side of the window.
///
/// ```rust
/// # use bevy_ui::{UiRect, Val};
/// #
/// let position = UiRect {
///     left: Val::Px(100.0),
///     top: Val::Px(50.0),
///     ..Default::default()
/// };
///
/// assert_eq!(position.left, Val::Px(100.0));
/// assert_eq!(position.right, Val::Undefined);
/// assert_eq!(position.top, Val::Px(50.0));
/// assert_eq!(position.bottom, Val::Undefined);
/// ```
///
/// If you define opposite sides of the position, the size of the UI element will automatically be calculated
/// if not explicitly specified. This means that if you have a [`Size`] that uses [`Val::Undefined`] as a
/// width and height, the size would be determined by the window size and the values specified in the position.
///
/// In this example we are creating another UI position. It has a left value of 100px, a right value of 200px,
/// a top value of 300px and a bottom value of 400px. If positioned absolutely this would correspond to a
/// UI element that is positioned 100px to the right from the left side of the window and 300px down from
/// the top side of the window.
///
/// ```rust
/// # use bevy_ui::{UiRect, Val};
/// #
/// let position = UiRect {
///     left: Val::Px(100.0),
///     right: Val::Px(200.0),
///     top: Val::Px(300.0),
///     bottom: Val::Px(400.0),
/// };
///
/// assert_eq!(position.left, Val::Px(100.0));
/// assert_eq!(position.right, Val::Px(200.0));
/// assert_eq!(position.top, Val::Px(300.0));
/// assert_eq!(position.bottom, Val::Px(400.0));
/// ```
///
/// The size of the UI element would now be determined by the window size and the position values.
/// To determine the width of the UI element you have to take the width of the window and subtract it by the
/// left and right values of the position. To determine the height of the UI element you have to take the height
/// of the window and subtract it by the top and bottom values of the position.
///
/// If we had a window with a width and height of 1000px, the UI element would have a width of 700px and a height
/// of 300px.
///
/// ```rust
/// let window_size = 1000.0;
/// let left = 100.0;
/// let right = 200.0;
/// let top = 300.0;
/// let bottom = 400.0;
///
/// let ui_element_width = window_size - left - right;
/// let ui_element_height = window_size - top - bottom;
///
/// assert_eq!(ui_element_width, 700.0);
/// assert_eq!(ui_element_height, 300.0);
/// ```
///
/// If you define a [`Size`] and also all four sides of the position, the top and left values of the position
/// are used to determine where to place the UI element. The size will not be calculated using the bottom and
/// right values of the position because the size of the UI element is already explicitly specified.
///
/// ## Margin
///
/// A margin is used to create space around UI elements, outside of any defined borders.
///
/// In this example we are creating a UI margin. It has a left value of 10px, a top value of 20px,
/// a right value of 30px and a bottom value of 40px. This would add a margin of 10px on the left,
/// 20px on the top, 30px on the right and 40px on the bottom of the UI element.
///
/// ```rust
/// # use bevy_ui::{UiRect, Val};
/// #
/// let margin = UiRect {
///     left: Val::Px(10.0),
///     top: Val::Px(20.0),
///     right: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
///
/// assert_eq!(margin.left, Val::Px(10.0));
/// assert_eq!(margin.right, Val::Px(30.0));
/// assert_eq!(margin.top, Val::Px(20.0));
/// assert_eq!(margin.bottom, Val::Px(40.0));
/// ```
///
/// ## Padding
///
/// A padding is used to create space around UI elements, inside of any defined borders.
///
/// In this example we are creating a UI padding. It has a left value of 10px, a top value of 20px,
/// a right value of 30px and a bottom value of 40px. This would add a padding of 10px on the left,
/// 20px on the top, 30px on the right and 40px on the bottom of the UI element.
///
/// ```rust
/// # use bevy_ui::{UiRect, Val};
/// #
/// let padding = UiRect {
///     left: Val::Px(10.0),
///     top: Val::Px(20.0),
///     right: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
///
/// assert_eq!(padding.left, Val::Px(10.0));
/// assert_eq!(padding.right, Val::Px(30.0));
/// assert_eq!(padding.top, Val::Px(20.0));
/// assert_eq!(padding.bottom, Val::Px(40.0));
/// ```
///
/// ## Borders
///
/// A border is used to define the width of the border of a UI element.
///
/// In this example we are creating a UI border. It has a left value of 10px, a top value of 20px,
/// a right value of 30px and a bottom value of 40px. This would create a border around a UI element
/// that has a width of 10px on the left, 20px on the top, 30px on the right and 40px on the bottom.
///
/// ```rust
/// # use bevy_ui::{UiRect, Val};
/// #
/// let border = UiRect {
///     left: Val::Px(10.0),
///     top: Val::Px(20.0),
///     right: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
///
/// assert_eq!(border.left, Val::Px(10.0));
/// assert_eq!(border.right, Val::Px(30.0));
/// assert_eq!(border.top, Val::Px(20.0));
/// assert_eq!(border.bottom, Val::Px(40.0));
/// ```
#[derive(Default, Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct UiRect {
    /// The left value of the [`UiRect`].
    pub left: Val,
    /// The right value of the [`UiRect`].
    pub right: Val,
    /// The top value of the [`UiRect`].
    pub top: Val,
    /// The bottom value of the [`UiRect`].
    pub bottom: Val,
}

impl UiRect {
    /// Creates a new [`UiRect`] from the values specified.
    ///
    /// # Example
    ///
    /// ```rust
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
    pub fn new(left: Val, right: Val, top: Val, bottom: Val) -> Self {
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
    /// ```rust
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub fn all(value: Val) -> Self {
        UiRect {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::Vec2;

    #[test]
    fn size_ops() {
        assert_eq!(
            Size::new(Val::Px(10.), Val::Px(10.)) + Vec2::new(10., 10.),
            Size::new(Val::Px(20.), Val::Px(20.))
        );
        assert_eq!(
            Size::new(Val::Px(20.), Val::Px(20.)) - Vec2::new(10., 10.),
            Size::new(Val::Px(10.), Val::Px(10.))
        );
        assert_eq!(
            Size::new(Val::Px(10.), Val::Px(10.)) * 2.,
            Size::new(Val::Px(20.), Val::Px(20.))
        );
        assert_eq!(
            Size::new(Val::Px(20.), Val::Px(20.)) / 2.,
            Size::new(Val::Px(10.), Val::Px(10.))
        );

        let mut size = Size::new(Val::Px(10.), Val::Px(10.));

        size += Vec2::new(10., 10.);

        assert_eq!(size, Size::new(Val::Px(20.), Val::Px(20.)));
    }
}
