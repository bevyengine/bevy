use crate::{
    error::ReflectCloneError,
    generics::{Generics, TypeParamInfo},
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    list::{List, ListInfo, ListIter},
    prelude::*,
    reflect::{impl_full_reflect, ApplyError},
    type_info::{MaybeTyped, TypeInfo, Typed},
    type_registry::{
        FromType, GetTypeRegistration, ReflectFromPtr, TypeRegistration, TypeRegistry,
    },
    utility::GenericTypeInfoCell,
};
use alloc::borrow::Cow;
use alloc::collections::VecDeque;
use bevy_platform::prelude::*;
use bevy_reflect_derive::impl_type_path;

macro_rules! impl_reflect_for_veclike {
    ($ty:ty, $insert:expr, $remove:expr, $push:expr, $pop:expr, $sub:ty) => {
        impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration> List for $ty {
            #[inline]
            fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
                <$sub>::get(self, index).map(|value| value as &dyn PartialReflect)
            }

            #[inline]
            fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
                <$sub>::get_mut(self, index).map(|value| value as &mut dyn PartialReflect)
            }

            fn insert(&mut self, index: usize, value: Box<dyn PartialReflect>) {
                let value = value.try_take::<T>().unwrap_or_else(|value| {
                    T::from_reflect(&*value).unwrap_or_else(|| {
                        panic!(
                            "Attempted to insert invalid value of type {}.",
                            value.reflect_type_path()
                        )
                    })
                });
                $insert(self, index, value);
            }

            fn remove(&mut self, index: usize) -> Box<dyn PartialReflect> {
                Box::new($remove(self, index))
            }

            fn push(&mut self, value: Box<dyn PartialReflect>) {
                let value = T::take_from_reflect(value).unwrap_or_else(|value| {
                    panic!(
                        "Attempted to push invalid value of type {}.",
                        value.reflect_type_path()
                    )
                });
                $push(self, value);
            }

            fn pop(&mut self) -> Option<Box<dyn PartialReflect>> {
                $pop(self).map(|value| Box::new(value) as Box<dyn PartialReflect>)
            }

            #[inline]
            fn len(&self) -> usize {
                <$sub>::len(self)
            }

            #[inline]
            fn iter(&self) -> ListIter {
                ListIter::new(self)
            }

            #[inline]
            fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
                self.drain(..)
                    .map(|value| Box::new(value) as Box<dyn PartialReflect>)
                    .collect()
            }
        }

        impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration> PartialReflect for $ty {
            #[inline]
            fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
                Some(<Self as Typed>::type_info())
            }

            fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
                self
            }

            #[inline]
            fn as_partial_reflect(&self) -> &dyn PartialReflect {
                self
            }

            #[inline]
            fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
                self
            }

            fn try_into_reflect(
                self: Box<Self>,
            ) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
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

            fn reflect_ref(&self) -> ReflectRef {
                ReflectRef::List(self)
            }

            fn reflect_mut(&mut self) -> ReflectMut {
                ReflectMut::List(self)
            }

            fn reflect_owned(self: Box<Self>) -> ReflectOwned {
                ReflectOwned::List(self)
            }

            fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
                Ok(Box::new(
                    self.iter()
                        .map(|value| {
                            value.reflect_clone()?.take().map_err(|_| {
                                ReflectCloneError::FailedDowncast {
                                    expected: Cow::Borrowed(<T as TypePath>::type_path()),
                                    received: Cow::Owned(value.reflect_type_path().to_string()),
                                }
                            })
                        })
                        .collect::<Result<Self, ReflectCloneError>>()?,
                ))
            }

            fn reflect_hash(&self) -> Option<u64> {
                crate::list_hash(self)
            }

            fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
                crate::list_partial_eq(self, value)
            }

            fn apply(&mut self, value: &dyn PartialReflect) {
                crate::list_apply(self, value);
            }

            fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
                crate::list_try_apply(self, value)
            }
        }

        impl_full_reflect!(<T> for $ty where T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration);

        impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration> Typed for $ty {
            fn type_info() -> &'static TypeInfo {
                static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    TypeInfo::List(
                        ListInfo::new::<Self, T>().with_generics(Generics::from_iter([
                            TypeParamInfo::new::<T>("T")
                        ]))
                    )
                })
            }
        }

        impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration> GetTypeRegistration
            for $ty
        {
            fn get_type_registration() -> TypeRegistration {
                let mut registration = TypeRegistration::of::<$ty>();
                registration.insert::<ReflectFromPtr>(FromType::<$ty>::from_type());
                registration.insert::<ReflectFromReflect>(FromType::<$ty>::from_type());
                registration
            }

            fn register_type_dependencies(registry: &mut TypeRegistry) {
                registry.register::<T>();
            }
        }

        impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration> FromReflect for $ty {
            fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
                let ref_list = reflect.reflect_ref().as_list().ok()?;

                let mut new_list = Self::with_capacity(ref_list.len());

                for field in ref_list.iter() {
                    $push(&mut new_list, T::from_reflect(field)?);
                }

                Some(new_list)
            }
        }
    };
}

impl_reflect_for_veclike!(Vec<T>, Vec::insert, Vec::remove, Vec::push, Vec::pop, [T]);
impl_type_path!(::alloc::vec::Vec<T>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(Vec<T>; <T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration>);

impl_reflect_for_veclike!(
    VecDeque<T>,
    VecDeque::insert,
    VecDeque::remove,
    VecDeque::push_back,
    VecDeque::pop_back,
    VecDeque::<T>
);
impl_type_path!(::alloc::collections::VecDeque<T>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(VecDeque<T>; <T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration>);
