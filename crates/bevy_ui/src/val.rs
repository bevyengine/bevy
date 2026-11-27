use bevy_math::{MismatchedUnitsError, StableInterpolate as _, TryStableInterpolate, Vec2};
use bevy_platform::sync::RwLock;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_text::{EmSize, FontSize, RemSize, DEFAULT_REM_SIZE_PX};
use core::cell::Cell;
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

/// The unit of a numeric [`Val`].
///
/// The variants should match the numeric variants of `Val`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum ValUnit {
    /// Use `Val::Px` units.
    Px = 0,
    /// Use `Val::Percent` units.
    Percent = 1,
    /// Use `Val::Vw` units.
    Vw = 2,
    /// Use `Val::Vh` units.
    Vh = 3,
    /// Use `Val::VMin` units.
    VMin = 4,
    /// Use `Val::VMax` units.
    VMax = 5,
    /// Use `Val::Em` units.
    Em = 6,
    /// Use `Val::Rem` units.
    Rem = 7,
}

impl ValUnit {
    /// List of all the units in discriminant order.
    pub const ALL: [Self; 8] = [
        Self::Px,
        Self::Percent,
        Self::Vw,
        Self::Vh,
        Self::VMin,
        Self::VMax,
        Self::Em,
        Self::Rem,
    ];

    /// Constructs a numeric [`Val`] using this unit and the given value.
    pub const fn with_value(self, value: f32) -> Val {
        match self {
            ValUnit::Percent => Val::Percent(value),
            ValUnit::Px => Val::Px(value),
            ValUnit::Vw => Val::Vw(value),
            ValUnit::Vh => Val::Vh(value),
            ValUnit::VMin => Val::VMin(value),
            ValUnit::VMax => Val::VMax(value),
            ValUnit::Em => Val::Em(value),
            ValUnit::Rem => Val::Rem(value),
        }
    }
}

impl core::str::FromStr for ValUnit {
    type Err = ValParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "%" => Ok(ValUnit::Percent),
            unit if unit.eq_ignore_ascii_case("px") => Ok(ValUnit::Px),
            unit if unit.eq_ignore_ascii_case("vw") => Ok(ValUnit::Vw),
            unit if unit.eq_ignore_ascii_case("vh") => Ok(ValUnit::Vh),
            unit if unit.eq_ignore_ascii_case("vmin") => Ok(ValUnit::VMin),
            unit if unit.eq_ignore_ascii_case("vmax") => Ok(ValUnit::VMax),
            unit if unit.eq_ignore_ascii_case("rem") => Ok(ValUnit::Rem),
            unit if unit.eq_ignore_ascii_case("em") => Ok(ValUnit::Em),
            _ => Err(ValParseError::InvalidUnit),
        }
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
    /// A length calculated by a function, see [`Val::calc`].
    Calc {
        /// The function used to calculate the length.
        calc: Calc,
        /// Multiplier applied to the function's result.
        scalar: f32,
    },
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
        } else if s.eq_ignore_ascii_case("min-content") || s.eq_ignore_ascii_case("mincontent") {
            return Ok(Val::MinContent);
        } else if s.eq_ignore_ascii_case("max-content") || s.eq_ignore_ascii_case("maxcontent") {
            return Ok(Val::MaxContent);
        } else if s.eq_ignore_ascii_case("fit-content") || s.eq_ignore_ascii_case("fitcontent") {
            return Ok(Val::FitContent);
        } else if s.eq_ignore_ascii_case("stretch") {
            return Ok(Val::Stretch);
        } else if s.eq_ignore_ascii_case("content") {
            return Ok(Val::Content);
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

        let unit = ValUnit::from_str(unit)?;

        Ok(unit.with_value(value))
    }
}

impl PartialEq for Val {
    fn eq(&self, other: &Self) -> bool {
        if let Self::Calc { calc: a, scalar: s } = self
            && let Self::Calc { calc: b, scalar: t } = other
        {
            return a == b && s == t;
        }

        match (self.unit_value(), other.unit_value()) {
            // All zero-value variants are considered equal.
            (Ok((u, a)), Ok((v, b))) => (u, a) == (v, b) || (a == 0. && b == 0.),
            (Err(_), Err(_)) => core::mem::discriminant(self) == core::mem::discriminant(other),
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

    /// Returns the unit and value of a numeric `Val`.
    pub const fn unit_value(self) -> Result<(ValUnit, f32), ValArithmeticError> {
        match self {
            Self::Px(v) => Ok((ValUnit::Px, v)),
            Self::Percent(v) => Ok((ValUnit::Percent, v)),
            Self::Vw(v) => Ok((ValUnit::Vw, v)),
            Self::Vh(v) => Ok((ValUnit::Vh, v)),
            Self::VMin(v) => Ok((ValUnit::VMin, v)),
            Self::VMax(v) => Ok((ValUnit::VMax, v)),
            Self::Em(v) => Ok((ValUnit::Em, v)),
            Self::Rem(v) => Ok((ValUnit::Rem, v)),
            _ => Err(ValArithmeticError::NonEvaluable),
        }
    }

    /// Returns the unit of this `Val` if it is a numeric variant.
    pub const fn unit(self) -> Result<ValUnit, ValArithmeticError> {
        match self.unit_value() {
            Ok((unit, _)) => Ok(unit),
            Err(err) => Err(err),
        }
    }

    /// Returns the number this `Val` wraps, whatever its unit, or an error if it's a non-numeric variant.
    ///
    /// The number is meaningless without the variant it came from — `Val::Px(4.0)` and
    /// `Val::Rem(4.0)` both return `4.0`. Use [`Val::resolve`] for a length in pixels.
    pub const fn value(self) -> Result<f32, ValArithmeticError> {
        match self.unit_value() {
            Ok((_, value)) => Ok(value),
            Err(err) => Err(err),
        }
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
    /// Returns [`ValArithmeticError::IncompatibleUnits`] if the units differ or both are `Val::Auto`.
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
        if let Ok((u, x)) = self.unit_value()
            && let Ok((v, y)) = val.unit_value()
            && (u as u8) == (v as u8)
        {
            Ok(u.with_value(x + y))
        } else {
            Err(ValArithmeticError::IncompatibleUnits)
        }
    }

    /// Try to subtract one `Val` from another.
    ///
    /// Returns [`ValArithmeticError::IncompatibleUnits`] if the units differ or both are `Val::Auto`.
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
        if let Ok((u, x)) = self.unit_value()
            && let Ok((v, y)) = val.unit_value()
            && (u as u8) == (v as u8)
        {
            Ok(u.with_value(x - y))
        } else {
            Err(ValArithmeticError::IncompatibleUnits)
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

    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl Mul<Val> for f32 {
    type Output = Val;

    fn mul(self, rhs: Val) -> Self::Output {
        rhs * self
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
            Val::Calc { scalar, .. } => *scalar *= rhs,
            _non_numeric => {}
        }
    }
}

impl Div<f32> for Val {
    type Output = Val;

    fn div(mut self, rhs: f32) -> Self::Output {
        self /= rhs;
        self
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
            Val::Calc { scalar, .. } => *scalar /= rhs,
            _non_numeric => {}
        }
    }
}

impl Neg for Val {
    type Output = Val;

    fn neg(self) -> Self::Output {
        match self.unit_value() {
            Ok((unit, value)) => unit.with_value(-value),
            Err(_) => self * -1.,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Error)]
pub enum ValArithmeticError {
    #[error("the given variant of Val is not evaluable (non-numeric)")]
    NonEvaluable,
    #[error("the given variants of Val have incompatible units")]
    IncompatibleUnits,
}

impl Val {
    /// Resolves this [`Val`] to a value in physical pixels from the given context and `physical_base_value`.
    ///
    /// Returns a [`ValArithmeticError::NonEvaluable`] if the [`Val`] is impossible to resolve into a concrete value.
    pub fn resolve(
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
            Val::Calc { calc, scalar } => Ok(calc.resolve(context, physical_base_value).0 * scalar),
            _ => Err(ValArithmeticError::NonEvaluable),
        }
    }

    /// Creates a `Val` from a function
    ///
    /// The function cannot capture its environment, and is lost if it is serialized.
    ///
    /// ```
    /// # use bevy_ui::Val;
    /// // A quarter of the viewport's width, clamped between 200 and 400 logical pixels.
    /// let sidebar_width = Val::calc(|ctx| ctx.vw(25.).clamp(200., 400.));
    ///
    /// // The parent's width minus a 1rem gutter on each side.
    /// let content_width = Val::calc(|ctx| ctx.percent(100.) - 2. * ctx.rem(1.));
    /// ```
    pub fn calc(f: fn(&CalcContext) -> f32) -> Val {
        Val::Calc {
            calc: Calc::new(f),
            scalar: 1.,
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
            Val::Calc { calc, scalar } => {
                FontSize::calc(move |ctx| {
                    calc.resolve(
                        &ValContext {
                            scale_factor: 1.,
                            physical_size: ctx.view,
                            em_size: ctx.rem,
                            rem_size: ctx.rem,
                        },
                        ctx.rem,
                    )
                    .0
                }) * scalar
            }
            _ => FontSize::Rem(1.),
        }
    }
}

impl TryStableInterpolate for Val {
    type Error = MismatchedUnitsError;

    /// # Example
    ///
    /// ```
    /// # use bevy_ui::Val;
    /// # use bevy_math::TryStableInterpolate;
    /// assert!(matches!(Val::Px(0.0).try_interpolate_stable(&Val::Px(10.0), 0.5), Ok(Val::Px(5.0))));
    /// ```
    fn try_interpolate_stable(&self, other: &Self, t: f32) -> Result<Self, Self::Error> {
        if let Ok((u, x)) = self.unit_value()
            && let Ok((v, y)) = other.unit_value()
            && u == v
        {
            Ok(u.with_value(x.interpolate_stable(&y, t)))
        } else if self == other {
            Ok(*self)
        } else {
            Err(MismatchedUnitsError)
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

static CALC_FUNCTIONS: RwLock<Vec<fn(&CalcContext) -> f32>> = RwLock::new(Vec::new());

/// The id of a function that calculates a length, see [`Val::calc`].
#[derive(Copy, Clone, PartialEq, Eq, Debug, Reflect)]
#[reflect(opaque, PartialEq, Debug, Clone)]
#[cfg_attr(feature = "serialize", reflect(Serialize, Deserialize))]
pub struct Calc(u16);

impl Calc {
    /// Registers a function and returns a 2-byte id that can be used to look it up.
    /// Storing the id instead of a function pointer keeps [`Val`] at 8 bytes.
    ///
    /// # Panics
    ///
    /// Panics if more than 65,536 different functions are registered.
    pub fn new(f: fn(&CalcContext) -> f32) -> Self {
        let mut functions = CALC_FUNCTIONS.write().unwrap();
        let index = match functions.iter().position(|g| core::ptr::fn_addr_eq(*g, f)) {
            Some(index) => index,
            None => {
                functions.push(f);
                functions.len() - 1
            }
        };
        Calc(u16::try_from(index).expect("too many calc functions"))
    }

    /// Resolves the `Calc` to a value in physical pixels, or zero if the result isn't finite.
    /// Returns the value, along with true if the function used the percentage basis or false otherwise.
    pub(crate) fn resolve(self, context: &ValContext, basis: f32) -> (f32, bool) {
        Self::resolve_function(self.get(), context, basis)
    }

    /// Returns the function corresponding to this id.
    pub(crate) fn get(self) -> fn(&CalcContext) -> f32 {
        CALC_FUNCTIONS.read().unwrap()[self.0 as usize]
    }

    /// Resolves `f` to a value in physical pixels, or zero if the result isn't finite.
    /// Returns the value, along with true if the function used the percentage basis or false otherwise.
    pub(crate) fn resolve_function(
        f: fn(&CalcContext) -> f32,
        context: &ValContext,
        basis: f32,
    ) -> (f32, bool) {
        let calc_context = CalcContext {
            context,
            basis,
            used_percent: Cell::new(false),
        };
        let value = f(&calc_context) * context.scale_factor;
        (
            if value.is_finite() { value } else { 0. },
            calc_context.used_percent.get(),
        )
    }
}

#[cfg(feature = "serialize")]
impl serde::Serialize for Calc {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        bevy_log::warn_once!(
            "A calc function can't be serialized, it will be deserialized as a function returning zero."
        );
        serializer.serialize_unit()
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for Calc {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <()>::deserialize(deserializer)?;
        bevy_log::warn_once!("Deserialized a calc without a function. It will return zero.");
        Ok(Calc::new(|_| 0.))
    }
}

/// Converts lengths in each [`Val`] unit to logical pixels, passed to the function of a [`Val::calc`].
pub struct CalcContext<'a> {
    context: &'a ValContext,
    basis: f32,
    used_percent: Cell<bool>,
}

impl CalcContext<'_> {
    /// Length in logical pixels as a percentage of the parent node's length, see [`Val::Percent`].
    pub fn percent(&self, value: f32) -> f32 {
        self.used_percent.set(true);
        self.basis * value / 100. / self.context.scale_factor
    }

    /// Length in logical pixels as a percentage of the viewport width.
    pub fn vw(&self, value: f32) -> f32 {
        self.context.physical_size.x * value / 100. / self.context.scale_factor
    }

    /// Length in logical pixels as a percentage of the viewport height.
    pub fn vh(&self, value: f32) -> f32 {
        self.context.physical_size.y * value / 100. / self.context.scale_factor
    }

    /// Length in logical pixels as a percentage of the viewport's smaller dimension.
    pub fn vmin(&self, value: f32) -> f32 {
        self.context.physical_size.min_element() * value / 100. / self.context.scale_factor
    }

    /// Length in logical pixels as a percentage of the viewport's larger dimension.
    pub fn vmax(&self, value: f32) -> f32 {
        self.context.physical_size.max_element() * value / 100. / self.context.scale_factor
    }

    /// Length in logical pixels as a multiple of the node's own font size.
    pub fn em(&self, value: f32) -> f32 {
        self.context.em_size * value
    }

    /// Length in logical pixels as a multiple of the root font size.
    pub fn rem(&self, value: f32) -> f32 {
        self.context.rem_size * value
    }
}

#[cfg(test)]
mod tests {
    use bevy_math::vec2;

    use super::*;

    #[test]
    fn val_numeric_equality() {
        assert_eq!(Val::Px(1.), Val::Px(1.));
        assert_ne!(Val::Px(1.), Val::Px(2.));
        assert_ne!(Val::Px(1.), Val::Percent(1.));
        assert_ne!(Val::Px(0.), Val::Percent(1.));
        assert_eq!(Val::Px(0.), Val::Percent(-0.));
        assert_eq!(Val::Px(f32::INFINITY), Val::Px(f32::INFINITY));
        assert_ne!(Val::Px(f32::NAN), Val::Px(f32::NAN));
    }

    #[test]
    fn val_non_numeric_equality() {
        let values = [
            Val::Auto,
            Val::MinContent,
            Val::MaxContent,
            Val::FitContent,
            Val::Stretch,
            Val::Content,
        ];

        for (i, left) in values.iter().enumerate() {
            for (j, right) in values.iter().enumerate() {
                assert_eq!(left == right, i == j);
            }
            for unit in ValUnit::ALL {
                assert_ne!(*left, unit.with_value(0.));
                assert_ne!(unit.with_value(0.), *left);
            }
        }
    }

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
            Val::Auto.try_add(Val::Auto),
            Err(ValArithmeticError::IncompatibleUnits)
        );
        assert_eq!(
            Val::Px(1.).try_add(Val::Percent(2.)),
            Err(ValArithmeticError::IncompatibleUnits)
        );
        assert_eq!(
            Val::Auto.try_add(Val::Px(1.)),
            Err(ValArithmeticError::IncompatibleUnits)
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
            Val::Auto.try_sub(Val::Auto),
            Err(ValArithmeticError::IncompatibleUnits)
        );
        assert_eq!(
            Val::Px(1.).try_sub(Val::Percent(2.)),
            Err(ValArithmeticError::IncompatibleUnits)
        );
        assert_eq!(
            Val::Auto.try_sub(Val::Px(1.)),
            Err(ValArithmeticError::IncompatibleUnits)
        );
    }

    #[test]
    fn val_str_parse() {
        assert_eq!("auto".parse::<Val>(), Ok(Val::Auto));
        assert_eq!("Auto".parse::<Val>(), Ok(Val::Auto));
        assert_eq!("AUTO".parse::<Val>(), Ok(Val::Auto));

        assert_eq!("min-content".parse::<Val>(), Ok(Val::MinContent));
        assert_eq!("MIN-CONTENT".parse::<Val>(), Ok(Val::MinContent));
        assert_eq!("mincontent".parse::<Val>(), Ok(Val::MinContent));
        assert_eq!("MinContent".parse::<Val>(), Ok(Val::MinContent));

        assert_eq!("max-content".parse::<Val>(), Ok(Val::MaxContent));
        assert_eq!("MAX-CONTENT".parse::<Val>(), Ok(Val::MaxContent));
        assert_eq!("maxcontent".parse::<Val>(), Ok(Val::MaxContent));
        assert_eq!("MaxContent".parse::<Val>(), Ok(Val::MaxContent));

        assert_eq!("fit-content".parse::<Val>(), Ok(Val::FitContent));
        assert_eq!("FIT-CONTENT".parse::<Val>(), Ok(Val::FitContent));
        assert_eq!("fitcontent".parse::<Val>(), Ok(Val::FitContent));
        assert_eq!("FitContent".parse::<Val>(), Ok(Val::FitContent));

        assert_eq!("stretch".parse::<Val>(), Ok(Val::Stretch));
        assert_eq!("Stretch".parse::<Val>(), Ok(Val::Stretch));
        assert_eq!("STRETCH".parse::<Val>(), Ok(Val::Stretch));

        assert_eq!("content".parse::<Val>(), Ok(Val::Content));
        assert_eq!("Content".parse::<Val>(), Ok(Val::Content));
        assert_eq!("CONTENT".parse::<Val>(), Ok(Val::Content));

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

    #[test]
    fn calc_resolves_in_logical_pixels() {
        let context = ValContext::new(2., vec2(1000., 500.), EmSize(10.), RemSize(20.));
        let calc = Val::calc(|ctx| 10. + ctx.percent(13.) + ctx.vh(20.) + ctx.em(1.) + ctx.rem(1.));
        assert_eq!(
            calc.resolve(&context, 200.),
            Ok(20. + 26. + 100. + 20. + 40.)
        );
    }

    #[test]
    fn calc_viewport_min_and_max() {
        let context = ValContext::new(2., vec2(1000., 500.), EmSize(10.), RemSize(20.));
        assert_eq!(Val::calc(|ctx| ctx.vmin(2.)).resolve(&context, 0.), Ok(10.));
        assert_eq!(Val::calc(|ctx| ctx.vmax(2.)).resolve(&context, 0.), Ok(20.));
    }

    #[test]
    fn calc_scales_and_negates() {
        let context = ValContext::TEST_CONTEXT;
        let calc = Val::calc(|ctx| 10. + ctx.percent(1.));
        assert_eq!((calc * 2.).resolve(&context, 200.), Ok(24.));
        assert_eq!((2. * calc).resolve(&context, 200.), Ok(24.));
        assert_eq!((calc / 2.).resolve(&context, 200.), Ok(6.));
        assert_eq!((-calc).resolve(&context, 200.), Ok(-12.));
        assert_ne!(calc * 2., calc);
    }

    #[test]
    fn calc_non_finite_results_are_zero() {
        let context = ValContext::TEST_CONTEXT;
        assert_eq!(Val::calc(|_| f32::NAN).resolve(&context, 0.), Ok(0.));
        assert_eq!(
            Val::calc(|ctx| 100. / ctx.percent(1.)).resolve(&context, 0.),
            Ok(0.)
        );
    }

    #[test]
    fn calc_cannot_be_added() {
        let calc = Val::calc(|_| 10.);
        assert_eq!(
            calc.try_add(Val::Px(1.)),
            Err(ValArithmeticError::IncompatibleUnits)
        );
        assert_eq!(
            calc.try_sub(calc),
            Err(ValArithmeticError::IncompatibleUnits)
        );
    }

    #[test]
    fn calc_equality_is_by_function() {
        let f: fn(&CalcContext) -> f32 = |_| 10.;
        assert_eq!(Val::calc(f), Val::calc(f));
        assert_ne!(Val::calc(f), Val::calc(|_| 20.));
        assert_ne!(Val::calc(f), Val::Px(10.));
        assert_ne!(Val::calc(f), Val::Auto);
    }

    #[test]
    fn calc_only_interpolates_with_itself() {
        let calc = Val::calc(|_| 10.);
        assert!(matches!(
            calc.try_interpolate_stable(&calc, 0.5),
            Ok(val) if val == calc
        ));
        assert!(calc.try_interpolate_stable(&Val::Px(10.), 0.5).is_err());
        assert!(Val::Px(10.).try_interpolate_stable(&calc, 0.5).is_err());
    }

    #[test]
    fn reflection_preserves_calc() {
        use bevy_reflect::{FromReflect, PartialReflect};

        let calc = Val::calc(|_| 10.);
        let dynamic = calc.to_dynamic().unwrap();
        assert_eq!(Val::from_reflect(dynamic.as_ref()), Some(calc));
        let cloned = calc.reflect_clone().unwrap();
        assert_eq!(cloned.downcast_ref::<Val>(), Some(&calc));
    }

    #[cfg(feature = "serialize")]
    #[test]
    fn calc_serialization_is_lossy() {
        let ron = ron::to_string(&Val::calc(|_| 10.)).unwrap();
        let calc = ron::from_str::<Val>(&ron).unwrap();
        assert!(matches!(calc, Val::Calc { .. }));
        assert_eq!(calc.resolve(&ValContext::TEST_CONTEXT, 0.), Ok(0.));
        let ron = ron::to_string(&Val::Percent(50.)).unwrap();
        assert_eq!(ron::from_str::<Val>(&ron).unwrap(), Val::Percent(50.));
    }

    #[cfg(feature = "serialize")]
    #[test]
    fn calc_reflect_serialization_is_lossy() {
        use bevy_reflect::serde::{ReflectDeserializer, ReflectSerializer};
        use bevy_reflect::{FromReflect, TypeRegistry};
        use serde::de::DeserializeSeed;

        let mut registry = TypeRegistry::default();
        registry.register::<Val>();
        let calc = Val::calc(|_| 10.);
        let ron = ron::to_string(&ReflectSerializer::new(&calc, &registry)).unwrap();
        let mut deserializer = ron::Deserializer::from_str(&ron).unwrap();
        let reflected = ReflectDeserializer::new(&registry)
            .deserialize(&mut deserializer)
            .unwrap();
        let calc = Val::from_reflect(reflected.as_ref()).unwrap();
        assert!(matches!(calc, Val::Calc { .. }));
        assert_eq!(calc.resolve(&ValContext::TEST_CONTEXT, 0.), Ok(0.));
    }

    #[test]
    fn val_is_eight_bytes() {
        assert_eq!(size_of::<Val>(), 8);
        assert_eq!(size_of::<crate::GridTrack>(), 16);
    }

    #[test]
    fn font_size_from_calc() {
        let font_size =
            FontSize::from(Val::calc(|ctx| ctx.percent(50.) + ctx.em(1.) + ctx.vh(10.)) * 2.);
        assert_eq!(
            font_size.eval(vec2(1000., 500.), RemSize(20.)),
            2. * (10. + 20. + 50.)
        );
    }

    #[test]
    fn val_unit_all_matches_discriminants() {
        for (i, u) in ValUnit::ALL.iter().enumerate() {
            assert!(*u as usize == i);
        }
    }

    #[test]
    fn val_unit_value_round_trip() {
        for val in [
            Val::Px(3.5),
            Val::Percent(3.5),
            Val::Vw(3.5),
            Val::Vh(3.5),
            Val::VMin(3.5),
            Val::VMax(3.5),
            Val::Em(3.5),
            Val::Rem(3.5),
        ] {
            let (unit, value) = val.unit_value().unwrap();
            assert_eq!(unit.with_value(value), val);
        }
    }
}
