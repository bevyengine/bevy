use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_utils::default;
use core::ops::{Div, DivAssign, Mul, MulAssign, Neg};
use thiserror::Error;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Represents the possible value types for layout properties.
///
/// This enum allows specifying values for various [`Node`](crate::Node) properties in different units,
/// such as logical pixels, percentages, or automatically determined values.
///
/// `Val` also implements [`core::str::FromStr`] to allow parsing values from strings in the format `#.#px`. Whitespaces between the value and unit is allowed. The following units are supported:
/// * `px`: logical pixels
/// * `%`: percentage
/// * `vw`: percentage of the viewport width
/// * `vh`: percentage of the viewport height
/// * `vmin`: percentage of the viewport's smaller dimension
/// * `vmax`: percentage of the viewport's larger dimension
///
/// Additionally, `auto` will be parsed as [`Val::Auto`].
#[derive(Copy, Clone, Debug, Reflect)]
#[reflect(Default, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum Val {
    /// Automatically determine the value based on the context and other [`Node`](crate::Node) properties.
    Auto,
    /// Set this value in logical pixels.
    Px(f32),
    /// Set the value as a percentage of its parent node's length along a specific axis.
    ///
    /// If the UI node has no parent, the percentage is calculated based on the window's length
    /// along the corresponding axis.
    ///
    /// The chosen axis depends on the [`Node`](crate::Node) field set:
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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ValParseError {
    UnitMissing,
    ValueMissing,
    InvalidValue,
    InvalidUnit,
}

impl core::fmt::Display for ValParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ValParseError::UnitMissing => write!(f, "unit missing"),
            ValParseError::ValueMissing => write!(f, "value missing"),
            ValParseError::InvalidValue => write!(f, "invalid value"),
            ValParseError::InvalidUnit => write!(f, "invalid unit"),
        }
    }
}

impl core::str::FromStr for Val {
    type Err = ValParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.eq_ignore_ascii_case("auto") {
            return Ok(Val::Auto);
        }

        let Some(end_of_number) = s
            .bytes()
            .position(|c| !(c.is_ascii_digit() || c == b'.' || c == b'-' || c == b'+'))
        else {
            return Err(ValParseError::UnitMissing);
        };

        if end_of_number == 0 {
            return Err(ValParseError::ValueMissing);
        }

        let (value, unit) = s.split_at(end_of_number);

        let value: f32 = value.parse().map_err(|_| ValParseError::InvalidValue)?;

        let unit = unit.trim();

        if unit.eq_ignore_ascii_case("px") {
            Ok(Val::Px(value))
        } else if unit.eq_ignore_ascii_case("%") {
            Ok(Val::Percent(value))
        } else if unit.eq_ignore_ascii_case("vw") {
            Ok(Val::Vw(value))
        } else if unit.eq_ignore_ascii_case("vh") {
            Ok(Val::Vh(value))
        } else if unit.eq_ignore_ascii_case("vmin") {
            Ok(Val::VMin(value))
        } else if unit.eq_ignore_ascii_case("vmax") {
            Ok(Val::VMax(value))
        } else {
            Err(ValParseError::InvalidUnit)
        }
    }
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
    #[error("the given variant of Val is not evaluable (non-numeric)")]
    NonEvaluable,
}

impl Val {
    /// Resolves this [`Val`] to a value in physical pixels from the given `scale_factor`, `physical_base_value`,
    /// and `physical_target_size` context values.
    ///
    /// Returns a [`ValArithmeticError::NonEvaluable`] if the [`Val`] is impossible to resolve into a concrete value.
    pub const fn resolve(
        self,
        scale_factor: f32,
        physical_base_value: f32,
        physical_target_size: Vec2,
    ) -> Result<f32, ValArithmeticError> {
        match self {
            Val::Percent(value) => Ok(physical_base_value * value / 100.0),
            Val::Px(value) => Ok(value * scale_factor),
            Val::Vw(value) => Ok(physical_target_size.x * value / 100.0),
            Val::Vh(value) => Ok(physical_target_size.y * value / 100.0),
            Val::VMin(value) => {
                Ok(physical_target_size.x.min(physical_target_size.y) * value / 100.0)
            }
            Val::VMax(value) => {
                Ok(physical_target_size.x.max(physical_target_size.y) * value / 100.0)
            }
            Val::Auto => Err(ValArithmeticError::NonEvaluable),
        }
    }
}

/// Returns a [`Val::Auto`] where the value is automatically determined
/// based on the context and other [`Node`](crate::Node) properties.
pub const fn auto() -> Val {
    Val::Auto
}

/// Returns a [`Val::Px`] representing a value in logical pixels.
pub const fn px(value: f32) -> Val {
    Val::Px(value)
}

/// Returns a [`Val::Percent`] representing a percentage of the parent node's length
/// along a specific axis.
///
/// If the UI node has no parent, the percentage is based on the window's length
/// along that axis.
///
/// Axis rules:
/// * For `flex_basis`, the percentage is relative to the main-axis length determined by the `flex_direction`.
/// * For `gap`, `min_size`, `size`, and `max_size`:
///   - `width` is relative to the parent's width.
///   - `height` is relative to the parent's height.
/// * For `margin`, `padding`, and `border` values: the percentage is relative to the parent's width.
/// * For positions, `left` and `right` are relative to the parent's width, while `bottom` and `top` are relative to the parent's height.
pub const fn percent(value: f32) -> Val {
    Val::Percent(value)
}

/// Returns a [`Val::Vw`] representing a percentage of the viewport width.
pub const fn vw(value: f32) -> Val {
    Val::Vw(value)
}

/// Returns a [`Val::Vh`] representing a percentage of the viewport height.
pub const fn vh(value: f32) -> Val {
    Val::Vh(value)
}

/// Returns a [`Val::VMin`] representing a percentage of the viewport's smaller dimension.
pub const fn vmin(value: f32) -> Val {
    Val::VMin(value)
}

/// Returns a [`Val::VMax`] representing a percentage of the viewport's larger dimension.
pub const fn vmax(value: f32) -> Val {
    Val::VMax(value)
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
#[reflect(Default, PartialEq, Debug, Clone)]
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
    pub const DEFAULT: Self = Self::all(Val::ZERO);
    pub const ZERO: Self = Self::all(Val::ZERO);
    pub const AUTO: Self = Self::all(Val::Auto);

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
    pub const fn horizontal(value: Val) -> Self {
        Self {
            left: value,
            right: value,
            ..Self::DEFAULT
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
    pub const fn vertical(value: Val) -> Self {
        Self {
            top: value,
            bottom: value,
            ..Self::DEFAULT
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
    pub const fn axes(horizontal: Val, vertical: Val) -> Self {
        Self {
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
    pub const fn left(left: Val) -> Self {
        Self {
            left,
            ..Self::DEFAULT
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
    pub const fn right(right: Val) -> Self {
        Self {
            right,
            ..Self::DEFAULT
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
    pub const fn top(top: Val) -> Self {
        Self {
            top,
            ..Self::DEFAULT
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
    pub const fn bottom(bottom: Val) -> Self {
        Self {
            bottom,
            ..Self::DEFAULT
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
    pub const fn with_left(mut self, left: Val) -> Self {
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
    pub const fn with_right(mut self, right: Val) -> Self {
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
    pub const fn with_top(mut self, top: Val) -> Self {
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
    pub const fn with_bottom(mut self, bottom: Val) -> Self {
        self.bottom = bottom;
        self
    }
}

impl Default for UiRect {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<Val> for UiRect {
    fn from(value: Val) -> Self {
        UiRect::all(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Default, Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
/// Responsive position relative to a UI node.
pub struct UiPosition {
    /// Normalized anchor point
    pub anchor: Vec2,
    /// Responsive horizontal position relative to the anchor point
    pub x: Val,
    /// Responsive vertical position relative to the anchor point
    pub y: Val,
}

impl Default for UiPosition {
    fn default() -> Self {
        Self::CENTER
    }
}

impl UiPosition {
    /// Position at the given normalized anchor point
    pub const fn anchor(anchor: Vec2) -> Self {
        Self {
            anchor,
            x: Val::ZERO,
            y: Val::ZERO,
        }
    }

    /// Position at the top-left corner
    pub const TOP_LEFT: Self = Self::anchor(Vec2::new(-0.5, -0.5));

    /// Position at the center of the left edge
    pub const LEFT: Self = Self::anchor(Vec2::new(-0.5, 0.0));

    /// Position at the bottom-left corner
    pub const BOTTOM_LEFT: Self = Self::anchor(Vec2::new(-0.5, 0.5));

    /// Position at the center of the top edge
    pub const TOP: Self = Self::anchor(Vec2::new(0.0, -0.5));

    /// Position at the center of the element
    pub const CENTER: Self = Self::anchor(Vec2::new(0.0, 0.0));

    /// Position at the center of the bottom edge
    pub const BOTTOM: Self = Self::anchor(Vec2::new(0.0, 0.5));

    /// Position at the top-right corner
    pub const TOP_RIGHT: Self = Self::anchor(Vec2::new(0.5, -0.5));

    /// Position at the center of the right edge
    pub const RIGHT: Self = Self::anchor(Vec2::new(0.5, 0.0));

    /// Position at the bottom-right corner
    pub const BOTTOM_RIGHT: Self = Self::anchor(Vec2::new(0.5, 0.5));

    /// Create a new position
    pub const fn new(anchor: Vec2, x: Val, y: Val) -> Self {
        Self { anchor, x, y }
    }

    /// Creates a position from self with the given `x` and `y` coordinates
    pub const fn at(self, x: Val, y: Val) -> Self {
        Self { x, y, ..self }
    }

    /// Creates a position from self with the given `x` coordinate
    pub const fn at_x(self, x: Val) -> Self {
        Self { x, ..self }
    }

    /// Creates a position from self with the given `y` coordinate
    pub const fn at_y(self, y: Val) -> Self {
        Self { y, ..self }
    }

    /// Creates a position in logical pixels from self with the given `x` and `y` coordinates
    pub const fn at_px(self, x: f32, y: f32) -> Self {
        self.at(Val::Px(x), Val::Px(y))
    }

    /// Creates a percentage position from self with the given `x` and `y` coordinates
    pub const fn at_percent(self, x: f32, y: f32) -> Self {
        self.at(Val::Percent(x), Val::Percent(y))
    }

    /// Creates a position from self with the given `anchor` point
    pub const fn with_anchor(self, anchor: Vec2) -> Self {
        Self { anchor, ..self }
    }

    /// Position relative to the top-left corner
    pub const fn top_left(x: Val, y: Val) -> Self {
        Self::TOP_LEFT.at(x, y)
    }

    /// Position relative to the left edge
    pub const fn left(x: Val, y: Val) -> Self {
        Self::LEFT.at(x, y)
    }

    /// Position relative to the bottom-left corner
    pub const fn bottom_left(x: Val, y: Val) -> Self {
        Self::BOTTOM_LEFT.at(x, y)
    }

    /// Position relative to the top edge
    pub const fn top(x: Val, y: Val) -> Self {
        Self::TOP.at(x, y)
    }

    /// Position relative to the center
    pub const fn center(x: Val, y: Val) -> Self {
        Self::CENTER.at(x, y)
    }

    /// Position relative to the bottom edge
    pub const fn bottom(x: Val, y: Val) -> Self {
        Self::BOTTOM.at(x, y)
    }

    /// Position relative to the top-right corner
    pub const fn top_right(x: Val, y: Val) -> Self {
        Self::TOP_RIGHT.at(x, y)
    }

    /// Position relative to the right edge
    pub const fn right(x: Val, y: Val) -> Self {
        Self::RIGHT.at(x, y)
    }

    /// Position relative to the bottom-right corner
    pub const fn bottom_right(x: Val, y: Val) -> Self {
        Self::BOTTOM_RIGHT.at(x, y)
    }

    /// Resolves the `Position` into physical coordinates.
    pub fn resolve(
        self,
        scale_factor: f32,
        physical_size: Vec2,
        physical_target_size: Vec2,
    ) -> Vec2 {
        let d = self.anchor.map(|p| if 0. < p { -1. } else { 1. });

        physical_size * self.anchor
            + d * Vec2::new(
                self.x
                    .resolve(scale_factor, physical_size.x, physical_target_size)
                    .unwrap_or(0.),
                self.y
                    .resolve(scale_factor, physical_size.y, physical_target_size)
                    .unwrap_or(0.),
            )
    }
}

impl From<Val> for UiPosition {
    fn from(x: Val) -> Self {
        Self { x, ..default() }
    }
}

impl From<(Val, Val)> for UiPosition {
    fn from((x, y): (Val, Val)) -> Self {
        Self { x, y, ..default() }
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
        let result = Val::Percent(80.).resolve(1., size, viewport_size).unwrap();

        assert_eq!(result, size * 0.8);
    }

    #[test]
    fn val_resolve_px() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let result = Val::Px(10.).resolve(1., size, viewport_size).unwrap();

        assert_eq!(result, 10.);
    }

    #[test]
    fn val_resolve_viewport_coords() {
        let size = 250.;
        let viewport_size = vec2(500., 500.);

        for value in (-10..10).map(|value| value as f32) {
            // for a square viewport there should be no difference between `Vw` and `Vh` and between `Vmin` and `Vmax`.
            assert_eq!(
                Val::Vw(value).resolve(1., size, viewport_size),
                Val::Vh(value).resolve(1., size, viewport_size)
            );
            assert_eq!(
                Val::VMin(value).resolve(1., size, viewport_size),
                Val::VMax(value).resolve(1., size, viewport_size)
            );
            assert_eq!(
                Val::VMin(value).resolve(1., size, viewport_size),
                Val::Vw(value).resolve(1., size, viewport_size)
            );
        }

        let viewport_size = vec2(1000., 500.);
        assert_eq!(
            Val::Vw(100.).resolve(1., size, viewport_size).unwrap(),
            1000.
        );
        assert_eq!(
            Val::Vh(100.).resolve(1., size, viewport_size).unwrap(),
            500.
        );
        assert_eq!(Val::Vw(60.).resolve(1., size, viewport_size).unwrap(), 600.);
        assert_eq!(Val::Vh(40.).resolve(1., size, viewport_size).unwrap(), 200.);
        assert_eq!(
            Val::VMin(50.).resolve(1., size, viewport_size).unwrap(),
            250.
        );
        assert_eq!(
            Val::VMax(75.).resolve(1., size, viewport_size).unwrap(),
            750.
        );
    }

    #[test]
    fn val_auto_is_non_evaluable() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let resolve_auto = Val::Auto.resolve(1., size, viewport_size);

        assert_eq!(resolve_auto, Err(ValArithmeticError::NonEvaluable));
    }

    #[test]
    fn val_arithmetic_error_messages() {
        assert_eq!(
            format!("{}", ValArithmeticError::NonEvaluable),
            "the given variant of Val is not evaluable (non-numeric)"
        );
    }

    #[test]
    fn val_str_parse() {
        assert_eq!("auto".parse::<Val>(), Ok(Val::Auto));
        assert_eq!("Auto".parse::<Val>(), Ok(Val::Auto));
        assert_eq!("AUTO".parse::<Val>(), Ok(Val::Auto));

        assert_eq!("3px".parse::<Val>(), Ok(Val::Px(3.)));
        assert_eq!("3 px".parse::<Val>(), Ok(Val::Px(3.)));
        assert_eq!("3.5px".parse::<Val>(), Ok(Val::Px(3.5)));
        assert_eq!("-3px".parse::<Val>(), Ok(Val::Px(-3.)));
        assert_eq!("3.5 PX".parse::<Val>(), Ok(Val::Px(3.5)));

        assert_eq!("3%".parse::<Val>(), Ok(Val::Percent(3.)));
        assert_eq!("3 %".parse::<Val>(), Ok(Val::Percent(3.)));
        assert_eq!("3.5%".parse::<Val>(), Ok(Val::Percent(3.5)));
        assert_eq!("-3%".parse::<Val>(), Ok(Val::Percent(-3.)));

        assert_eq!("3vw".parse::<Val>(), Ok(Val::Vw(3.)));
        assert_eq!("3 vw".parse::<Val>(), Ok(Val::Vw(3.)));
        assert_eq!("3.5vw".parse::<Val>(), Ok(Val::Vw(3.5)));
        assert_eq!("-3vw".parse::<Val>(), Ok(Val::Vw(-3.)));
        assert_eq!("3.5 VW".parse::<Val>(), Ok(Val::Vw(3.5)));

        assert_eq!("3vh".parse::<Val>(), Ok(Val::Vh(3.)));
        assert_eq!("3 vh".parse::<Val>(), Ok(Val::Vh(3.)));
        assert_eq!("3.5vh".parse::<Val>(), Ok(Val::Vh(3.5)));
        assert_eq!("-3vh".parse::<Val>(), Ok(Val::Vh(-3.)));
        assert_eq!("3.5 VH".parse::<Val>(), Ok(Val::Vh(3.5)));

        assert_eq!("3vmin".parse::<Val>(), Ok(Val::VMin(3.)));
        assert_eq!("3 vmin".parse::<Val>(), Ok(Val::VMin(3.)));
        assert_eq!("3.5vmin".parse::<Val>(), Ok(Val::VMin(3.5)));
        assert_eq!("-3vmin".parse::<Val>(), Ok(Val::VMin(-3.)));
        assert_eq!("3.5 VMIN".parse::<Val>(), Ok(Val::VMin(3.5)));

        assert_eq!("3vmax".parse::<Val>(), Ok(Val::VMax(3.)));
        assert_eq!("3 vmax".parse::<Val>(), Ok(Val::VMax(3.)));
        assert_eq!("3.5vmax".parse::<Val>(), Ok(Val::VMax(3.5)));
        assert_eq!("-3vmax".parse::<Val>(), Ok(Val::VMax(-3.)));
        assert_eq!("3.5 VMAX".parse::<Val>(), Ok(Val::VMax(3.5)));

        assert_eq!("".parse::<Val>(), Err(ValParseError::UnitMissing));
        assert_eq!(
            "hello world".parse::<Val>(),
            Err(ValParseError::ValueMissing)
        );
        assert_eq!("3".parse::<Val>(), Err(ValParseError::UnitMissing));
        assert_eq!("3.5".parse::<Val>(), Err(ValParseError::UnitMissing));
        assert_eq!("3pxx".parse::<Val>(), Err(ValParseError::InvalidUnit));
        assert_eq!("3.5pxx".parse::<Val>(), Err(ValParseError::InvalidUnit));
        assert_eq!("3-3px".parse::<Val>(), Err(ValParseError::InvalidValue));
        assert_eq!("3.5-3px".parse::<Val>(), Err(ValParseError::InvalidValue));
    }

    #[test]
    fn default_val_equals_const_default_val() {
        assert_eq!(Val::default(), Val::DEFAULT);
    }

    #[test]
    fn uirect_default_equals_const_default() {
        assert_eq!(UiRect::default(), UiRect::all(Val::ZERO));
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
