use crate::{
    error::ReflectCloneError,
    generics::{Generics, TypeParamInfo},
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    map::{map_apply, map_partial_eq, map_try_apply, DynamicMap, Map, MapInfo, MapIter},
    prelude::*,
    reflect::{impl_full_reflect, ApplyError},
    type_info::{MaybeTyped, TypeInfo, Typed},
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

macro_rules! impl_reflect_for_hashmap {
    ($ty:path) => {
        impl<K, V, S> Map for $ty
        where
            K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
            V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
            S: TypePath + BuildHasher + Default + Send + Sync,
        {
            fn get(&self, key: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
                key.try_downcast_ref::<K>()
                    .and_then(|key| Self::get(self, key))
                    .map(|value| value as &dyn PartialReflect)
            }

            fn get_mut(&mut self, key: &dyn PartialReflect) -> Option<&mut dyn PartialReflect> {
                key.try_downcast_ref::<K>()
                    .and_then(move |key| Self::get_mut(self, key))
                    .map(|value| value as &mut dyn PartialReflect)
            }

            fn get_at(&self, index: usize) -> Option<(&dyn PartialReflect, &dyn PartialReflect)> {
                self.iter()
                    .nth(index)
                    .map(|(key, value)| (key as &dyn PartialReflect, value as &dyn PartialReflect))
            }

            fn get_at_mut(
                &mut self,
                index: usize,
            ) -> Option<(&dyn PartialReflect, &mut dyn PartialReflect)> {
                self.iter_mut().nth(index).map(|(key, value)| {
                    (key as &dyn PartialReflect, value as &mut dyn PartialReflect)
                })
            }

            fn len(&self) -> usize {
                Self::len(self)
            }

            fn iter(&self) -> MapIter {
                MapIter::new(self)
            }

            fn drain(&mut self) -> Vec<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
                self.drain()
                    .map(|(key, value)| {
                        (
                            Box::new(key) as Box<dyn PartialReflect>,
                            Box::new(value) as Box<dyn PartialReflect>,
                        )
                    })
                    .collect()
            }

            fn to_dynamic_map(&self) -> DynamicMap {
                let mut dynamic_map = DynamicMap::default();
                dynamic_map.set_represented_type(self.get_represented_type_info());
                for (k, v) in self {
                    let key = K::from_reflect(k).unwrap_or_else(|| {
                        panic!(
                            "Attempted to clone invalid key of type {}.",
                            k.reflect_type_path()
                        )
                    });
                    dynamic_map.insert_boxed(Box::new(key), v.to_dynamic());
                }
                dynamic_map
            }

            fn insert_boxed(
                &mut self,
                key: Box<dyn PartialReflect>,
                value: Box<dyn PartialReflect>,
            ) -> Option<Box<dyn PartialReflect>> {
                let key = K::take_from_reflect(key).unwrap_or_else(|key| {
                    panic!(
                        "Attempted to insert invalid key of type {}.",
                        key.reflect_type_path()
                    )
                });
                let value = V::take_from_reflect(value).unwrap_or_else(|value| {
                    panic!(
                        "Attempted to insert invalid value of type {}.",
                        value.reflect_type_path()
                    )
                });
                self.insert(key, value)
                    .map(|old_value| Box::new(old_value) as Box<dyn PartialReflect>)
            }

            fn remove(&mut self, key: &dyn PartialReflect) -> Option<Box<dyn PartialReflect>> {
                let mut from_reflect = None;
                key.try_downcast_ref::<K>()
                    .or_else(|| {
                        from_reflect = K::from_reflect(key);
                        from_reflect.as_ref()
                    })
                    .and_then(|key| self.remove(key))
                    .map(|value| Box::new(value) as Box<dyn PartialReflect>)
            }
        }

        impl<K, V, S> PartialReflect for $ty
        where
            K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
            V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
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
                ReflectKind::Map
            }

            fn reflect_ref(&self) -> ReflectRef {
                ReflectRef::Map(self)
            }

            fn reflect_mut(&mut self) -> ReflectMut {
                ReflectMut::Map(self)
            }

            fn reflect_owned(self: Box<Self>) -> ReflectOwned {
                ReflectOwned::Map(self)
            }

            fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
                let mut map = Self::with_capacity_and_hasher(self.len(), S::default());
                for (key, value) in self.iter() {
                    let key = key.reflect_clone()?.take().map_err(|_| {
                        ReflectCloneError::FailedDowncast {
                            expected: Cow::Borrowed(<K as TypePath>::type_path()),
                            received: Cow::Owned(key.reflect_type_path().to_string()),
                        }
                    })?;
                    let value = value.reflect_clone()?.take().map_err(|_| {
                        ReflectCloneError::FailedDowncast {
                            expected: Cow::Borrowed(<V as TypePath>::type_path()),
                            received: Cow::Owned(value.reflect_type_path().to_string()),
                        }
                    })?;
                    map.insert(key, value);
                }

                Ok(Box::new(map))
            }

            fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
                map_partial_eq(self, value)
            }

            fn apply(&mut self, value: &dyn PartialReflect) {
                map_apply(self, value);
            }

            fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
                map_try_apply(self, value)
            }
        }

        impl_full_reflect!(
            <K, V, S> for $ty
            where
                K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
                V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
                S: TypePath + BuildHasher + Default + Send + Sync,
        );

        impl<K, V, S> Typed for $ty
        where
            K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
            V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
            S: TypePath + BuildHasher + Default + Send + Sync,
        {
            fn type_info() -> &'static TypeInfo {
                static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    TypeInfo::Map(
                        MapInfo::new::<Self, K, V>().with_generics(Generics::from_iter([
                            TypeParamInfo::new::<K>("K"),
                            TypeParamInfo::new::<V>("V"),
                        ])),
                    )
                })
            }
        }

        impl<K, V, S> GetTypeRegistration for $ty
        where
            K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
            V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
            S: TypePath + BuildHasher + Default + Send + Sync + Default,
        {
            fn get_type_registration() -> TypeRegistration {
                let mut registration = TypeRegistration::of::<Self>();
                registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
                registration.insert::<ReflectFromReflect>(FromType::<Self>::from_type());
                registration
            }

            fn register_type_dependencies(registry: &mut TypeRegistry) {
                registry.register::<K>();
                registry.register::<V>();
            }
        }

        impl<K, V, S> FromReflect for $ty
        where
            K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
            V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
            S: TypePath + BuildHasher + Default + Send + Sync,
        {
            fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
                let ref_map = reflect.reflect_ref().as_map().ok()?;

                let mut new_map = Self::with_capacity_and_hasher(ref_map.len(), S::default());

                for (key, value) in ref_map.iter() {
                    let new_key = K::from_reflect(key)?;
                    let new_value = V::from_reflect(value)?;
                    new_map.insert(new_key, new_value);
                }

                Some(new_map)
            }
        }
    };
}

#[cfg(feature = "std")]
impl_reflect_for_hashmap!(::std::collections::HashMap<K, V, S>);
impl_type_path!(::core::hash::BuildHasherDefault<H>);
#[cfg(feature = "std")]
impl_type_path!(::std::collections::hash_map::RandomState);
#[cfg(feature = "std")]
impl_type_path!(::std::collections::HashMap<K, V, S>);
#[cfg(all(feature = "functions", feature = "std"))]
crate::func::macros::impl_function_traits!(::std::collections::HashMap<K, V, S>;
    <
        K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
        V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

impl_reflect_for_hashmap!(bevy_platform::collections::HashMap<K, V, S>);
impl_type_path!(::bevy_platform::collections::HashMap<K, V, S>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::bevy_platform::collections::HashMap<K, V, S>;
    <
        K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
        V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

#[cfg(feature = "hashbrown")]
impl_reflect_for_hashmap!(hashbrown::hash_map::HashMap<K, V, S>);
#[cfg(feature = "hashbrown")]
impl_type_path!(::hashbrown::hash_map::HashMap<K, V, S>);
#[cfg(all(feature = "functions", feature = "hashbrown"))]
crate::func::macros::impl_function_traits!(::hashbrown::hash_map::HashMap<K, V, S>;
    <
        K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
        V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);