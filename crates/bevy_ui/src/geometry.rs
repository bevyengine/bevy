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
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Size<T: Reflect + PartialEq = f32> {
    /// The width of the 2-dimensional area.
    pub width: T,
    /// The height of the 2-dimensional area.
    pub height: T,
}

impl<T: Reflect + PartialEq> Size<T> {
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
    pub fn new(width: T, height: T) -> Self {
        Size { width, height }
    }
}

impl<T: Default + Reflect + PartialEq> Default for Size<T> {
    fn default() -> Self {
        Self {
            width: Default::default(),
            height: Default::default(),
        }
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
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct UiRect<T: Reflect + PartialEq> {
    /// The left value of the [`UiRect`].
    pub left: T,
    /// The right value of the [`UiRect`].
    pub right: T,
    /// The top value of the [`UiRect`].
    pub top: T,
    /// The bottom value of the [`UiRect`].
    pub bottom: T,
}

impl<T: Reflect + PartialEq> UiRect<T> {
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
    pub fn new(left: T, right: T, top: T, bottom: T) -> Self {
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
    pub fn all(value: T) -> Self
    where
        T: Clone,
    {
        UiRect {
            left: value.clone(),
            right: value.clone(),
            top: value.clone(),
            bottom: value,
        }
    }
}

impl<T: Default + Reflect + PartialEq> Default for UiRect<T> {
    fn default() -> Self {
        Self {
            left: Default::default(),
            right: Default::default(),
            top: Default::default(),
            bottom: Default::default(),
        }
    }
}

impl<T: Reflect + PartialEq> Add<Vec2> for Size<T>
where
    T: Add<f32, Output = T>,
{
    type Output = Size<T>;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            width: self.width + rhs.x,
            height: self.height + rhs.y,
        }
    }
}

impl<T: Reflect + PartialEq> AddAssign<Vec2> for Size<T>
where
    T: AddAssign<f32>,
{
    fn add_assign(&mut self, rhs: Vec2) {
        self.width += rhs.x;
        self.height += rhs.y;
    }
}

impl<T: Reflect + PartialEq> Sub<Vec2> for Size<T>
where
    T: Sub<f32, Output = T>,
{
    type Output = Size<T>;

    fn sub(self, rhs: Vec2) -> Self::Output {
        Self {
            width: self.width - rhs.x,
            height: self.height - rhs.y,
        }
    }
}

impl<T: Reflect + PartialEq> SubAssign<Vec2> for Size<T>
where
    T: SubAssign<f32>,
{
    fn sub_assign(&mut self, rhs: Vec2) {
        self.width -= rhs.x;
        self.height -= rhs.y;
    }
}

impl<T: Reflect + PartialEq> Mul<f32> for Size<T>
where
    T: Mul<f32, Output = T>,
{
    type Output = Size<T>;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::Output {
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}

impl<T: Reflect + PartialEq> MulAssign<f32> for Size<T>
where
    T: MulAssign<f32>,
{
    fn mul_assign(&mut self, rhs: f32) {
        self.width *= rhs;
        self.height *= rhs;
    }
}

impl<T: Reflect + PartialEq> Div<f32> for Size<T>
where
    T: Div<f32, Output = T>,
{
    type Output = Size<T>;

    fn div(self, rhs: f32) -> Self::Output {
        Self::Output {
            width: self.width / rhs,
            height: self.height / rhs,
        }
    }
}

impl<T: Reflect + PartialEq> DivAssign<f32> for Size<T>
where
    T: DivAssign<f32>,
{
    fn div_assign(&mut self, rhs: f32) {
        self.width /= rhs;
        self.height /= rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_ops() {
        type SizeF = Size<f32>;

        assert_eq!(
            SizeF::new(10., 10.) + Vec2::new(10., 10.),
            SizeF::new(20., 20.)
        );
        assert_eq!(
            SizeF::new(20., 20.) - Vec2::new(10., 10.),
            SizeF::new(10., 10.)
        );
        assert_eq!(SizeF::new(10., 10.) * 2., SizeF::new(20., 20.));
        assert_eq!(SizeF::new(20., 20.) / 2., SizeF::new(10., 10.));

        let mut size = SizeF::new(10., 10.);

        size += Vec2::new(10., 10.);

        assert_eq!(size, SizeF::new(20., 20.));
    }
}
