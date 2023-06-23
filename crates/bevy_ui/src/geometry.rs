use crate::Val;
use bevy_reflect::{FromReflect, Reflect, ReflectFromReflect};

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
#[derive(Copy, Clone, PartialEq, Debug, Reflect, FromReflect)]
#[reflect(FromReflect, PartialEq)]
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

    /// Creates a new [`UiRect`] from the values specified in logical pixels.
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
    pub const fn px(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        UiRect {
            left: Val::Px(left),
            right: Val::Px(right),
            top: Val::Px(top),
            bottom: Val::Px(bottom),
        }
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
    pub const fn percent(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        UiRect {
            left: Val::Percent(left),
            right: Val::Percent(right),
            top: Val::Percent(top),
            bottom: Val::Percent(bottom),
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

    /// Creates a new [`UiRect`] where both `left` and `right` take the value of `horizontal`, and both `top` and `bottom` take the value of `vertical`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::axes(Val::Px(10.0), Val::Percent(15.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Percent(15.0));
    /// assert_eq!(ui_rect.bottom, Val::Percent(15.0));
    /// ```
    pub fn axes(horizontal: Val, vertical: Val) -> Self {
        UiRect {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
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
    fn test_uirect_axes() {
        let x = Val::Px(1.);
        let y = Val::Vw(4.);
        let r = UiRect::axes(x, y);
        let h = UiRect::horizontal(x);
        let v = UiRect::vertical(y);

        assert_eq!(r.top, v.top);
        assert_eq!(r.bottom, v.bottom);
        assert_eq!(r.left, h.left);
        assert_eq!(r.right, h.right);
    }

    #[test]
    fn uirect_px() {
        let r = UiRect::px(3., 5., 20., 999.);
        assert_eq!(r.left, Val::Px(3.));
        assert_eq!(r.right, Val::Px(5.));
        assert_eq!(r.top, Val::Px(20.));
        assert_eq!(r.bottom, Val::Px(999.));
    }

    #[test]
    fn uirect_percent() {
        let r = UiRect::percent(3., 5., 20., 99.);
        assert_eq!(r.left, Val::Percent(3.));
        assert_eq!(r.right, Val::Percent(5.));
        assert_eq!(r.top, Val::Percent(20.));
        assert_eq!(r.bottom, Val::Percent(99.));
    }
}
