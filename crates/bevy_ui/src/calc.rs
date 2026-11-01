use bevy_math::Vec2;
use bevy_reflect::Reflect;
use bevy_text::{EmSize, RemSize};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use crate::{Val, ValArithmeticError};

#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(opaque, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct Calc(pub Box<CalcOps>);

#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum CalcOps {
    Add(Val, Val),
    Sub(Val, Val),
    Scale(Val, f32),
    Min(Val, Val),
    Max(Val, Val),
    Clamp(Val, Val, Val),
}

impl CalcOps {
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
            Self::Scale(value, _) => uses_percent(value),
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
            Self::Scale(value, scale) => resolve(value)? * scale,
            Self::Min(a, b) => resolve(a)?.min(resolve(b)?),
            Self::Max(a, b) => resolve(a)?.max(resolve(b)?),
            Self::Clamp(min, value, max) => resolve(min)?.max(resolve(value)?.min(resolve(max)?)),
        })
    }
}

impl From<CalcOps> for Val {
    fn from(op: CalcOps) -> Self {
        Self::Calc(Calc(Box::new(op)))
    }
}

#[cfg(te        let value =
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
            resolve(&CalcOps::Min(value.clone(), Val::Px(100.0)).into()),
            Ok(200.0)
        );
        assert_eq!(
            resolve(&CalcOps::Max(value.clone(), Val::Px(200.0)).into()),
            Ok(400.0)
        );
        assert_eq!(
            resolve(&CalcOps::Clamp(Val::Px(100.0), value.clone(), Val::Px(125.0)).into()),
            Ok(250.0)
        );
        assert_eq!(
            resolve(&CalcOps::Clamp(Val::Px(200.0), value.clone(), Val::Px(100.0)).into()),
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
k(139.0));
        assert_eq!(resolve(&(Val::Auto + Val::Px(1.0))), Err(ValArithmeticError::NonEvaluable));
    }

    #[test]
    fn clone_and_reflection_preserve_expressions() {
        use bevy_reflect::{FromReflect, PartialReflect, TypeRegistry};

        let value = Val::Percent(100.0) - Val::Px(24.0);
        let mut cloned = value.clone();
        assert_eq!(value, cloned);
        cloned *= 2.0;
        assert_ne!(value, cloned);
        assert_eq!(value, Val::Percent(100        assert_eq!(
            bevy_text::FontSize::try_from(Val::Rem(2.0)),
            Ok(bevy_text::FontSize::Rem(2.0))
        );
        assert_eq!(
            bevy_text::FontSize::try_from(value.clone()),
            Err(ValArithmeticError::NonEvaluable)
        );
.0) - Val::Px(24.0));
        let mut registry = TypeRegistry::default();
        registry.register::<Val>();
        let dynamic = value.to_dynamic().unwrap();
        assert_eq!(Val::from_reflect(dynamic.as_ref()), Some(value));
    }
}
