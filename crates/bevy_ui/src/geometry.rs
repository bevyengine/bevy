use std::ops::{Mul, MulAssign, Div, DivAssign};

use crate::{Val, Num};
use bevy_reflect::Reflect;

macro_rules! uirect_impl {
    ($t:ty, $v:ty, $e:expr) => {
        impl $t {
            /// Default value for the fields of the field type.
            pub const FIELD_DEFAULT: $v = $e;

            pub const DEFAULT: Self = {
                Self {
                    left: Self::FIELD_DEFAULT,
                    right: Self::FIELD_DEFAULT,
                    top: Self::FIELD_DEFAULT,
                    bottom: Self::FIELD_DEFAULT,
                }
            };

            /// Creates a new uirect type from the values specified.
            pub fn new(
                left: impl Into<$v>,
                right: impl Into<$v>,
                top: impl Into<$v>,
                bottom: impl Into<$v>,
            ) -> Self {
                Self {
                    left: left.into(),
                    right: right.into(),
                    top: top.into(),
                    bottom: bottom.into(),
                }
            }

            
            /// Creates a new uirect from the values specified in logical pixels.
            ///
            /// This is a shortcut for [`UiRect::new()`], applying [`Val::Px`] to all arguments.
            ///
            /// # Example
            ///
            /// ```
            /// # use bevy_ui::{UiRect, Val};
            /// #
            /// let ui_rect = UiRect::px(10., 20., 30., 40.);
            /// assert_eq!(ui_rect.left, Val::Px(10.));
            /// assert_eq!(ui_rect.right, Val::Px(20.));
            /// assert_eq!(ui_rect.top, Val::Px(30.));
            /// assert_eq!(ui_rect.bottom, Val::Px(40.));
            /// ```
            pub fn px(left: f32, right: f32, top: f32, bottom: f32) -> Self {
                Self::new(
                    Num::Px(left),
                    Num::Px(right),
                    Num::Px(top),
                    Num::Px(bottom),
                )
            }

            /// Creates a new [`UiRect`] from the values specified in percentages.
            ///
            /// This is a shortcut for [`UiRect::new()`], applying [`Val::Percent`] to all arguments.
            ///
            /// # Example
            ///
            /// ```
            /// # use bevy_ui::{UiRect, Val};
            /// #
            /// let ui_rect = UiRect::percent(5., 10., 2., 1.);
            /// assert_eq!(ui_rect.left, Val::Percent(5.));
            /// assert_eq!(ui_rect.right, Val::Percent(10.));
            /// assert_eq!(ui_rect.top, Val::Percent(2.));
            /// assert_eq!(ui_rect.bottom, Val::Percent(1.));
            /// ```
            pub fn percent(left: f32, right: f32, top: f32, bottom: f32) -> Self {
                Self::new(
                    Num::Percent(left),
                    Num::Percent(right),
                    Num::Percent(top),
                    Num::Percent(bottom),
                )
            }

            /// Creates a new uirect type where `left` takes the given value.
            pub fn left(left: impl Into<$v>) -> Self {
                Self::new(
                    left.into(),
                    Self::FIELD_DEFAULT,
                    Self::FIELD_DEFAULT,
                    Self::FIELD_DEFAULT,
                )
            }

            /// Creates a new uirect type where `right` takes the given value.
            pub fn right(right: impl Into<$v>) -> Self {
                Self::new(
                    Self::FIELD_DEFAULT,
                    right.into(),
                    Self::FIELD_DEFAULT,
                    Self::FIELD_DEFAULT,
                )
            }

            /// Creates a new uirect type where `top` takes the given value.
            pub fn top(top: impl Into<$v>) -> Self {
                Self::new(
                    Self::FIELD_DEFAULT,
                    Self::FIELD_DEFAULT,
                    top.into(),
                    Self::FIELD_DEFAULT,
                )
            }

            /// Creates a new uirect type where `bottom` takes the given value.
            pub fn bottom(bottom: impl Into<$v>) -> Self {
                Self::new(
                    Self::FIELD_DEFAULT,
                    Self::FIELD_DEFAULT,
                    Self::FIELD_DEFAULT,
                    bottom.into(),
                )
            }

            /// Creates a new uirect type where `left` and `right` take the given value.
            pub fn horizontal(value: impl Into<$v> + Copy) -> Self {
                Self::new(value, value, Self::FIELD_DEFAULT, Self::FIELD_DEFAULT)
            }

            /// Creates a new uirect type where `top` and `bottom` take the given value.
            pub fn vertical(value: impl Into<$v> + Copy) -> Self {
                Self::new(Self::FIELD_DEFAULT, Self::FIELD_DEFAULT, value, value)
            }

            /// Creates a new uirect type where all sides have the same value.
            pub fn axes(horizontal: impl Into<$v> + Copy, vertical: impl Into<$v> + Copy) -> Self {
                Self::new(horizontal, horizontal, vertical, vertical)
            }

            /// Creates a new uirect type where all sides have the same value.
            pub fn all(value: impl Into<$v> + Copy) -> Self {
                Self::new(value, value, value, value)
            }

            
        }

        impl Default for $t {
            fn default() -> Self {
                Self::DEFAULT
            }
        }
    };
}

/// A margin is used to create space around UI elements, outside of any defined borders.
///
/// ```
/// # use bevy_ui::{Margin, AutoVal};
/// #
/// let margin = Margin::all(AutoVal::Auto); // Centers the UI element
/// ```
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Margin {
    pub left: Val,
    pub right: Val,
    pub top: Val,
    pub bottom: Val,
}

uirect_impl!(Margin, Val, Val::Px(0.));

/// A padding is used to create space around UI elements, inside of any defined borders.
///
/// ```
/// # use bevy_ui::{Padding, Val};
/// #
/// let padding = Padding {
///     left: Val::Px(10.0),
///     right: Val::Px(20.0),
///     top: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
/// ```
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Padding {
    pub left: Num,
    pub right: Num,
    pub top: Num,
    pub bottom: Num,
}

uirect_impl!(Padding, Num, Num::Px(0.));

/// ## Borders
///
/// A border is used to define the width of the border of a UI element.
///
/// ```
/// # use bevy_ui::{Border, Val};
/// #
/// let border = Border {
///     left: Val::Px(10.0),
///     right: Val::Px(20.0),
///     top: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
/// ```
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
pub struct Border {
    pub left: Num,
    pub right: Num,
    pub top: Num,
    pub bottom: Num,
}

uirect_impl!(Border, Num, Num::Px(0.));

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
    /// # use bevy_ui::{Size, AutoVal};
    /// #
    /// let size = Size::new(AutoVal::Px(100.0), AutoVal::Px(200.0));
    ///
    /// assert_eq!(size.width, AutoVal::Px(100.0));
    /// assert_eq!(size.height, AutoVal::Px(200.0));
    /// ```
    #[inline]
    pub fn new(width: impl Into<Val>, height: impl Into<Val>) -> Self {
        Size {
            width: width.into(),
            height: height.into(),
        }
    }

    /// Creates a new [`Size`] where both sides take the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Size, AutoVal};
    /// #
    /// let size = Size::all(AutoVal::Px(10.));
    ///
    /// assert_eq!(size.width, AutoVal::Px(10.0));
    /// assert_eq!(size.height, AutoVal::Px(10.0));
    /// ```
    #[inline]
    pub fn all(value: impl Into<Val> + Copy) -> Self {
        Self {
            width: value.into(),
            height: value.into(),
        }
    }

    /// Creates a new [`Size`] where `width` takes the given value,
    /// and `height` is set to [`AutoVal::Auto`].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Size, AutoVal};
    /// #
    /// let size = Size::width(AutoVal::Px(10.));
    ///
    /// assert_eq!(size.width, AutoVal::Px(10.0));
    /// assert_eq!(size.height, AutoVal::Auto);
    /// ```
    #[inline]
    pub fn width(width: impl Into<Val>) -> Self {
        Self {
            width: width.into(),
            height: Val::Auto,
        }
    }

    /// Creates a new [`Size`] where `height` takes the given value,
    /// and `width` is set to [`AutoVal::Auto`].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{Size, AutoVal};
    /// #
    /// let size = Size::height(AutoVal::Px(10.));
    ///
    /// assert_eq!(size.width, AutoVal::Auto);
    /// assert_eq!(size.height, AutoVal::Px(10.));
    /// ```
    #[inline]
    pub fn height(height: impl Into<Val>) -> Self {
        Self {
            width: Val::Auto,
            height: height.into(),
        }
    }

    /// Creates a Size where both values are [`AutoVal::Auto`].
    pub const AUTO: Self = Self {
        width: Val::Auto,
        height: Val::Auto,
    };
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
    fn border_default_equals_const_default() {
        assert_eq!(Border::default().left, Padding::FIELD_DEFAULT);
        assert_eq!(
            Border::default(),
            Border {
                left: Num::Px(0.),
                right: Num::Px(0.),
                top: Num::Px(0.),
                bottom: Num::Px(0.)
            }
        );
        assert_eq!(Border::default(), Border::DEFAULT);
    }

    #[test]
    fn margin_default_equals_const_default() {
        assert_eq!(Margin::default().left, Margin::FIELD_DEFAULT);
        assert_eq!(
            Margin::default(),
            Margin {
                left: Val::Px(0.),
                right: Val::Px(0.),
                top: Val::Px(0.),
                bottom: Val::Px(0.)
            }
        );
        assert_eq!(Margin::default(), Margin::DEFAULT);
    }

    #[test]
    fn padding_default_equals_const_default() {
        assert_eq!(Padding::default().left, Padding::FIELD_DEFAULT);
        assert_eq!(
            Padding::default(),
            Padding {
                left: Num::Px(0.),
                right: Num::Px(0.),
                top: Num::Px(0.),
                bottom: Num::Px(0.)
            }
        );
        assert_eq!(Padding::default(), Padding::DEFAULT);
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
        assert_eq!(
            Size::all(Val::Px(10.)) * 2.,
            Size::all(Val::Px(20.))
        );

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
    
    #[test]
    fn test_uirect_axes() {
        let x = Num::Px(1.);
        let y = Num::Vw(4.);
        let r = Border::axes(x, y);
        let h = Border::horizontal(x);
        let v = Border::vertical(y);

        assert_eq!(r.top, v.top);
        assert_eq!(r.bottom, v.bottom);
        assert_eq!(r.left, h.left);
        assert_eq!(r.right, h.right);
    }

    #[test]
    fn uirect_px() {
        let r = Border::px(3., 5., 20., 999.);
        assert_eq!(r.left, Num::Px(3.));
        assert_eq!(r.right, Num::Px(5.));
        assert_eq!(r.top, Num::Px(20.));
        assert_eq!(r.bottom, Num::Px(999.));
    }

    #[test]
    fn uirect_percent() {
        let r = Border::percent(3., 5., 20., 99.);
        assert_eq!(r.left, Num::Percent(3.));
        assert_eq!(r.right, Num::Percent(5.));
        assert_eq!(r.top, Num::Percent(20.));
        assert_eq!(r.bottom, Num::Percent(99.));
    }
}
