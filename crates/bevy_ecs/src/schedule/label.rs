pub use bevy_ecs_macros::{AmbiguitySetLabel, RunCriteriaLabel, StageLabel, SystemLabel};

use std::{
    any::Any,
    borrow::Cow,
    fmt::Debug,
    hash::{Hash, Hasher},
};

pub trait DynEq: Any {
    fn as_any(&self) -> &dyn Any;

    fn dyn_eq(&self, other: &dyn DynEq) -> bool;
}

impl<T> DynEq for T
where
    T: Any + Eq,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn DynEq) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<T>() {
            return self == other;
        }
        false
    }
}

pub trait DynHash: DynEq {
    fn as_dyn_eq(&self) -> &dyn DynEq;

    fn dyn_hash(&self, state: &mut dyn Hasher);
}

impl<T> DynHash for T
where
    T: DynEq + Hash,
{
    fn as_dyn_eq(&self) -> &dyn DynEq {
        self
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        T::hash(self, &mut state);
        self.type_id().hash(&mut state);
    }
}

pub trait StageLabel: DynHash + Debug + Send + Sync + 'static {
    #[doc(hidden)]
    fn dyn_clone(&self) -> Box<dyn StageLabel>;
}
pub(crate) type BoxedStageLabel = Box<dyn StageLabel>;

pub trait SystemLabel: DynHash + Debug + Send + Sync + 'static {
    #[doc(hidden)]
    fn dyn_clone(&self) -> Box<dyn SystemLabel>;
}
pub(crate) type BoxedSystemLabel = Box<dyn SystemLabel>;

pub trait AmbiguitySetLabel: DynHash + Debug + Send + Sync + 'static {
    #[doc(hidden)]
    fn dyn_clone(&self) -> Box<dyn AmbiguitySetLabel>;
}
pub(crate) type BoxedAmbiguitySetLabel = Box<dyn AmbiguitySetLabel>;

pub trait RunCriteriaLabel: DynHash + Debug + Send + Sync + 'static {
    #[doc(hidden)]
    fn dyn_clone(&self) -> Box<dyn RunCriteriaLabel>;
}
pub(crate) type BoxedRunCriteriaLabel = Box<dyn RunCriteriaLabel>;

macro_rules! impl_label {
    ($trait_name:ident) => {
        impl PartialEq for dyn $trait_name {
            fn eq(&self, other: &Self) -> bool {
                self.dyn_eq(other.as_dyn_eq())
            }
        }

        impl Eq for dyn $trait_name {}

        impl Hash for dyn $trait_name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.dyn_hash(state);
            }
        }

        impl Clone for Box<dyn $trait_name> {
            fn clone(&self) -> Self {
                self.dyn_clone()
            }
        }

        impl $trait_name for Cow<'static, str> {
            fn dyn_clone(&self) -> Box<dyn $trait_name> {
                Box::new(self.clone())
            }
        }

        impl $trait_name for &'static str {
            fn dyn_clone(&self) -> Box<dyn $trait_name> {
                Box::new(<&str>::clone(self))
            }
        }
    };
}

impl_label!(StageLabel);
impl_label!(SystemLabel);
impl_label!(AmbiguitySetLabel);
impl_label!(RunCriteriaLabel);
