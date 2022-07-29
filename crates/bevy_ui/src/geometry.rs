use bevy_math::Vec2;
use bevy_reflect::Reflect;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

/// A type which is commonly used to define positions, margins, paddings and borders.
///
/// # Examples
///
/// ## Position
///
/// A position is used to determine where to place a UI element.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// # use bevy_utils::default;
/// #
/// let position = UiRect {
///     left: Val::Px(100.0),
///     top: Val::Px(50.0),
///     ..default()
/// };
/// ```
///
/// If you define opposite sides of the position, the size of the UI element will automatically be calculated
/// if not explicitly specified. This means that if you have a [`Size`] that uses [`Val::Undefined`](crate::Val::Undefined)
/// as a width and height, the size would be determined by the window size and the values specified in the position.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let position = UiRect {
///     left: Val::Px(100.0),
///     right: Val::Px(200.0),
///     top: Val::Px(300.0),
///     bottom: Val::Px(400.0),
/// };
/// ```
///
/// To determine the width of the UI element you have to take the width of the window and subtract it by the
/// left and right values of the position. To determine the height of the UI element you have to take the height
/// of the window and subtract it by the top and bottom values of the position. If we had a window with a width
/// and height of 1000px, the UI element declared above would have a width of 700px and a height of 300px.
///
/// ```
/// // Size of the window
/// let window_width = 1000.0;
/// let window_height = 1000.0;
///
/// // Values of the position
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
/// If you define a [`Size`] and also all four sides of the position, the top and left values of the position
/// are used to determine where to place the UI element. The size will not be calculated using the bottom and
/// right values of the position because the size of the UI element is already explicitly specified.
///
/// ```
/// # use bevy_ui::{UiRect, Size, Val, Style};
/// # use bevy_utils::default;
/// #
/// let style = Style {
///     position: UiRect { // Defining all four sides
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
pub struct UiRect<T: Reflect + PartialEq> {
    /// The value corresponding to the left side of the UI rect.
    pub left: T,
    /// The value corresponding to the right side of the UI rect.
    pub right: T,
    /// The value corresponding to the top side of the UI rect.
    pub top: T,
    /// The value corresponding to the bottom side of the UI rect.
    pub bottom: T,
}

impl<T: Reflect + PartialEq> UiRect<T> {
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

/// A 2-dimensional area defined by a width and height.
///
/// It is commonly used to define the size of a text or UI element.
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
    /// ```
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
        assert_eq!(
            Size::new(10., 10.) + Vec2::new(10., 10.),
            Size::new(20., 20.)
        );
        assert_eq!(
            Size::new(20., 20.) - Vec2::new(10., 10.),
            Size::new(10., 10.)
        );
        assert_eq!(Size::new(10., 10.) * 2., Size::new(20., 20.));
        assert_eq!(Size::new(20., 20.) / 2., Size::new(10., 10.));

        let mut size = Size::new(10., 10.);

        size += Vec2::new(10., 10.);

        assert_eq!(size, Size::new(20., 20.));
    }
}
