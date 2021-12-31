use bevy_reflect::Reflect;
use glam::Vec2;
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

/// A 2-dimensional area defined by a width and height.
///
/// It is commonly used to define the size of a text or UI element.
///
/// # Example
///
/// ```rust
/// # use bevy_math::Size;
/// #
/// let size = Size::<f32> {
///     width: 1.0,
///     height: 2.0,
/// };
///
/// assert_eq!(size.width, 1.0);
/// assert_eq!(size.height, 2.0);
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
    /// # use bevy_math::Size;
    /// #
    /// let size = Size::new(1.0, 2.0);
    ///
    /// assert_eq!(size.width, 1.0);
    /// assert_eq!(size.height, 2.0);
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

/// A rectangle defined by its side locations.
///
/// It is commonly used to define positions, margins, paddings and borders.
///
/// The values supplied define the distances from the nominal center of the
/// rectangle to the edge. If any of the values supplied are negative the
/// nominal center wouldn't be inside of the rectangle.
///
/// # Example
///
/// In the following example the rectangle has a left value of 1.0 and a right
/// value of 2.0, which means that the width of the rectangle is 3.0. The actual
/// center of this rectangle is offset by 0.5 to the right of the nominal center.
///
/// ```rust
/// # use bevy_math::Rect;
/// #
/// let rect = Rect::<f32> {
///     left: 1.0,
///     right: 2.0,
///     top: 3.0,
///     bottom: 4.0,
/// };
///
/// assert_eq!(rect.left, 1.0);
/// assert_eq!(rect.right, 2.0);
/// assert_eq!(rect.top, 3.0);
/// assert_eq!(rect.bottom, 4.0);
/// ```
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Rect<T: Reflect + PartialEq> {
    /// The left side of the rectangle.
    pub left: T,
    /// The right side of the rectangle.
    pub right: T,
    /// The top side of the rectangle.
    pub top: T,
    /// The bottom side of the rectangle.
    pub bottom: T,
}

impl<T: Reflect + PartialEq> Rect<T> {
    /// Creates a new [`Rect`] where all sides are equidistant from the center.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use bevy_math::Rect;
    /// #
    /// let rect = Rect::<f32>::all(1.0);
    ///
    /// assert_eq!(rect.left, 1.0);
    /// assert_eq!(rect.right, 1.0);
    /// assert_eq!(rect.top, 1.0);
    /// assert_eq!(rect.bottom, 1.0);
    /// ```
    pub fn all(distance_from_center: T) -> Self
    where
        T: Clone,
    {
        Rect {
            left: distance_from_center.clone(),
            right: distance_from_center.clone(),
            top: distance_from_center.clone(),
            bottom: distance_from_center,
        }
    }
}

impl<T: Default + Reflect + PartialEq> Default for Rect<T> {
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
