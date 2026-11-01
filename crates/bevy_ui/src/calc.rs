use std::sync::Arc;

use bevy_math::Vec2;
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
        let uses_percent = |val: &Val| match val {
            Val::Percent(_) => true,
            Val::Calc(calc) => calc.0.uses_percent(),
            _ => false,
        };
        match self {
            Self::Add(a, b) | Self::Sub(a, b) | Self::Min(a, b) | Self::Max(a, b) => {
                uses_percent(a) || uses_percent(b)
            }
            Self::Clamp(min, value, max) => {
                uses_percent(min) || uses_percent(value) || uses_percent(max)
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

#[cfg(test)]
mod tests {
    use super::*;

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
