use crate::{
    dynamic::DynamicValue,
    error::ReflectCloneError,
    info::{OpaqueInfo, TypeInfo, Typed},
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    prelude::*,
    type_registry::{GetTypeRegistration, ReflectFromPtr, TypeRegistration},
    utility::{reflect_hasher, NonGenericTypeInfoCell},
};
use bevy_platform::prelude::*;
use core::any::Any;
use core::hash::{Hash, Hasher};
use core::panic::Location;

impl TypePath for &'static Location<'static> {
    fn type_path() -> &'static str {
        "core::panic::Location"
    }

    fn short_type_path() -> &'static str {
        "Location"
    }
}

impl Reflect for &'static Location<'static> {
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::Opaque
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Opaque(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Opaque(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Opaque(self)
    }

    fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
        Ok(Box::new(*self))
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = reflect_hasher();
        Hash::hash(&Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        if let Some(value) = value.downcast_ref::<Self>() {
            Some(PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }

    fn reflect_partial_cmp(&self, value: &dyn Reflect) -> Option<core::cmp::Ordering> {
        if let Some(value) = value.downcast_ref::<Self>() {
            PartialOrd::partial_cmp(self, value)
        } else {
            None
        }
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }
}

impl Typed for &'static Location<'static> {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

impl GetTypeRegistration for &'static Location<'static> {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.register_type_data::<ReflectFromPtr, Self>();
        registration.register_type_data::<ReflectFromDynamic, Self>();
        registration
    }
}

impl FromDynamic for &'static Location<'static> {
    fn from_dynamic(dynamic: DynamicValue) -> Option<Self> {
        dynamic.into_concrete()?.downcast_ref::<Self>().copied()
    }
}
