use crate::{
    error::ReflectCloneError,
    generics::{Generics, TypeParamInfo},
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    prelude::*,
    reflect::{impl_full_reflect, ApplyError},
    set::{set_apply, set_partial_eq, set_try_apply, Set, SetInfo},
    type_info::{TypeInfo, Typed},
    type_registry::{
        FromType, GetTypeRegistration, ReflectFromPtr, TypeRegistration, TypeRegistry,
    },
    utility::GenericTypeInfoCell,
};
use alloc::borrow::Cow;
use alloc::vec::Vec;
use bevy_platform::prelude::*;
use bevy_reflect_derive::impl_type_path;
use core::hash::{BuildHasher, Hash};

macro_rules! impl_reflect_for_hashset {
    ($ty:path) => {
        impl<V, S> Set for $ty
        where
            V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
            S: TypePath + BuildHasher + Default + Send + Sync,
        {
            fn get(&self, value: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
                value
                    .try_downcast_ref::<V>()
                    .and_then(|value| Self::get(self, value))
                    .map(|value| value as &dyn PartialReflect)
            }

            fn len(&self) -> usize {
                Self::len(self)
            }

            fn iter(&self) -> Box<dyn Iterator<Item = &dyn PartialReflect> + '_> {
                let iter = self.iter().map(|v| v as &dyn PartialReflect);
                Box::new(iter)
            }

            fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
                self.drain()
                    .map(|value| Box::new(value) as Box<dyn PartialReflect>)
                    .collect()
            }

            fn insert_boxed(&mut self, value: Box<dyn PartialReflect>) -> bool {
                let value = V::take_from_reflect(value).unwrap_or_else(|value| {
                    panic!(
                        "Attempted to insert invalid value of type {}.",
                        value.reflect_type_path()
                    )
                });
                self.insert(value)
            }

            fn remove(&mut self, value: &dyn PartialReflect) -> bool {
                let mut from_reflect = None;
                value
                    .try_downcast_ref::<V>()
                    .or_else(|| {
                        from_reflect = V::from_reflect(value);
                        from_reflect.as_ref()
                    })
                    .is_some_and(|value| self.remove(value))
            }

            fn contains(&self, value: &dyn PartialReflect) -> bool {
                let mut from_reflect = None;
                value
                    .try_downcast_ref::<V>()
                    .or_else(|| {
                        from_reflect = V::from_reflect(value);
                        from_reflect.as_ref()
                    })
                    .is_some_and(|value| self.contains(value))
            }
        }

        impl<V, S> PartialReflect for $ty
        where
            V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
            S: TypePath + BuildHasher + Default + Send + Sync,
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

            #[inline]
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

            fn apply(&mut self, value: &dyn PartialReflect) {
                set_apply(self, value);
            }

            fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
                set_try_apply(self, value)
            }

            fn reflect_kind(&self) -> ReflectKind {
                ReflectKind::Set
            }

            fn reflect_ref(&self) -> ReflectRef {
                ReflectRef::Set(self)
            }

            fn reflect_mut(&mut self) -> ReflectMut {
                ReflectMut::Set(self)
            }

            fn reflect_owned(self: Box<Self>) -> ReflectOwned {
                ReflectOwned::Set(self)
            }

            fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
                let mut set = Self::with_capacity_and_hasher(self.len(), S::default());
                for value in self.iter() {
                    let value = value.reflect_clone()?.take().map_err(|_| {
                        ReflectCloneError::FailedDowncast {
                            expected: Cow::Borrowed(<V as TypePath>::type_path()),
                            received: Cow::Owned(value.reflect_type_path().to_string()),
                        }
                    })?;
                    set.insert(value);
                }

                Ok(Box::new(set))
            }

            fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
                set_partial_eq(self, value)
            }
        }

        impl<V, S> Typed for $ty
        where
            V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
            S: TypePath + BuildHasher + Default + Send + Sync,
        {
            fn type_info() -> &'static TypeInfo {
                static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    TypeInfo::Set(
                        SetInfo::new::<Self, V>().with_generics(Generics::from_iter([
                            TypeParamInfo::new::<V>("V")
                        ]))
                    )
                })
            }
        }

        impl<V, S> GetTypeRegistration for $ty
        where
            V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
            S: TypePath + BuildHasher + Default + Send + Sync + Default,
        {
            fn get_type_registration() -> TypeRegistration {
                let mut registration = TypeRegistration::of::<Self>();
                registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
                registration.insert::<ReflectFromReflect>(FromType::<Self>::from_type());
                registration
            }

            fn register_type_dependencies(registry: &mut TypeRegistry) {
                registry.register::<V>();
            }
        }

        impl_full_reflect!(
            <V, S> for $ty
            where
                V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
                S: TypePath + BuildHasher + Default + Send + Sync,
        );

        impl<V, S> FromReflect for $ty
        where
            V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
            S: TypePath + BuildHasher + Default + Send + Sync,
        {
            fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
                let ref_set = reflect.reflect_ref().as_set().ok()?;

                let mut new_set = Self::with_capacity_and_hasher(ref_set.len(), S::default());

                for value in ref_set.iter() {
                    let new_value = V::from_reflect(value)?;
                    new_set.insert(new_value);
                }

                Some(new_set)
            }
        }
    };
}

#[cfg(feature = "std")]
impl_reflect_for_hashset!(::std::collections::HashSet<V,S>);
#[cfg(feature = "std")]
impl_type_path!(::std::collections::HashSet<V, S>);
#[cfg(all(feature = "functions", feature = "std"))]
crate::func::macros::impl_function_traits!(::std::collections::HashSet<V, S>;
    <
        V: Hash + Eq + FromReflect + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

impl_reflect_for_hashset!(::bevy_platform::collections::HashSet<V,S>);
impl_type_path!(::bevy_platform::collections::HashSet<V, S>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::bevy_platform::collections::HashSet<V, S>;
    <
        V: Hash + Eq + FromReflect + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

#[cfg(feature = "hashbrown")]
impl_reflect_for_hashset!(::hashbrown::hash_set::HashSet<V,S>);
#[cfg(feature = "hashbrown")]
impl_type_path!(::hashbrown::hash_set::HashSet<V, S>);
#[cfg(all(feature = "functions", feature = "hashbrown"))]
crate::func::macros::impl_function_traits!(::hashbrown::hash_set::HashSet<V, S>;
    <
        V: Hash + Eq + FromReflect + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);