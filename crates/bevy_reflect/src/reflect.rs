use crate::{serde::Serializable, List, Map, Struct, TupleStruct};
use std::{any::Any, fmt::Debug};

pub use bevy_utils::AHasher as ReflectHasher;

pub enum ReflectRef<'a> {
    Struct(&'a dyn Struct),
    TupleStruct(&'a dyn TupleStruct),
    List(&'a dyn List),
    Map(&'a dyn Map),
    Value(&'a dyn Reflect),
}

pub enum ReflectMut<'a> {
    Struct(&'a mut dyn Struct),
    TupleStruct(&'a mut dyn TupleStruct),
    List(&'a mut dyn List),
    Map(&'a mut dyn Map),
    Value(&'a mut dyn Reflect),
}

/// A reflected rust type.
pub trait Reflect: Any + Send + Sync {
    fn type_name(&self) -> &str;
    fn any(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
    fn apply(&mut self, value: &dyn Reflect);
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>;
    fn reflect_ref(&self) -> ReflectRef;
    fn reflect_mut(&mut self) -> ReflectMut;
    fn clone_value(&self) -> Box<dyn Reflect>;
    /// Returns a hash of the value (which includes the type) if hashing is supported. Otherwise `None` will be returned.
    fn hash(&self) -> Option<u64>;
    /// Returns a "partial equal" comparison result if comparison is supported. Otherwise `None` will be returned.
    fn partial_eq(&self, _value: &dyn Reflect) -> Option<bool>;
    /// Returns a serializable value, if serialization is supported. Otherwise `None` will be returned.
    fn serializable(&self) -> Option<Serializable>;
}

impl Debug for dyn Reflect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Reflect({})", self.type_name()))
    }
}

impl dyn Reflect {
    pub fn downcast<T: Reflect>(self: Box<dyn Reflect>) -> Result<Box<T>, Box<dyn Reflect>> {
        // SAFE?: Same approach used by std::any::Box::downcast. ReflectValue is always Any and type has been checked.
        if self.is::<T>() {
            unsafe {
                let raw: *mut dyn Reflect = Box::into_raw(self);
                Ok(Box::from_raw(raw as *mut T))
            }
        } else {
            Err(self)
        }
    }

    pub fn take<T: Reflect>(self: Box<dyn Reflect>) -> Result<T, Box<dyn Reflect>> {
        self.downcast::<T>().map(|value| *value)
    }

    #[inline]
    pub fn is<T: Reflect>(&self) -> bool {
        self.any().is::<T>()
    }

    #[inline]
    pub fn downcast_ref<T: Reflect>(&self) -> Option<&T> {
        self.any().downcast_ref::<T>()
    }

    #[inline]
    pub fn downcast_mut<T: Reflect>(&mut self) -> Option<&mut T> {
        self.any_mut().downcast_mut::<T>()
    }
}
