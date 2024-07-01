use bevy_math::Vec2;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use std::ops::Neg;
use std::ops::{Div, DivAssign, Mul, MulAssign};
use thiserror::Error;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Represents the possible value types for layout properties.
///
/// This enum allows specifying values for various [`Style`](crate::Style) properties in different units,
/// such as logical pixels, percentages, or automatically determined values.

#[derive(Copy, Clone, Debug, Reflect)]
#[reflect(Default, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum Val {
    /// Automatically determine the value based on the context and other [`Style`](crate::Style) properties.
    Auto,
    /// Set this value in logical pixels.
    Px(f32),
    /// Set the value as a percentage of its parent node's length along a specific axis.
    ///
    /// If the UI node has no parent, the percentage is calculated based on the window's length
    /// along the corresponding axis.
    ///
    /// The chosen axis depends on the [`Style`](crate::Style) field set:
    /// * For `flex_basis`, the percentage is relative to the main-axis length determined by the `flex_direction`.
    /// * For `gap`, `min_size`, `size`, and `max_size`:
    ///   - `width` is relative to the parent's width.
    ///   - `height` is relative to the parent's height.
    /// * For `margin`, `padding`, and `border` values: the percentage is relative to the parent node's width.
    /// * For positions, `left` and `right` are relative to the parent's width, while `bottom` and `top` are relative to the parent's height.
    Percent(f32),
    /// Set this value in percent of the viewport width
    Vw(f32),
    /// Set this value in percent of the viewport height
    Vh(f32),
    /// Set this value in percent of the viewport's smaller dimension.
    VMin(f32),
    /// Set this value in percent of the viewport's larger dimension.
    VMax(f32),
}

impl PartialEq for Val {
    fn eq(&self, other: &Self) -> bool {
        let same_unit = matches!(
            (self, other),
            (Self::Auto, Self::Auto)
                | (Self::Px(_), Self::Px(_))
                | (Self::Percent(_), Self::Percent(_))
                | (Self::Vw(_), Self::Vw(_))
                | (Self::Vh(_), Self::Vh(_))
                | (Self::VMin(_), Self::VMin(_))
                | (Self::VMax(_), Self::VMax(_))
        );

        let left = match self {
            Self::Auto => None,
            Self::Px(v)
            | Self::Percent(v)
            | Self::Vw(v)
            | Self::Vh(v)
            | Self::VMin(v)
            | Self::VMax(v) => Some(v),
        };

        let right = match other {
            Self::Auto => None,
            Self::Px(v)
            | Self::Percent(v)
            | Self::Vw(v)
            | Self::Vh(v)
            | Self::VMin(v)
            | Self::VMax(v) => Some(v),
        };

        match (same_unit, left, right) {
            (true, a, b) => a == b,
            // All zero-value variants are considered equal.
            (false, Some(&a), Some(&b)) => a == 0. && b == 0.,
            _ => false,
        }
    }
}

impl Val {
    pub const DEFAULT: Self = Self::Auto;
    pub const ZERO: Self = Self::Px(0.0);
}

impl Default for Val {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Mul<f32> for Val {
    type Output = Val;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Val::Auto => Val::Auto,
            Val::Px(value) => Val::Px(value * rhs),
            Val::Percent(value) => Val::Percent(value * rhs),
            Val::Vw(value) => Val::Vw(value * rhs),
            Val::Vh(value) => Val::Vh(value * rhs),
            Val::VMin(value) => Val::VMin(value * rhs),
            Val::VMax(value) => Val::VMax(value * rhs),
        }
    }
}

impl MulAssign<f32> for Val {
    fn mul_assign(&mut self, rhs: f32) {
        match self {
            Val::Auto => {}
            Val::Px(value)
            | Val::Percent(value)
            | Val::Vw(value)
            | Val::Vh(value)
            | Val::VMin(value)
            | Val::VMax(value) => *value *= rhs,
        }
    }
}

impl Div<f32> for Val {
    type Output = Val;

    fn div(self, rhs: f32) -> Self::Output {
        match self {
            Val::Auto => Val::Auto,
            Val::Px(value) => Val::Px(value / rhs),
            Val::Percent(value) => Val::Percent(value / rhs),
            Val::Vw(value) => Val::Vw(value / rhs),
            Val::Vh(value) => Val::Vh(value / rhs),
            Val::VMin(value) => Val::VMin(value / rhs),
            Val::VMax(value) => Val::VMax(value / rhs),
        }
    }
}

impl DivAssign<f32> for Val {
    fn div_assign(&mut self, rhs: f32) {
        match self {
            Val::Auto => {}
            Val::Px(value)
            | Val::Percent(value)
            | Val::Vw(value)
            | Val::Vh(value)
            | Val::VMin(value)
            | Val::VMax(value) => *value /= rhs,
        }
    }
}

impl Neg for Val {
    type Output = Val;

    fn neg(self) -> Self::Output {
        match self {
            Val::Px(value) => Val::Px(-value),
            Val::Percent(value) => Val::Percent(-value),
            Val::Vw(value) => Val::Vw(-value),
            Val::Vh(value) => Val::Vh(-value),
            Val::VMin(value) => Val::VMin(-value),
            Val::VMax(value) => Val::VMax(-value),
            _ => self,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Error)]
pub enum ValArithmeticError {
    #[error("the variants of the Vals don't match")]
    NonIdenticalVariants,
    #[error("the given variant of Val is not evaluateable (non-numeric)")]
    NonEvaluateable,
}

impl Val {
    /// Resolves a [`Val`] to its value in logical pixels and returns this as an [`f32`].
    /// Returns a [`ValArithmeticError::NonEvaluateable`] if the [`Val`] is impossible to resolve into a concrete value.
    ///
    /// **Note:** If a [`Val::Px`] is resolved, its inner value is returned unchanged.
    pub fn resolve(self, parent_size: f32, viewport_size: Vec2) -> Result<f32, ValArithmeticError> {
        match self {
            Val::Percent(value) => Ok(parent_size * value / 100.0),
            Val::Px(value) => Ok(value),
            Val::Vw(value) => Ok(viewport_size.x * value / 100.0),
            Val::Vh(value) => Ok(viewport_size.y * value / 100.0),
            Val::VMin(value) => Ok(viewport_size.min_element() * value / 100.0),
            Val::VMax(value) => Ok(viewport_size.max_element() * value / 100.0),
            Val::Auto => Err(ValArithmeticError::NonEvaluateable),
        }
    }
}

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
#[reflect(Default, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
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
        left: Val::ZERO,
        right: Val::ZERO,
        top: Val::ZERO,
        bottom: Val::ZERO,
    };

    pub const ZERO: Self = Self {
        left: Val::ZERO,
        right: Val::ZERO,
        top: Val::ZERO,
        bottom: Val::ZERO,
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
    /// and `top` and `bottom` set to zero `Val::ZERO`.
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
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub fn horizontal(value: Val) -> Self {
        UiRect {
            left: value,
            right: value,
            ..Default::default()
        }
    }

    /// Creates a new [`UiRect`] where `top` and `bottom` take the given value,
    /// and `left` and `right` are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::vertical(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
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
    /// the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::left(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub fn left(value: Val) -> Self {
        UiRect {
            left: value,
            ..Default::default()
        }
    }

    /// Creates a new [`UiRect`] where `right` takes the given value,
    /// and the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::right(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub fn right(value: Val) -> Self {
        UiRect {
            right: value,
            ..Default::default()
        }
    }

    /// Creates a new [`UiRect`] where `top` takes the given value,
    /// and the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::top(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub fn top(value: Val) -> Self {
        UiRect {
            top: value,
            ..Default::default()
        }
    }

    /// Creates a new [`UiRect`] where `bottom` takes the given value,
    /// and the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::bottom(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub fn bottom(value: Val) -> Self {
        UiRect {
            bottom: value,
            ..Default::default()
        }
    }

    /// Returns the [`UiRect`] with its `left` field set to the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(20.0)).with_left(Val::Px(10.0));
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(20.0));
    /// assert_eq!(ui_rect.top, Val::Px(20.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(20.0));
    /// ```
    #[inline]
    pub fn with_left(mut self, left: Val) -> Self {
        self.left = left;
        self
    }

    /// Returns the [`UiRect`] with its `right` field set to the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(20.0)).with_right(Val::Px(10.0));
    /// assert_eq!(ui_rect.left, Val::Px(20.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Px(20.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(20.0));
    /// ```
    #[inline]
    pub fn with_right(mut self, right: Val) -> Self {
        self.right = right;
        self
    }

    /// Returns the [`UiRect`] with its `top` field set to the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(20.0)).with_top(Val::Px(10.0));
    /// assert_eq!(ui_rect.left, Val::Px(20.0));
    /// assert_eq!(ui_rect.right, Val::Px(20.0));
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(20.0));
    /// ```
    #[inline]
    pub fn with_top(mut self, top: Val) -> Self {
        self.top = top;
        self
    }

    /// Returns the [`UiRect`] with its `bottom` field set to the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(20.0)).with_bottom(Val::Px(10.0));
    /// assert_eq!(ui_rect.left, Val::Px(20.0));
    /// assert_eq!(ui_rect.right, Val::Px(20.0));
    /// assert_eq!(ui_rect.top, Val::Px(20.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    #[inline]
    pub fn with_bottom(mut self, bottom: Val) -> Self {
        self.bottom = bottom;
        self
    }
}

impl Default for UiRect {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use crate::geometry::*;
    use bevy_math::vec2;

    #[test]
    fn val_evaluate() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let result = Val::Percent(80.).resolve(size, viewport_size).unwrap();

        assert_eq!(result, size * 0.8);
    }

    #[test]
    fn val_resolve_px() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let result = Val::Px(10.).resolve(size, viewport_size).unwrap();

        assert_eq!(result, 10.);
    }

    #[test]
    fn val_resolve_viewport_coords() {
        let size = 250.;
        let viewport_size = vec2(500., 500.);

        for value in (-10..10).map(|value| value as f32) {
            // for a square viewport there should be no difference between `Vw` and `Vh` and between `Vmin` and `Vmax`.
            assert_eq!(
                Val::Vw(value).resolve(size, viewport_size),
                Val::Vh(value).resolve(size, viewport_size)
            );
            assert_eq!(
                Val::VMin(value).resolve(size, viewport_size),
                Val::VMax(value).resolve(size, viewport_size)
            );
            assert_eq!(
                Val::VMin(value).resolve(size, viewport_size),
                Val::Vw(value).resolve(size, viewport_size)
            );
        }

        let viewport_size = vec2(1000., 500.);
        assert_eq!(Val::Vw(100.).resolve(size, viewport_size).unwrap(), 1000.);
        assert_eq!(Val::Vh(100.).resolve(size, viewport_size).unwrap(), 500.);
        assert_eq!(Val::Vw(60.).resolve(size, viewport_size).unwrap(), 600.);
        assert_eq!(Val::Vh(40.).resolve(size, viewport_size).unwrap(), 200.);
        assert_eq!(Val::VMin(50.).resolve(size, viewport_size).unwrap(), 250.);
        assert_eq!(Val::VMax(75.).resolve(size, viewport_size).unwrap(), 750.);
    }

    #[test]
    fn val_auto_is_non_resolveable() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let resolve_auto = Val::Auto.resolve(size, viewport_size);

        assert_eq!(resolve_auto, Err(ValArithmeticError::NonEvaluateable));
    }

    #[test]
    fn val_arithmetic_error_messages() {
        assert_eq!(
            format!("{}", ValArithmeticError::NonIdenticalVariants),
            "the variants of the Vals don't match"
        );
        assert_eq!(
            format!("{}", ValArithmeticError::NonEvaluateable),
            "the given variant of Val is not evaluateable (non-numeric)"
        );
    }

    #[test]
    fn default_val_equals_const_default_val() {
        assert_eq!(Val::default(), Val::DEFAULT);
    }

    #[test]
    fn uirect_default_equals_const_default() {
        assert_eq!(
            UiRect::default(),
            UiRect {
                left: Val::ZERO,
                right: Val::ZERO,
                top: Val::ZERO,
                bottom: Val::ZERO
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
