use bevy_math::{StableInterpolate, TryStableInterpolate, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_text::{EmSize, FontSize, RemSize, DEFAULT_REM_SIZE_PX};
use core::ops::{Div, DivAssign, Mul, MulAssign, Neg};
use thiserror::Error;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use crate::UiRect;

/// All the context needed to resolve a [`Val`] into a length in physical pixels, except the percentage basis.
#[derive(Copy, Clone)]
pub struct ValContext {
    /// Logical pixel lengths are multiplied by the scale factor to get a physical length.
    pub scale_factor: f32,
    /// Size of the target surface in physical pixels.
    pub physical_size: Vec2,
    /// The local font size in logical pixels.
    /// `Val::Em` values are normalized, so `em(1)` is `em_size * scale_factor` in physical pixels.
    pub em_size: f32,
    /// The root font size in logical pixels.
    /// `Val::Em` values are normalized, so `rem(1)` is `rem_size * scale_factor` in physical pixels.
    pub rem_size: f32,
}

impl ValContext {
    pub const DEFAULT: Self = Self {
        scale_factor: 1.0,
        physical_size: Vec2::ZERO,
        em_size: DEFAULT_REM_SIZE_PX,
        rem_size: DEFAULT_REM_SIZE_PX,
    };

    #[inline]
    pub const fn new(
        scale_factor: f32,
        physical_size: Vec2,
        em_size: EmSize,
        rem_size: RemSize,
    ) -> Self {
        Self {
            scale_factor,
            physical_size,
            em_size: em_size.0,
            rem_size: rem_size.0,
        }
    }
}

#[cfg(test)]
impl ValContext {
    pub const TEST_CONTEXT: Self = Self {
        physical_size: Vec2::new(1000.0, 1000.0),
        ..Self::DEFAULT
    };
}

impl Default for ValContext {
    fn default() -> Self {
        Self::DEFAULT
    }
}

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
    /// Set this value as a multiple of the node's own font size.
    ///
    /// `1em` is the node's font size, so `Val::Em(2.0)` is twice that.
    /// Resolved from the node's [`EmSize`] component.
    Em(f32),
    /// Set this value as a multiple of the root font size.
    ///
    /// Unlike [`Val::Em`] this ignores the node's own font size, so it means the same
    /// length anywhere in the hierarchy. Resolved from the [`RemSize`] resource.
    Rem(f32),
    /// The smallest size that can fit the node's contents, taking all available
    /// soft wrapping opportunities.
    ///
    /// Only supported on the `width`, `height` and `flex_basis` fields, behaves as `Auto` elsewhere.
    MinContent,
    /// The smallest size that can fit the node's contents, without taking any
    /// soft wrapping opportunities.
    ///
    /// Only supported on the `width`, `height` and `flex_basis` fields, behaves as `Auto` elsewhere.
    MaxContent,
    /// Size the node to fit the available space, clamped between min-content and max-content.
    ///
    /// Only supported on the `width`, `height` and `flex_basis` fields, behaves as `Auto` elsewhere.
    FitContent,
    /// Sizes the box to fill the available space, subject to layout constraints.
    /// (<https://www.w3.org/TR/css-sizing-4/#stretch-fit-sizing>)
    ///
    /// Compared to `Percent(100.)`, `Percent` sizes are relative to the parent's inner content size.
    /// A `Node` with 100% width and 10px margins may overflow its parent, but
    /// with `Stretch` the node's margins are accounted for when calculating its size.
    /// For example, with 300px available space and 10px margins, `Stretch` requests a width of 280px.
    ///
    /// Only supported on the `width`, `height` and `flex_basis` fields, behaves as `Auto` elsewhere.
    Stretch,
    /// The size of the node's content determines its initial flex size, rather than its `width` when the
    /// parent has `FlexDirection::Row`, or `height` when the parent has `FlexDirection::Column`.
    ///
    /// (<https://www.w3.org/TR/css-flexbox-1/#valdef-flex-basis-content>)
    ///
    /// Only supported on the `flex_basis` field, behaves as `Auto` elsewhere.
    Content,
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
        } else if unit.eq_ignore_ascii_case("rem") {
            Ok(Val::Rem(value))
        } else if unit.eq_ignore_ascii_case("em") {
            Ok(Val::Em(value))
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
                | (Self::MinContent, Self::MinContent)
                | (Self::MaxContent, Self::MaxContent)
                | (Self::FitContent, Self::FitContent)
                | (Self::Stretch, Self::Stretch)
                | (Self::Content, Self::Content)
                | (Self::Px(_), Self::Px(_))
                | (Self::Percent(_), Self::Percent(_))
                | (Self::Vw(_), Self::Vw(_))
                | (Self::Vh(_), Self::Vh(_))
                | (Self::VMin(_), Self::VMin(_))
                | (Self::VMax(_), Self::VMax(_))
                | (Self::Em(_), Self::Em(_))
                | (Self::Rem(_), Self::Rem(_))
        );

        // as long as the units are the same comparing `inner`
        // is valid.
        let left = self.inner();
        let right = other.inner();

        match (same_unit, left, right) {
            (true, a, b) => a == b,
            // All zero-value variants are considered equal.
            (false, Some(a), Some(b)) => a == 0. && b == 0.,
            _ => false,
        }
    }
}

impl Val {
    pub const DEFAULT: Self = Self::Auto;

    /// Zero. In [`Val::Px`], but zero is the same in any unit.
    pub const ZERO: Self = Self::Px(0.0);

    /// The largest finite [`Val::Px`] value.
    pub const MAX: Self = Self::Px(f32::MAX);

    /// Returns the number this `Val` wraps, whatever its unit, or `None` for keyword variants.
    ///
    /// The number is meaningless without the variant it came from — `Val::Px(4.0)` and
    /// `Val::Rem(4.0)` both return `4.0`. Use [`Val::resolve`] for a length in pixels.
    const fn inner(self) -> Option<f32> {
        match self {
            Self::Px(v)
            | Self::Percent(v)
            | Self::Vw(v)
            | Self::Vh(v)
            | Self::VMin(v)
            | Self::VMax(v)
            | Self::Em(v)
            | Self::Rem(v) => Some(v),
            _ => None,
        }
    }

    /// Returns true if this `Val` is a non-numeric keyword value.
    pub const fn is_keyword(self) -> bool {
        matches!(
            self,
            Val::Auto
                | Val::MinContent
                | Val::MaxContent
                | Val::FitContent
                | Val::Stretch
                | Val::Content
        )
    }

    /// Returns a [`UiRect`] with its `left` equal to this value,
    /// and all other fields set to `Val::ZERO`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).left();
    ///
    /// assert_eq!(ui_rect.left, Val::Px(1.));
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn left(self) -> UiRect {
        UiRect::left(self)
    }

    /// Returns a [`UiRect`] with its `right` equal to this value,
    /// and all other fields set to `Val::ZERO`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).right();
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::Px(1.));
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn right(self) -> UiRect {
        UiRect::right(self)
    }

    /// Returns a [`UiRect`] with its `top` equal to this value,
    /// and all other fields set to `Val::ZERO`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).top();
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::Px(1.));
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn top(self) -> UiRect {
        UiRect::top(self)
    }

    /// Returns a [`UiRect`] with its `bottom` equal to this value,
    /// and all other fields set to `Val::ZERO`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).bottom();
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::Px(1.));
    /// ```
    pub const fn bottom(self) -> UiRect {
        UiRect::bottom(self)
    }

    /// Returns a [`UiRect`] with all its fields equal to this value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).all();
    ///
    /// assert_eq!(ui_rect.left, Val::Px(1.));
    /// assert_eq!(ui_rect.right, Val::Px(1.));
    /// assert_eq!(ui_rect.top, Val::Px(1.));
    /// assert_eq!(ui_rect.bottom, Val::Px(1.));
    /// ```
    pub const fn all(self) -> UiRect {
        UiRect::all(self)
    }

    /// Returns a [`UiRect`] with all its `left` and `right` equal to this value,
    /// and its `top` and `bottom` set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).horizontal();
    ///
    /// assert_eq!(ui_rect.left, Val::Px(1.));
    /// assert_eq!(ui_rect.right, Val::Px(1.));
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn horizontal(self) -> UiRect {
        UiRect::horizontal(self)
    }

    /// Returns a [`UiRect`] with all its `top` and `bottom` equal to this value,
    /// and its `left` and `right` set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = Val::Px(1.).vertical();
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::Px(1.));
    /// assert_eq!(ui_rect.bottom, Val::Px(1.));
    /// ```
    pub const fn vertical(self) -> UiRect {
        UiRect::vertical(self)
    }

    /// Try to add two `Val`s.
    ///
    /// Returns [`ValArithmeticError::IncompatibleUnits`] if the units differ and [`ValArithmeticError::NonEvaluable`] if
    /// either `Val` is a keyword.
    ///
    /// ```
    /// # use bevy_ui::{Val, ValArithmeticError};
    /// assert_eq!(Val::Px(1.).try_add(Val::Px(2.)), Ok(Val::Px(3.)));
    /// assert_eq!(
    ///     Val::Px(1.).try_add(Val::Percent(2.)),
    ///     Err(ValArithmeticError::IncompatibleUnits)
    /// );
    /// ```
    pub const fn try_add(self, val: Val) -> Result<Val, ValArithmeticError> {
        match (self, val) {
            (Val::Px(u), Val::Px(v)) => Ok(Val::Px(u + v)),
            (Val::Percent(u), Val::Percent(v)) => Ok(Val::Percent(u + v)),
            (Val::Vw(u), Val::Vw(v)) => Ok(Val::Vw(u + v)),
            (Val::Vh(u), Val::Vh(v)) => Ok(Val::Vh(u + v)),
            (Val::VMin(u), Val::VMin(v)) => Ok(Val::VMin(u + v)),
            (Val::VMax(u), Val::VMax(v)) => Ok(Val::VMax(u + v)),
            (Val::Em(u), Val::Em(v)) => Ok(Val::Em(u + v)),
            (Val::Rem(u), Val::Rem(v)) => Ok(Val::Rem(u + v)),
            (a, b) if a.is_keyword() || b.is_keyword() => Err(ValArithmeticError::NonEvaluable),
            _ => Err(ValArithmeticError::IncompatibleUnits),
        }
    }

    /// Try to subtract one `Val` from another.
    ///
    /// Returns [`ValArithmeticError::IncompatibleUnits`] if the units differ and [`ValArithmeticError::NonEvaluable`] if
    /// either `Val` is a keyword.
    ///
    /// ```
    /// # use bevy_ui::{Val, ValArithmeticError};
    /// assert_eq!(Val::Px(3.).try_sub(Val::Px(2.)), Ok(Val::Px(1.)));
    /// assert_eq!(
    ///     Val::Px(1.).try_sub(Val::Percent(2.)),
    ///     Err(ValArithmeticError::IncompatibleUnits)
    /// );
    /// ```
    pub const fn try_sub(self, val: Val) -> Result<Val, ValArithmeticError> {
        match (self, val) {
            (Val::Px(u), Val::Px(v)) => Ok(Val::Px(u - v)),
            (Val::Percent(u), Val::Percent(v)) => Ok(Val::Percent(u - v)),
            (Val::Vw(u), Val::Vw(v)) => Ok(Val::Vw(u - v)),
            (Val::Vh(u), Val::Vh(v)) => Ok(Val::Vh(u - v)),
            (Val::VMin(u), Val::VMin(v)) => Ok(Val::VMin(u - v)),
            (Val::VMax(u), Val::VMax(v)) => Ok(Val::VMax(u - v)),
            (Val::Em(u), Val::Em(v)) => Ok(Val::Em(u - v)),
            (Val::Rem(u), Val::Rem(v)) => Ok(Val::Rem(u - v)),
            (a, b) if a.is_keyword() || b.is_keyword() => Err(ValArithmeticError::NonEvaluable),
            _ => Err(ValArithmeticError::IncompatibleUnits),
        }
    }

    /// Try to multiply a `Val` by a scalar.
    ///
    /// Returns [`ValArithmeticError::NonEvaluable`] if the `Val` is a keyword.
    ///
    /// ```
    /// # use bevy_ui::{Val, ValArithmeticError};
    /// assert_eq!(Val::Px(3.).try_mul(2.), Ok(Val::Px(6.)));
    /// assert_eq!(Val::Auto.try_mul(2.), Err(ValArithmeticError::NonEvaluable));
    /// ```
    pub const fn try_mul(self, scalar: f32) -> Result<Val, ValArithmeticError> {
        match self {
            Val::Px(value) => Ok(Val::Px(value * scalar)),
            Val::Percent(value) => Ok(Val::Percent(value * scalar)),
            Val::Vw(value) => Ok(Val::Vw(value * scalar)),
            Val::Vh(value) => Ok(Val::Vh(value * scalar)),
            Val::VMin(value) => Ok(Val::VMin(value * scalar)),
            Val::VMax(value) => Ok(Val::VMax(value * scalar)),
            Val::Em(value) => Ok(Val::Em(value * scalar)),
            Val::Rem(value) => Ok(Val::Rem(value * scalar)),
            _ => Err(ValArithmeticError::NonEvaluable),
        }
    }

    /// Try to divide a `Val` by a scalar.
    ///
    /// Returns [`ValArithmeticError::NonEvaluable`] if the `Val` is a keyword.
    ///
    /// ```
    /// # use bevy_ui::{Val, ValArithmeticError};
    /// assert_eq!(Val::Px(6.).try_div(2.), Ok(Val::Px(3.)));
    /// assert_eq!(Val::Auto.try_div(2.), Err(ValArithmeticError::NonEvaluable));
    /// ```
    pub const fn try_div(self, scalar: f32) -> Result<Val, ValArithmeticError> {
        match self {
            Val::Px(value) => Ok(Val::Px(value / scalar)),
            Val::Percent(value) => Ok(Val::Percent(value / scalar)),
            Val::Vw(value) => Ok(Val::Vw(value / scalar)),
            Val::Vh(value) => Ok(Val::Vh(value / scalar)),
            Val::VMin(value) => Ok(Val::VMin(value / scalar)),
            Val::VMax(value) => Ok(Val::VMax(value / scalar)),
            Val::Em(value) => Ok(Val::Em(value / scalar)),
            Val::Rem(value) => Ok(Val::Rem(value / scalar)),
            _ => Err(ValArithmeticError::NonEvaluable),
        }
    }
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
            Val::Px(value) => Val::Px(value * rhs),
            Val::Percent(value) => Val::Percent(value * rhs),
            Val::Vw(value) => Val::Vw(value * rhs),
            Val::Vh(value) => Val::Vh(value * rhs),
            Val::VMin(value) => Val::VMin(value * rhs),
            Val::VMax(value) => Val::VMax(value * rhs),
            Val::Em(value) => Val::Em(value * rhs),
            Val::Rem(value) => Val::Rem(value * rhs),
            keyword => {
                panic!("attempt to multiply non-numeric `Val::Val::{keyword:?}` by a scalar")
            }
        }
    }
}

impl MulAssign<f32> for Val {
    fn mul_assign(&mut self, rhs: f32) {
        match self {
            Val::Px(value)
            | Val::Percent(value)
            | Val::Vw(value)
            | Val::Vh(value)
            | Val::VMin(value)
            | Val::VMax(value)
            | Val::Em(value)
            | Val::Rem(value) => *value *= rhs,
            keyword => {
                panic!("attempt to multiply keyword `Val::{keyword:?}` by a scalar")
            }
        }
    }
}

impl Div<f32> for Val {
    type Output = Val;

    fn div(self, rhs: f32) -> Self::Output {
        match self {
            Val::Px(value) => Val::Px(value / rhs),
            Val::Percent(value) => Val::Percent(value / rhs),
            Val::Vw(value) => Val::Vw(value / rhs),
            Val::Vh(value) => Val::Vh(value / rhs),
            Val::VMin(value) => Val::VMin(value / rhs),
            Val::VMax(value) => Val::VMax(value / rhs),
            Val::Em(value) => Val::Em(value / rhs),
            Val::Rem(value) => Val::Rem(value / rhs),
            keyword => {
                panic!("attempt to divide keyword `Val::{keyword:?}` by a scalar")
            }
        }
    }
}

impl DivAssign<f32> for Val {
    fn div_assign(&mut self, rhs: f32) {
        match self {
            Val::Px(value)
            | Val::Percent(value)
            | Val::Vw(value)
            | Val::Vh(value)
            | Val::VMin(value)
            | Val::VMax(value)
            | Val::Em(value)
            | Val::Rem(value) => *value /= rhs,
            keyword => {
                panic!("attempt to divide keyword `Val::{keyword:?}` by a scalar")
            }
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
            Val::Em(value) => Val::Em(-value),
            Val::Rem(value) => Val::Rem(-value),
            keyword => panic!("attempt to negate keyword `Val::{keyword:?}`"),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Error)]
pub enum ValArithmeticError {
    #[error("the given variant of Val is not evaluable (a keyword)")]
    NonEvaluable,
    #[error("the given variants of Val have incompatible units")]
    IncompatibleUnits,
}

impl Val {
    /// Resolves this [`Val`] to a value in physical pixels from the given context and `physical_base_value`.
    ///
    /// Returns a [`ValArithmeticError::NonEvaluable`] if the [`Val`] is impossible to resolve into a concrete value.
    pub const fn resolve(
        self,
        context: &ValContext,
        physical_base_value: f32,
    ) -> Result<f32, ValArithmeticError> {
        match self {
            Val::Percent(value) => Ok(physical_base_value * value / 100.0),
            Val::Px(value) => Ok(value * context.scale_factor),
            Val::Vw(value) => Ok(context.physical_size.x * value / 100.0),
            Val::Vh(value) => Ok(context.physical_size.y * value / 100.0),
            Val::VMin(value) => {
                Ok(context.physical_size.x.min(context.physical_size.y) * value / 100.0)
            }
            Val::VMax(value) => {
                Ok(context.physical_size.x.max(context.physical_size.y) * value / 100.0)
            }
            Val::Em(value) => Ok(value * context.em_size * context.scale_factor),
            Val::Rem(value) => Ok(value * context.rem_size * context.scale_factor),
            _ => Err(ValArithmeticError::NonEvaluable),
        }
    }
}

impl From<Val> for FontSize {
    /// Converts a [`Val`] into a [`FontSize`].
    ///
    /// [`Val::Em`] becomes [`FontSize::Rem`]: em is relative to the font size of the node
    /// itself, which is the very thing being defined here, so it resolves against the root
    /// size instead to avoid the circularity.
    fn from(val: Val) -> Self {
        match val {
            Val::Px(px) => FontSize::Px(px),
            Val::Percent(percent) => FontSize::Rem(percent / 100.),
            Val::Vw(vw) => FontSize::Vw(vw),
            Val::Vh(vh) => FontSize::Vh(vh),
            Val::VMin(vmin) => FontSize::VMin(vmin),
            Val::VMax(vmax) => FontSize::VMax(vmax),
            Val::Rem(rem) => FontSize::Rem(rem),
            Val::Em(em) => FontSize::Rem(em), // see comment above
            _ => FontSize::Rem(1.),
        }
    }
}

impl TryStableInterpolate for Val {
    type Error = ValArithmeticError;

    /// # Example
    ///
    /// ```
    /// # use bevy_ui::Val;
    /// # use bevy_math::TryStableInterpolate;
    /// assert!(matches!(Val::Px(0.0).try_interpolate_stable(&Val::Px(10.0), 0.5), Ok(Val::Px(5.0))));
    /// ```
    fn try_interpolate_stable(&self, other: &Self, t: f32) -> Result<Self, Self::Error> {
        match (self, other) {
            (Val::Px(a), Val::Px(b)) => Ok(Val::Px(a.interpolate_stable(b, t))),
            (Val::Percent(a), Val::Percent(b)) => Ok(Val::Percent(a.interpolate_stable(b, t))),
            (Val::Vw(a), Val::Vw(b)) => Ok(Val::Vw(a.interpolate_stable(b, t))),
            (Val::Vh(a), Val::Vh(b)) => Ok(Val::Vh(a.interpolate_stable(b, t))),
            (Val::VMin(a), Val::VMin(b)) => Ok(Val::VMin(a.interpolate_stable(b, t))),
            (Val::VMax(a), Val::VMax(b)) => Ok(Val::VMax(a.interpolate_stable(b, t))),
            (Val::Em(a), Val::Em(b)) => Ok(Val::Em(a.interpolate_stable(b, t))),
            (Val::Rem(a), Val::Rem(b)) => Ok(Val::Rem(a.interpolate_stable(b, t))),
            (a, b) if a.is_keyword() | b.is_keyword() => Err(ValArithmeticError::NonEvaluable),
            _ => Err(ValArithmeticError::IncompatibleUnits),
        }
    }
}

/// All the types that should be able to be used in the [`Val`] enum should implement this trait.
///
/// Instead of just implementing `Into<Val>` a custom trait is added.
/// This is done in order to prevent having to define a default unit, which could lead to confusion especially for newcomers.
pub trait ValNum {
    /// Called by the [`Val`] helper functions to convert the implementing type to an `f32` that can
    /// be used by [`Val`].
    fn val_num_f32(self) -> f32;
}

macro_rules! impl_to_val_num {
    ($($impl_type:ty),*$(,)?) => {
        $(
            impl ValNum for $impl_type {
                fn val_num_f32(self) -> f32 {
                    self as f32
                }
            }
        )*
    };
}

impl_to_val_num!(f32, f64, i8, i16, i32, i64, u8, u16, u32, u64, usize, isize);

/// Returns a [`Val::Auto`] where the value is automatically determined
/// based on the context and other [`Node`](crate::Node) properties.
pub const fn auto() -> Val {
    Val::Auto
}

/// Returns a [`Val::Px`] representing a value in logical pixels.
pub fn px<T: ValNum>(value: T) -> Val {
    Val::Px(value.val_num_f32())
}

/// Returns a [`Val::Em`] representing a value proportional to the node's own font size, represented by an [`EmSize`] component.
pub fn em<T: ValNum>(value: T) -> Val {
    Val::Em(value.val_num_f32())
}

/// Returns a [`Val::Rem`] representing a value proportional to the root font size.
pub fn rem<T: ValNum>(value: T) -> Val {
    Val::Rem(value.val_num_f32())
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
pub fn percent<T: ValNum>(value: T) -> Val {
    Val::Percent(value.val_num_f32())
}

/// Returns a [`Val::Vw`] representing a percentage of the viewport width.
pub fn vw<T: ValNum>(value: T) -> Val {
    Val::Vw(value.val_num_f32())
}

/// Returns a [`Val::Vh`] representing a percentage of the viewport height.
pub fn vh<T: ValNum>(value: T) -> Val {
    Val::Vh(value.val_num_f32())
}

/// Returns a [`Val::VMin`] representing a percentage of the viewport's smaller dimension.
pub fn vmin<T: ValNum>(value: T) -> Val {
    Val::VMin(value.val_num_f32())
}

/// Returns a [`Val::VMax`] representing a percentage of the viewport's larger dimension.
pub fn vmax<T: ValNum>(value: T) -> Val {
    Val::VMax(value.val_num_f32())
}

#[cfg(test)]
mod tests {
    use bevy_math::vec2;

    use super::*;

    #[test]
    fn val_evaluate() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let em_size = EmSize(10.);
        let rem_size = RemSize(20.);
        let mut context = ValContext::new(1., viewport_size, em_size, rem_size);

        assert_eq!(
            Val::Percent(80.).resolve(&context, size).unwrap(),
            size * 0.8
        );
        // Check em and rem
        assert_eq!(Val::Em(2.).resolve(&context, size).unwrap(), em_size.0 * 2.);
        assert_eq!(
            Val::Rem(5.).resolve(&context, size).unwrap(),
            rem_size.0 * 5.
        );
        // Check scale_factor is considered
        context.scale_factor = 2.;
        assert_eq!(
            Val::Em(2.).resolve(&context, size).unwrap(),
            em_size.0 * 2. * 2.
        );
        assert_eq!(
            Val::Rem(5.).resolve(&context, size).unwrap(),
            rem_size.0 * 5. * 2.
        );
    }

    #[test]
    fn val_resolve_px() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let mut context = ValContext::new(1., viewport_size, EmSize(20.0), RemSize(20.0));

        let result = Val::Px(10.).resolve(&context, size).unwrap();
        assert_eq!(result, 10.);

        context.scale_factor = 3.;
        let result = Val::Px(10.).resolve(&context, size).unwrap();
        assert_eq!(result, 30.);
        context.scale_factor = 0.25;
        let result = Val::Px(10.).resolve(&context, size).unwrap();
        assert_eq!(result, 2.5);

        context.scale_factor = 1.;
        let result = Val::Em(1.).resolve(&context, size).unwrap();
        assert_eq!(result, 20.0);
        let result = Val::Rem(1.).resolve(&context, size).unwrap();
        assert_eq!(result, 20.0);
    }

    #[test]
    fn val_resolve_viewport_coords() {
        let size = 250.;
        let viewport_size = vec2(500., 500.);
        let context = ValContext::new(1., viewport_size, EmSize(20.0), RemSize(20.0));

        for value in (-10..10).map(|value| value as f32) {
            // for a square viewport there should be no difference between `Vw` and `Vh` and between `Vmin` and `Vmax`.
            assert_eq!(
                Val::Vw(value).resolve(&context, size),
                Val::Vh(value).resolve(&context, size)
            );
            assert_eq!(
                Val::VMin(value).resolve(&context, size),
                Val::VMax(value).resolve(&context, size)
            );
            assert_eq!(
                Val::VMin(value).resolve(&context, size),
                Val::Vw(value).resolve(&context, size)
            );
        }

        let viewport_size = vec2(1000., 500.);
        let context = ValContext::new(1., viewport_size, EmSize(20.0), RemSize(20.0));
        assert_eq!(Val::Vw(100.).resolve(&context, size).unwrap(), 1000.);
        assert_eq!(Val::Vh(100.).resolve(&context, size).unwrap(), 500.);
        assert_eq!(Val::Vw(60.).resolve(&context, size).unwrap(), 600.);
        assert_eq!(Val::Vh(40.).resolve(&context, size).unwrap(), 200.);
        assert_eq!(Val::VMin(50.).resolve(&context, size).unwrap(), 250.);
        assert_eq!(Val::VMax(75.).resolve(&context, size).unwrap(), 750.);
    }

    #[test]
    fn val_auto_is_non_evaluable() {
        let size = 250.;
        let viewport_size = vec2(1000., 500.);
        let context = ValContext::new(1., viewport_size, EmSize(20.0), RemSize(20.0));
        let resolve_auto = Val::Auto.resolve(&context, size);

        assert_eq!(resolve_auto, Err(ValArithmeticError::NonEvaluable));
    }

    #[test]
    fn val_try_add_compatible_variants() {
        assert_eq!(Val::Px(1.).try_add(Val::Px(2.)), Ok(Val::Px(3.)));
        assert_eq!(
            Val::Percent(1.).try_add(Val::Percent(2.)),
            Ok(Val::Percent(3.))
        );
        assert_eq!(Val::Vw(1.).try_add(Val::Vw(2.)), Ok(Val::Vw(3.)));
        assert_eq!(Val::Vh(1.).try_add(Val::Vh(2.)), Ok(Val::Vh(3.)));
        assert_eq!(Val::VMin(1.).try_add(Val::VMin(2.)), Ok(Val::VMin(3.)));
        assert_eq!(Val::VMax(1.).try_add(Val::VMax(2.)), Ok(Val::VMax(3.)));
    }

    #[test]
    fn val_try_add_incompatible_variants() {
        assert_eq!(
            Val::Px(1.).try_add(Val::Percent(2.)),
            Err(ValArithmeticError::IncompatibleUnits)
        );
    }

    #[test]
    fn val_try_add_keyword_is_non_evaluable() {
        assert_eq!(
            Val::Auto.try_add(Val::Auto),
            Err(ValArithmeticError::NonEvaluable)
        );
        assert_eq!(
            Val::Auto.try_add(Val::Px(1.)),
            Err(ValArithmeticError::NonEvaluable)
        );
        assert_eq!(
            Val::Px(1.).try_add(Val::MinContent),
            Err(ValArithmeticError::NonEvaluable)
        );
    }

    #[test]
    fn val_try_sub_compatible_variants() {
        assert_eq!(Val::Px(3.).try_sub(Val::Px(2.)), Ok(Val::Px(1.)));
        assert_eq!(
            Val::Percent(3.).try_sub(Val::Percent(2.)),
            Ok(Val::Percent(1.))
        );
        assert_eq!(Val::Vw(3.).try_sub(Val::Vw(2.)), Ok(Val::Vw(1.)));
        assert_eq!(Val::Vh(3.).try_sub(Val::Vh(2.)), Ok(Val::Vh(1.)));
        assert_eq!(Val::VMin(3.).try_sub(Val::VMin(2.)), Ok(Val::VMin(1.)));
        assert_eq!(Val::VMax(3.).try_sub(Val::VMax(2.)), Ok(Val::VMax(1.)));
    }

    #[test]
    fn val_try_sub_incompatible_variants() {
        assert_eq!(
            Val::Px(1.).try_sub(Val::Percent(2.)),
            Err(ValArithmeticError::IncompatibleUnits)
        );
    }

    #[test]
    fn val_try_sub_keyword_is_non_evaluable() {
        assert_eq!(
            Val::Auto.try_sub(Val::Auto),
            Err(ValArithmeticError::NonEvaluable)
        );
        assert_eq!(
            Val::Auto.try_sub(Val::Px(1.)),
            Err(ValArithmeticError::NonEvaluable)
        );
        assert_eq!(
            Val::Px(1.).try_sub(Val::MinContent),
            Err(ValArithmeticError::NonEvaluable)
        );
    }

    #[test]
    fn val_try_mul() {
        assert_eq!(Val::Px(3.).try_mul(2.), Ok(Val::Px(6.)));
        assert_eq!(Val::Percent(3.).try_mul(2.), Ok(Val::Percent(6.)));
        assert_eq!(Val::Rem(3.).try_mul(-1.), Ok(Val::Rem(-3.)));
        assert_eq!(Val::Auto.try_mul(2.), Err(ValArithmeticError::NonEvaluable));
        assert_eq!(
            Val::Stretch.try_mul(2.),
            Err(ValArithmeticError::NonEvaluable)
        );
    }

    #[test]
    fn val_try_div() {
        assert_eq!(Val::Px(6.).try_div(2.), Ok(Val::Px(3.)));
        assert_eq!(Val::Percent(6.).try_div(2.), Ok(Val::Percent(3.)));
        assert_eq!(Val::Em(6.).try_div(-2.), Ok(Val::Em(-3.)));
        assert_eq!(Val::Auto.try_div(2.), Err(ValArithmeticError::NonEvaluable));
        assert_eq!(
            Val::FitContent.try_div(2.),
            Err(ValArithmeticError::NonEvaluable)
        );
    }

    #[test]
    fn val_is_keyword() {
        for keyword in [
            Val::Auto,
            Val::MinContent,
            Val::MaxContent,
            Val::FitContent,
            Val::Stretch,
            Val::Content,
        ] {
            assert!(keyword.is_keyword());
        }
        for numeric in [
            Val::Px(1.),
            Val::Percent(1.),
            Val::Vw(1.),
            Val::Vh(1.),
            Val::VMin(1.),
            Val::VMax(1.),
            Val::Em(1.),
            Val::Rem(1.),
        ] {
            assert!(!numeric.is_keyword());
        }
    }

    #[test]
    fn val_numeric_operators() {
        assert_eq!(Val::Px(3.) * 2., Val::Px(6.));
        assert_eq!(Val::Px(6.) / 2., Val::Px(3.));
        assert_eq!(-Val::Px(3.), Val::Px(-3.));
        let mut val = Val::Percent(3.);
        val *= 2.;
        assert_eq!(val, Val::Percent(6.));
        val /= 3.;
        assert_eq!(val, Val::Percent(2.));
    }

    #[test]
    #[should_panic]
    fn val_mul_keyword_panics() {
        let _ = Val::Auto * 2.;
    }

    #[test]
    #[should_panic]
    fn val_mul_assign_keyword_panics() {
        let mut val = Val::Auto;
        val *= 2.;
    }

    #[test]
    #[should_panic]
    fn val_div_keyword_panics() {
        let _ = Val::Auto / 2.;
    }

    #[test]
    #[should_panic]
    fn val_div_assign_keyword_panics() {
        let mut val = Val::Auto;
        val /= 2.;
    }

    #[test]
    #[should_panic]
    fn val_neg_keyword_panics() {
        let _ = -Val::Auto;
    }

    #[test]
    fn val_try_interpolate_stable() {
        assert_eq!(
            Val::Px(0.).try_interpolate_stable(&Val::Px(10.), 0.5),
            Ok(Val::Px(5.))
        );
        assert_eq!(
            Val::Px(0.).try_interpolate_stable(&Val::Percent(10.), 0.5),
            Err(ValArithmeticError::IncompatibleUnits)
        );
        assert_eq!(
            Val::Auto.try_interpolate_stable(&Val::Px(10.), 0.5),
            Err(ValArithmeticError::NonEvaluable)
        );
        assert_eq!(
            Val::Auto.try_interpolate_stable(&Val::Auto, 0.5),
            Err(ValArithmeticError::NonEvaluable)
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

        assert_eq!("3em".parse::<Val>(), Ok(Val::Em(3.)));
        assert_eq!("3rem".parse::<Val>(), Ok(Val::Rem(3.)));

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
}
