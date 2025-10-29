use crate::{
    error::ReflectCloneError,
    generics::{Generics, TypeParamInfo},
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    map::{map_apply, map_partial_eq, map_try_apply, Map, MapInfo},
    prelude::*,
    reflect::{impl_full_reflect, ApplyError},
    type_info::{MaybeTyped, TypeInfo, Typed},
    type_registry::{FromType, GetTypeRegistration, ReflectFromPtr, TypeRegistration},
    utility::GenericTypeInfoCell,
};
use alloc::vec::Vec;
use bevy_platform::prelude::*;
use bevy_reflect_derive::impl_type_path;

impl<K, V> Map for ::alloc::collections::BTreeMap<K, V>
where
    K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
    V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
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

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (&dyn PartialReflect, &dyn PartialReflect)> + '_> {
        Box::new(
            self.iter()
                .map(|(k, v)| (k as &dyn PartialReflect, v as &dyn PartialReflect)),
        )
    }

    fn drain(&mut self) -> Vec<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
        // BTreeMap doesn't have a `drain` function. See
        // https://github.com/rust-lang/rust/issues/81074. So we have to fake one by popping
        // elements off one at a time.
        let mut result = Vec::with_capacity(self.len());
        while let Some((k, v)) = self.pop_first() {
            result.push((
                Box::new(k) as Box<dyn PartialReflect>,
                Box::new(v) as Box<dyn PartialReflect>,
            ));
        }
        result
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn PartialReflect, &mut dyn PartialReflect) -> bool) {
        self.retain(move |k, v| f(k, v));
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

impl<K, V> PartialReflect for ::alloc::collections::BTreeMap<K, V>
where
    K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
    V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
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
        ReflectKind::Map
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Map(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Map(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Map(self)
    }

    fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
        let mut map = Self::new();
        for (key, value) in self.iter() {
            let key = key.reflect_clone_and_take()?;
            let value = value.reflect_clone_and_take()?;
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
    <K, V> for ::alloc::collections::BTreeMap<K, V>
    where
        K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
        V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
);

impl<K, V> Typed for ::alloc::collections::BTreeMap<K, V>
where
    K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
    V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
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

impl<K, V> GetTypeRegistration for ::alloc::collections::BTreeMap<K, V>
where
    K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
    V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration.insert::<ReflectFromReflect>(FromType::<Self>::from_type());
        registration
    }
}

impl<K, V> FromReflect for ::alloc::collections::BTreeMap<K, V>
where
    K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
    V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
{
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        let ref_map = reflect.reflect_ref().as_map().ok()?;

        let mut new_map = Self::new();

        for (key, value) in ref_map.iter() {
            let new_key = K::from_reflect(key)?;
            let new_value = V::from_reflect(value)?;
            new_map.insert(new_key, new_value);
        }

        Some(new_map)
    }
}

impl_type_path!(::alloc::collections::BTreeMap<K, V>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::alloc::collections::BTreeMap<K, V>;
    <
        K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
        V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration
    >
);
