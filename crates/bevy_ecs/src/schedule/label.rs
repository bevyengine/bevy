use std::{
    any::Any,
    borrow::Cow,
    fmt::Debug,
    hash::{Hash, Hasher},
};

use crate::{StageLabelMarker, SystemLabelMarker};

pub trait Label<T>: DynHash + DynClone<T> + Send + Sync + 'static {
    fn name(&self) -> Cow<'static, str>;
}

impl<M: 'static> Debug for dyn Label<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.name())
    }
}

pub type SystemLabel = Box<dyn Label<SystemLabelMarker>>;
pub type StageLabel = Box<dyn Label<StageLabelMarker>>;

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

pub trait DynClone<T> {
    fn dyn_clone(&self) -> Box<dyn Label<T>>;
}

impl<M, T> DynClone<M> for T
where
    T: Label<M> + Clone + 'static,
{
    fn dyn_clone(&self) -> Box<dyn Label<M>> {
        Box::new(self.clone())
    }
}

impl<T> PartialEq for dyn Label<T> {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other.as_dyn_eq())
    }
}

impl<T> Eq for dyn Label<T> {}

impl<T> Hash for dyn Label<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}

impl<T> Clone for Box<dyn Label<T>> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}

impl<T: Label<StageLabelMarker>> From<T> for Box<dyn Label<StageLabelMarker>> {
    fn from(t: T) -> Self {
        Box::new(t)
    }
}

impl<T: Label<SystemLabelMarker>> From<T> for Box<dyn Label<SystemLabelMarker>> {
    fn from(t: T) -> Self {
        Box::new(t)
    }
}

impl Label<SystemLabelMarker> for Cow<'static, str> {
    fn name(&self) -> Cow<'static, str> {
        self.clone()
    }
}

impl Label<SystemLabelMarker> for &'static str {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(self)
    }
}
impl Label<StageLabelMarker> for Cow<'static, str> {
    fn name(&self) -> Cow<'static, str> {
        self.clone()
    }
}

impl Label<StageLabelMarker> for &'static str {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(self)
    }
}
