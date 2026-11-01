use bevy_math::{StableInterpolate as _, Vec2};
use bevy_platform::sync::Arc;
use bevy_reflect::Reflect;
use bevy_text::{EmSize, RemSize};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use crate::{Val, ValArithmeticError};

/// Wrapper around an opteration on `Val`s.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(opaque, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct Calc(pub(crate) Arc<CalcOp>);

/// An operation performed on `Val`s
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum CalcOp {
    /// Add two vals.
    Add(Val, Val),
    /// Subtract the second `Val` from the first.
    Sub(Val, Val),
    /// Return the minimum of the two `Val`s.
    Min(Val, Val),
    /// Return the maximum of the two `Val`s.
    Max(Val, Val),
    /// Clamp the second `Val` between the first and third.
    Clamp(Val, Val, Val),
}

impl CalcOp {
    pub(crate) fn uses_percent(&self) -> bool {
        match self {
            Self::Add(a, b) | Self::Sub(a, b) | Self::Min(a, b) | Self::Max(a, b) => {
                a.uses_percent() || b.uses_percent()
            }
            Self::Clamp(min, value, max) => {
                min.uses_percent() || value.uses_percent() || max.uses_percent()
            }
        }
    }

    pub fn resolve(
        &self,
        scale_factor: f32,
        physical_base_value: f32,
        physical_target_size: Vec2,
        em_size: EmSize,
        rem_size: RemSize,
    ) -> Result<f32, ValArithmeticError> {
        let resolve = |val: &Val| {
            val.resolve(
                scale_factor,
                physical_base_value,
                physical_target_size,
                em_size,
                rem_size,
            )
        };
        Ok(match self {
            Self::Add(a, b) => resolve(a)? + resolve(b)?,
            Self::Sub(a, b) => resolve(a)? - resolve(b)?,
            Self::Min(a, b) => resolve(a)?.min(resolve(b)?),
            Self::Max(a, b) => resolve(a)?.max(resolve(b)?),
            Self::Clamp(min, value, max) => resolve(min)?.max(resolve(value)?.min(resolve(max)?)),
        })
    }

    pub(crate) fn scale(&mut self, k: f32) {
        use core::mem::take;
        match self {
            Self::Min(a, b) if k < 0.0 => *self = Self::Max(take(a) * k, take(b) * k),
            Self::Max(a, b) if k < 0.0 => *self = Self::Min(take(a) * k, take(b) * k),
            Self::Clamp(min, value, max) if k < 0.0 => {
                *self = Self::Min(
                    take(min) * k,
                    Self::Max(take(value) * k, take(max) * k).into(),
                );
            }
            Self::Add(a, b) | Self::Sub(a, b) | Self::Min(a, b) | Self::Max(a, b) => {
                *a *= k;
                *b *= k;
            }
            Self::Clamp(min, value, max) => {
                *min *= k;
                *value *= k;
                *max *= k;
            }
        }
    }
}

impl From<CalcOp> for Val {
    fn from(op: CalcOp) -> Self {
        Self::Calc(Calc(Arc::new(op)))
    }
}

/// Discriminator enum, should match `Val`'s unit types.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum ValUnit {
    /// Use `Px` units
    Px,
    /// Use `Percent` units
    Percent,
    /// Use `Vw` units
    Vw,
    Vh,
    VMin,
    VMax,
    Em,
    Rem,
}

impl ValUnit {
    /// Every unit, in discriminant order.
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

    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// If more unit types are added to `Val`, a `u8` discriminator won't be sufficient.
const _: () = assert!(ValUnit::ALL.len() <= u8::BITS as usize);

/// A set of [`ValUnit`]s.
///
/// [`Val::Sum`], [`Val::Min`] and [`Val::Max`] pair one of these with an array of values:
/// the `n`th value belongs to the `n`th unit in [`ValUnitSet::iter`] order, and the
/// slots after the last unit are unused.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct ValUnitSet(u8);

impl ValUnitSet {
    /// The empty set.
    pub const EMPTY: Self = Self(0);

    /// The number of units in the set.
    pub const fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    /// Whether the set has no units.
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Whether `unit` is in the set.
    pub const fn contains(&self, unit: ValUnit) -> bool {
        self.0 & unit.bit() != 0
    }

    /// The position `unit` has, or would have, among the set's units in ascending order.
    pub const fn slot(&self, unit: ValUnit) -> usize {
        (self.0 & (unit.bit() - 1)).count_ones() as usize
    }

    /// The units in ascending order.
    pub fn iter(self) -> impl Iterator<Item = ValUnit> {
        ValUnit::ALL
            .into_iter()
            .filter(move |unit| self.contains(*unit))
    }

    const fn insert(&mut self, unit: ValUnit) {
        self.0 |= unit.bit();
    }
}

impl core::fmt::Debug for ValUnitSet {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// Working copy of the terms of a [`Val::Sum`], [`Val::Min`] or [`Val::Max`].
#[derive(Clone, Copy)]
pub(crate) struct ValTerms {
    units: ValUnitSet,
    values: [f32; 3],
}

impl ValTerms {
    /// The maximum number of terms.
    pub(crate) const CAPACITY: usize = 3;

    pub(crate) const fn new(units: ValUnitSet, values: [f32; 3]) -> Self {
        Self { units, values }
    }

    pub(crate) const fn into_parts(self) -> (ValUnitSet, [f32; 3]) {
        (self.units, self.values)
    }

    const fn single(unit: ValUnit, value: f32) -> Self {
        let mut units = ValUnitSet::EMPTY;
        units.insert(unit);
        Self::new(units, [value, 0.0, 0.0])
    }

    /// The terms in ascending unit order.
    pub(crate) fn terms(self) -> impl Iterator<Item = (ValUnit, f32)> {
        self.units.iter().zip(self.values)
    }

    /// Folds `value` into the existing term for `unit` with `op`, or inserts it as a new term.
    ///
    /// Returns `false` and leaves `self` unchanged when a new term would exceed [`Self::CAPACITY`].
    fn fold(&mut self, unit: ValUnit, value: f32, op: fn(f32, f32) -> f32) -> bool {
        let slot = self.units.slot(unit);
        if self.units.contains(unit) {
            self.values[slot] = op(self.values[slot], value);
        } else if self.units.len() < Self::CAPACITY {
            self.values[slot..].rotate_right(1);
            self.values[slot] = value;
            self.units.insert(unit);
        } else {
            return false;
        }
        true
    }

    /// Removes the terms whose value is zero.
    fn retain_nonzero(&mut self) {
        let mut kept = Self::new(ValUnitSet::EMPTY, [0.0; 3]);
        for (unit, value) in self.terms().filter(|(_, value)| *value != 0.0) {
            kept.values[kept.units.len()] = value;
            kept.units.insert(unit);
        }
        *self = kept;
    }

    pub(crate) fn resolve(
        &self,
        op: fn(f32, f32) -> f32,
        scale_factor: f32,
        physical_base_value: f32,
        physical_target_size: Vec2,
        em_size: EmSize,
        rem_size: RemSize,
    ) -> f32 {
        self.terms()
            .map(|(unit, value)| {
                Val::leaf(unit, value)
                    .resolve(
                        scale_factor,
                        physical_base_value,
                        physical_target_size,
                        em_size,
                        rem_size,
                    )
                    .unwrap_or(0.0)
            })
            .reduce(op)
            .unwrap_or(0.0)
    }

    /// Interpolates term-wise, or `None` when the units differ.
    pub(crate) fn interpolate_stable(&self, other: &Self, t: f32) -> Option<Self> {
        (self.units == other.units).then(|| {
            let mut result = *self;
            for (value, other) in result.values.iter_mut().zip(other.values) {
                *value = value.interpolate_stable(&other, t);
            }
            result
        })
    }
}

impl PartialEq for ValTerms {
    fn eq(&self, other: &Self) -> bool {
        self.units == other.units && self.terms().eq(other.terms())
    }
}

/// The operation a [`Val::Sum`], [`Val::Min`] or [`Val::Max`] applies to its terms.
#[derive(Clone, Copy)]
pub(crate) enum TermOp {
    Sum,
    Min,
    Max,
}

impl TermOp {
    pub(crate) fn apply(self) -> fn(f32, f32) -> f32 {
        match self {
            Self::Sum => |a, b| a + b,
            Self::Min => f32::min,
            Self::Max => f32::max,
        }
    }

    /// The operation that gives the same result after every term is negated.
    const fn reversed(self) -> Self {
        match self {
            Self::Sum => Self::Sum,
            Self::Min => Self::Max,
            Self::Max => Self::Min,
        }
    }

    /// The [`Val`] for `terms` under this operation, as a single unit value when there is one term.
    fn wrap(self, terms: ValTerms) -> Val {
        if terms.units.len() == 1
            && let Some((unit, value)) = terms.terms().next()
        {
            return Val::leaf(unit, value);
        }
        match self {
            Self::Sum => Val::Sum(terms.units, terms.values),
            Self::Min => Val::Min(terms.units, terms.values),
            Self::Max => Val::Max(terms.units, terms.values),
        }
    }

    fn calc(self, a: Val, b: Val) -> Val {
        match self {
            Self::Sum => CalcOp::Add(a, b),
            Self::Min => CalcOp::Min(a, b),
            Self::Max => CalcOp::Max(a, b),
        }
        .into()
    }
}

impl Val {
    /// The terms of a [`Val::Sum`], [`Val::Min`] or [`Val::Max`], or the single term of a
    /// unit value. Empty for [`Val::Auto`], [`Val::Clamp`] and [`Val::Calc`].
    pub fn terms(&self) -> impl Iterator<Item = (ValUnit, f32)> {
        let terms = match self {
            Val::Sum(units, values) | Val::Min(units, values) | Val::Max(units, values) => {
                ValTerms::new(*units, *values)
            }
            _ => match (self.unit(), self.inner()) {
                (Some(unit), Some(value)) => ValTerms::single(unit, value),
                _ => ValTerms::new(ValUnitSet::EMPTY, [0.0; 3]),
            },
        };
        terms.terms()
    }

    /// Combines two values with `op`, folding them into one set of terms when both sides are
    /// unit values or terms of the same operation, and building a [`Calc`] otherwise.
    pub(crate) fn combine(self, rhs: Val, op: TermOp) -> Val {
        if let (Some(mut terms), Some(rhs_terms)) = (self.terms_for(op), rhs.terms_for(op))
            && rhs_terms
                .terms()
                .all(|(unit, value)| terms.fold(unit, value, op.apply()))
        {
            return op.wrap(terms);
        }
        op.calc(self, rhs)
    }

    fn terms_for(&self, op: TermOp) -> Option<ValTerms> {
        match (self, op) {
            (Val::Sum(units, values), TermOp::Sum)
            | (Val::Min(units, values), TermOp::Min)
            | (Val::Max(units, values), TermOp::Max) => Some(ValTerms::new(*units, *values)),
            _ => Some(ValTerms::single(self.unit()?, self.inner()?)),
        }
    }

    /// The terms of a [`Val::Sum`] or unit value with zero terms removed, so that sums
    /// compare equal whatever zero terms they carry. `None` for other values.
    pub(crate) fn sum_terms(&self) -> Option<ValTerms> {
        let mut terms = match self {
            Val::Sum(units, values) => ValTerms::new(*units, *values),
            _ => ValTerms::single(self.unit()?, self.inner()?),
        };
        terms.retain_nonzero();
        Some(terms)
    }

    /// The inline clamp of `value` between `min` and `max`, when all three are unit values.
    pub(crate) fn clamp_inline(min: &Val, value: &Val, max: &Val) -> Option<Val> {
        let term = |val: &Val| Some((val.unit()?, val.inner()?));
        let [lo, v, hi] = [term(min)?, term(value)?, term(max)?];
        if lo.0 == v.0 && v.0 == hi.0 {
            return Some(Val::leaf(v.0, lo.1.max(v.1.min(hi.1))));
        }
        // Inverted bounds always resolve to the minimum, so an inline clamp with same-unit
        // bounds is guaranteed to have `min <= max`.
        if lo.0 == hi.0 && lo.1 > hi.1 {
            return Some(Val::leaf(lo.0, lo.1));
        }
        Some(Val::Clamp([lo.0, v.0, hi.0], [lo.1, v.1, hi.1]))
    }

    pub(crate) fn resolve_clamp(
        units: [ValUnit; 3],
        values: [f32; 3],
        scale_factor: f32,
        physical_base_value: f32,
        physical_target_size: Vec2,
        em_size: EmSize,
        rem_size: RemSize,
    ) -> f32 {
        let [lo, v, hi] = core::array::from_fn(|i| {
            Val::leaf(units[i], values[i])
                .resolve(
                    scale_factor,
                    physical_base_value,
                    physical_target_size,
                    em_size,
                    rem_size,
                )
                .unwrap_or(0.0)
        });
        lo.max(v.min(hi))
    }

    /// Applies `f` to every term of a [`Val::Sum`], [`Val::Min`], [`Val::Max`] or [`Val::Clamp`].
    /// `reverses` swaps `Min` and `Max` and the bounds of `Clamp`, for an `f` that reverses ordering.
    pub(crate) fn map_terms(self, f: impl Fn(f32) -> f32, reverses: bool) -> Self {
        let (op, units, mut values) = match self {
            Val::Sum(units, values) => (TermOp::Sum, units, values),
            Val::Min(units, values) => (TermOp::Min, units, values),
            Val::Max(units, values) => (TermOp::Max, units, values),
            Val::Clamp(units, values) => {
                let values = values.map(&f);
                if !reverses {
                    return Val::Clamp(units, values);
                }
                // `f` reverses order, so `f(max(lo, min(v, hi)))` is `min(f(lo), max(f(v), f(hi)))`.
                // With same-unit bounds `lo <= hi` holds, so that is the clamp with the bounds
                // swapped; mixed-unit bounds may invert at some bases, so keep the exact form.
                let [lo, v, hi] = core::array::from_fn(|i| Val::leaf(units[i], values[i]));
                if units[0] == units[2] {
                    return Val::Clamp(
                        [units[2], units[1], units[0]],
                        [values[2], values[1], values[0]],
                    );
                }
                return lo.min(v.max(hi));
            }
            other => return other,
        };
        for value in &mut values[..units.len()] {
            *value = f(*value);
        }
        let op = if reverses { op.reversed() } else { op };
        op.wrap(ValTerms::new(units, values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn val_fits_in_two_words() {
        assert!(size_of::<Val>() <= 16);
    }

    #[test]
    fn sums_fold_inline() {
        let sum = Val::Px(10.0) + Val::Percent(50.0);
        assert!(matches!(sum, Val::Sum(..)), "got {sum:?}");
        assert_eq!(
            sum.terms().collect::<Vec<_>>(),
            [(ValUnit::Px, 10.0), (ValUnit::Percent, 50.0)]
        );
        assert_eq!(Val::Percent(50.0) + Val::Px(10.0), sum);
        assert_eq!(Val::Px(1.0) + Val::Px(2.0), Val::Px(3.0));
        assert_eq!(sum.clone() - Val::Px(10.0), Val::Percent(50.0));
        assert_eq!(Val::Px(0.0) + Val::Percent(0.0), Val::ZERO);
        assert_ne!(sum, Val::Px(10.0));
        assert_eq!(
            (Val::Px(1.0) + Val::Percent(2.0)) + (Val::Px(3.0) + Val::Em(4.0)),
            Val::Em(4.0) + Val::Px(4.0) + Val::Percent(2.0)
        );
        assert!(matches!(sum + Val::Em(1.0) + Val::Rem(1.0), Val::Calc(_)));
    }

    #[test]
    fn min_and_max_fold_inline() {
        assert_eq!(Val::Px(10.0).min(Val::Px(20.0)), Val::Px(10.0));
        assert_eq!(Val::Px(10.0).max(Val::Px(20.0)), Val::Px(20.0));
        let min = Val::Px(10.0).min(Val::Percent(50.0));
        assert!(matches!(min, Val::Min(..)));
        assert_eq!(Val::Percent(50.0).min(Val::Px(10.0)), min);
        assert_eq!(-min.clone(), Val::Px(-10.0).max(Val::Percent(-50.0)));
        assert!(matches!(min.clone().max(Val::Px(5.0)), Val::Calc(_)));
        let resolve =
            |value: &Val| value.resolve(1.0, 100.0, Vec2::ZERO, EmSize(0.0), RemSize(0.0));
        assert_eq!(resolve(&min), Ok(10.0));
        assert_eq!(resolve(&Val::Px(10.0).max(Val::Percent(50.0))), Ok(50.0));
        assert_eq!(resolve(&(min * -2.0)), Ok(-20.0));
    }

    #[test]
    fn negating_min_and_max_negates_their_results() {
        assert_eq!(-Val::Px(10.0).max(Val::Px(20.0)), Val::Px(-20.0));
        assert_eq!(-Val::Px(10.0).min(Val::Px(20.0)), Val::Px(-10.0));
        let resolve = |value: &Val, base: f32| {
            value.resolve(1.0, base, Vec2::ZERO, EmSize(0.0), RemSize(0.0))
        };
        for expr in [
            Val::Px(10.0).max(Val::Percent(50.0)),
            Val::Px(10.0).min(Val::Percent(50.0)),
        ] {
            for base in [0.0, 20.0, 100.0] {
                let expected = resolve(&expr, base).unwrap();
                assert_eq!(
                    resolve(&-expr.clone(), base),
                    Ok(-expected),
                    "{expr:?} at {base}"
                );
                assert_eq!(
                    resolve(&(expr.clone() * -2.0), base),
                    Ok(-2.0 * expected),
                    "{expr:?} at {base}"
                );
            }
        }
    }

    #[test]
    fn negating_a_clamp_negates_its_result() {
        let clamp = Val::Percent(50.0).clamp(Val::Px(100.0), Val::Percent(20.0));
        let resolve = |value: &Val, base: f32| {
            value.resolve(1.0, base, Vec2::ZERO, EmSize(0.0), RemSize(0.0))
        };
        for base in [50.0, 100.0, 150.0, 300.0, 1000.0] {
            let expected = resolve(&clamp, base).unwrap();
            assert_eq!(resolve(&-clamp.clone(), base), Ok(-expected), "base {base}");
            assert_eq!(
                resolve(&(clamp.clone() * -2.0), base),
                Ok(-2.0 * expected),
                "base {base}"
            );
            assert_eq!(
                resolve(&(Val::ZERO - clamp.clone()), base),
                Ok(-expected),
                "base {base}"
            );
        }
    }

    #[test]
    fn clamp_folds_inline() {
        use bevy_math::TryStableInterpolate;
        let clamp = Val::Percent(80.0).clamp(Val::Px(100.0), Val::Px(300.0));
        assert!(matches!(clamp, Val::Clamp(..)), "got {clamp:?}");
        assert_eq!(
            Val::Percent(80.0).clamp(Val::Px(300.0), Val::Px(100.0)),
            Val::Px(300.0)
        );
        let resolve = |value: &Val, base: f32| {
            value.resolve(1.0, base, Vec2::ZERO, EmSize(0.0), RemSize(0.0))
        };
        assert_eq!(resolve(&clamp, 100.0), Ok(100.0));
        assert_eq!(resolve(&clamp, 250.0), Ok(200.0));
        assert_eq!(resolve(&clamp, 1000.0), Ok(300.0));
        assert_eq!(
            -clamp.clone(),
            Val::Percent(-80.0).clamp(Val::Px(-300.0), Val::Px(-100.0))
        );
        assert_eq!(resolve(&(clamp.clone() * -2.0), 250.0), Ok(-400.0));
        assert_eq!(
            clamp.clone() * 2.0,
            Val::Percent(160.0).clamp(Val::Px(200.0), Val::Px(600.0))
        );
        assert_eq!(
            clamp
                .try_interpolate_stable(&Val::Percent(0.0).clamp(Val::Px(0.0), Val::Px(100.0)), 0.5)
                .ok(),
            Some(Val::Percent(40.0).clamp(Val::Px(50.0), Val::Px(200.0)))
        );
        assert!(matches!(
            (Val::Percent(80.0) + Val::Px(1.0)).clamp(Val::Px(100.0), Val::Px(300.0)),
            Val::Calc(_)
        ));
    }

    #[test]
    fn terms_interpolate_term_wise() {
        use bevy_math::TryStableInterpolate;
        let a = Val::Px(0.0) + Val::Percent(0.0);
        let b = Val::Px(10.0) + Val::Percent(100.0);
        assert_eq!(
            a.try_interpolate_stable(&b, 0.5).ok(),
            Some(Val::Px(5.0) + Val::Percent(50.0))
        );
        assert!(a
            .try_interpolate_stable(&(Val::Px(1.0) + Val::Em(1.0)), 0.5)
            .is_err());
    }

    #[test]
    fn resolve_expressions() {
        let value =
            Val::Percent(50.0) - Val::Px(10.0) + Val::Em(2.0) + Val::Rem(1.0) + Val::Vw(1.0);
        let resolve = |value: &Val| {
            value.resolve(
                2.0,
                400.0,
                Vec2::new(1000.0, 500.0),
                EmSize(12.0),
                RemSize(20.0),
            )
        };
        assert_eq!(resolve(&value), Ok(278.0));
        assert_eq!(resolve(&(value.clone() * 2.0)), Ok(556.0));
        assert_eq!(resolve(&(value.clone() / 2.0)), Ok(139.0));
        assert_eq!(resolve(&-value.clone()), Ok(-278.0));
        assert_eq!(
            resolve(&CalcOp::Min(value.clone(), Val::Px(100.0)).into()),
            Ok(200.0)
        );
        assert_eq!(
            resolve(&CalcOp::Max(value.clone(), Val::Px(200.0)).into()),
            Ok(400.0)
        );
        assert_eq!(
            resolve(&CalcOp::Clamp(Val::Px(100.0), value.clone(), Val::Px(125.0)).into()),
            Ok(250.0)
        );
        assert_eq!(
            resolve(&CalcOp::Clamp(Val::Px(200.0), value.clone(), Val::Px(100.0)).into()),
            Ok(400.0)
        );
        let mut scaled = value;
        scaled *= 2.0;
        scaled /= 4.0;
        assert_eq!(resolve(&scaled), Ok(139.0));
        assert_eq!(
            resolve(&(Val::Auto + Val::Px(1.0))),
            Err(ValArithmeticError::NonEvaluable)
        );
    }

    #[test]
    fn negative_scale_swaps_min_and_max() {
        let resolve = |value: Val| value.resolve(1.0, 0.0, Vec2::ZERO, EmSize(0.0), RemSize(0.0));
        let (a, b) = (Val::Px(10.0), Val::Px(20.0));
        assert_eq!(
            resolve(-Val::from(CalcOp::Min(a.clone(), b.clone()))),
            Ok(-10.0)
        );
        assert_eq!(
            resolve(-Val::from(CalcOp::Max(a.clone(), b.clone()))),
            Ok(-20.0)
        );
        assert_eq!(
            resolve(-Val::from(CalcOp::Clamp(
                a.clone(),
                Val::Px(15.0),
                b.clone()
            ))),
            Ok(-15.0)
        );
        assert_eq!(
            resolve(-Val::from(CalcOp::Clamp(b, Val::Px(15.0), a))),
            Ok(-20.0)
        );
    }

    #[test]
    fn repeated_scaling_does_not_grow_the_tree() {
        let mut value = Val::Px(1.0) + Val::Px(2.0);
        for _ in 0..100_000 {
            value *= 2.0;
            value /= 2.0;
        }
        assert_eq!(value, Val::Px(1.0) + Val::Px(2.0));
    }

    #[test]
    fn clone_and_reflection_preserve_expressions() {
        use bevy_reflect::{FromReflect, PartialReflect, TypeRegistry};

        let value = Val::Percent(100.0) - Val::Px(24.0);
        let mut cloned = value.clone();
        assert_eq!(value, cloned);
        cloned *= 2.0;
        assert_ne!(value, cloned);
        assert_eq!(value, Val::Percent(100.0) - Val::Px(24.0));
        let mut registry = TypeRegistry::default();
        registry.register::<Val>();
        let dynamic = value.to_dynamic().unwrap();
        assert_eq!(Val::from_reflect(dynamic.as_ref()), Some(value));
    }

    #[test]
    fn font_size_rejects_expressions() {
        assert_eq!(
            bevy_text::FontSize::try_from(Val::Rem(2.0)),
            Ok(bevy_text::FontSize::Rem(2.0))
        );
        assert_eq!(
            bevy_text::FontSize::try_from(Val::Percent(100.0) - Val::Px(24.0)),
            Err(ValArithmeticError::NonEvaluable)
        );
    }
}
