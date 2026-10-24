use crate::{
    dynamic::DynamicValue,
    error::ReflectCloneError,
    info::{MaybeTyped, OpaqueInfo, TypeInfo, Typed},
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    list::{List, ListInfo, ListIter},
    prelude::*,
    reflect::impl_full_reflect,
    type_registry::{
        GetTypeRegistration, ReflectDeserialize, ReflectFromPtr, ReflectSerialize,
        TypeRegistration, TypeRegistry,
    },
    utility::{reflect_hasher, GenericTypeInfoCell, NonGenericTypeInfoCell},
};
use alloc::borrow::Cow;
use alloc::vec::Vec;
use bevy_platform::prelude::*;
use bevy_reflect_derive::impl_type_path;
use core::any::Any;
use core::fmt;
use core::hash::{Hash, Hasher};

impl_type_path!(::alloc::borrow::Cow<'a: 'static, T: ToOwned + ?Sized>);

impl PartialReflect for Cow<'static, str> {
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
    }

    #[inline]
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

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
        Ok(Box::new(self.clone()))
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = reflect_hasher();
        Hash::hash(&Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            Some(PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }

    fn reflect_partial_cmp(&self, value: &dyn PartialReflect) -> Option<core::cmp::Ordering> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            PartialOrd::partial_cmp(self, value)
        } else {
            None
        }
    }

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl_full_reflect!(for Cow<'static, str>);

impl Typed for Cow<'static, str> {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

impl GetTypeRegistration for Cow<'static, str> {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Cow<'static, str>>();
        registration.register_type_data::<ReflectDeserialize, Self>();
        registration.register_type_data::<ReflectFromPtr, Self>();
        registration.register_type_data::<ReflectFromDynamic, Self>();
        registration.register_type_data::<ReflectSerialize, Self>();
        registration
    }
}

impl FromDynamic for Cow<'static, str> {
    fn from_dynamic(dynamic: DynamicValue) -> Option<Self> {
        dynamic
            .into_concrete()?
            .downcast::<Cow<'static, str>>()
            .ok()
            .map(|v| *v)
    }
}

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(Cow<'static, str>);

impl<T: FromDynamic + MaybeTyped + Clone + TypePath + GetTypeRegistration> List
    for Cow<'static, [T]>
{
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_ref().get(index).map(|x| x as &dyn PartialReflect)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.to_mut()
            .get_mut(index)
            .map(|x| x as &mut dyn PartialReflect)
    }

    fn value_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn PartialReflect>> {
        T::from_dynamic(dynamic).map(|value| Box::new(value) as _)
    }

    fn insert(&mut self, index: usize, element: Box<dyn PartialReflect>) {
        let value = *element.try_downcast::<T>().unwrap_or_else(|value| {
            panic!(
                "Attempted to insert invalid value of type {}.",
                value.reflect_type_path()
            );
        });
        self.to_mut().insert(index, value);
    }

    fn remove(&mut self, index: usize) -> Box<dyn PartialReflect> {
        Box::new(self.to_mut().remove(index))
    }

    fn swap(&mut self, a: usize, b: usize) {
        self.to_mut().swap(a, b);
    }

    fn push(&mut self, value: Box<dyn PartialReflect>) {
        let value = *value.try_downcast::<T>().unwrap_or_else(|value| {
            panic!(
                "Attempted to push invalid value of type {}.",
                value.reflect_type_path()
            )
        });
        self.to_mut().push(value);
    }

    fn pop(&mut self) -> Option<Box<dyn PartialReflect>> {
        self.to_mut()
            .pop()
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
    }

    fn len(&self) -> usize {
        self.as_ref().len()
    }

    fn iter(&self) -> ListIter<'_> {
        ListIter::new(self)
    }

    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
        self.to_mut()
            .drain(..)
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
            .collect()
    }
}

impl<T: FromDynamic + MaybeTyped + Clone + TypePath + GetTypeRegistration> PartialReflect
    for Cow<'static, [T]>
{
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
    }

    #[inline]
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::List
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::List(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::List(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::List(self)
    }

    fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
        Ok(Box::new(self.clone()))
    }

    fn reflect_hash(&self) -> Option<u64> {
        crate::list::list_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        crate::list::list_partial_eq(self, value)
    }

    fn reflect_partial_cmp(&self, value: &dyn PartialReflect) -> Option<::core::cmp::Ordering> {
        crate::list::list_partial_cmp(self, value)
    }
}

impl_full_reflect!(
    <T> for Cow<'static, [T]>
    where
        T: FromDynamic + Clone + MaybeTyped + TypePath + GetTypeRegistration,
);

impl<T: FromDynamic + MaybeTyped + Clone + TypePath + GetTypeRegistration> Typed
    for Cow<'static, [T]>
{
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::List(ListInfo::new::<Self, T>()))
    }
}

impl<T: FromDynamic + MaybeTyped + Clone + TypePath + GetTypeRegistration> GetTypeRegistration
    for Cow<'static, [T]>
{
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Cow<'static, [T]>>()
    }

    fn register_type_dependencies(registry: &mut TypeRegistry) {
        registry.register::<T>();
    }
}

impl<T: FromDynamic + MaybeTyped + Clone + TypePath + GetTypeRegistration> FromDynamic
    for Cow<'static, [T]>
{
    fn from_dynamic(dynamic: DynamicValue) -> Option<Self> {
        let dynamic = match dynamic {
            DynamicValue::List(dynamic) => dynamic,
            DynamicValue::Concrete(concrete) => return concrete.downcast().ok().map(|v| *v),
            _ => return None,
        };

        let mut temp_vec = Vec::with_capacity(dynamic.len());

        for value in dynamic.into_iter() {
            temp_vec.push(T::from_dynamic(value)?);
        }

        Some(temp_vec.into())
    }
}

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(Cow<'static, [T]>; <T: FromDynamic + MaybeTyped + Clone + TypePath + GetTypeRegistration>);
