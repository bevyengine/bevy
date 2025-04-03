use alloc::boxed::Box;
use core::fmt::Debug;

use crate::{
    ApplyError, PartialReflect, Reflect, ReflectMut, ReflectOwned, ReflectRef, TypeInfo, TypePath,
};

#[derive(Clone, Copy, Debug, Default, TypePath)]
pub struct Null;

impl PartialReflect for Null {
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        None
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        Box::new(Self)
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Err(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        None
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        None
    }

    fn try_apply(&mut self, _value: &dyn PartialReflect) -> Result<(), ApplyError> {
        todo!("how should values be applied?")
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Opaque(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Opaque(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Opaque(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(*self)
    }
}
