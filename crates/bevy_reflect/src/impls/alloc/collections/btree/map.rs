use crate::{
    dynamic::DynamicValue,
    error::ReflectCloneError,
    generics::{Generics, TypeParamInfo},
    info::{MaybeTyped, TypeInfo, Typed},
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    map::{map_partial_cmp, map_partial_eq, Map, MapInfo},
    prelude::*,
    reflect::impl_reflect_casts,
    type_registry::{GetTypeRegistration, ReflectFromPtr, TypeRegistration},
    utility::GenericTypeInfoCell,
};
use alloc::vec::Vec;
use bevy_platform::prelude::*;
use bevy_reflect_derive::impl_type_path;

impl<K, V> Map for ::alloc::collections::BTreeMap<K, V>
where
    K: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
    V: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
{
    fn key_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn Reflect>> {
        K::from_dynamic(dynamic).map(|value| Box::new(value) as _)
    }

    fn value_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn Reflect>> {
        V::from_dynamic(dynamic).map(|value| Box::new(value) as _)
    }

    fn get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
        key.downcast_ref::<K>()
            .and_then(|key| Self::get(self, key))
            .map(|value| value as &dyn Reflect)
    }

    fn get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
        key.downcast_ref::<K>()
            .and_then(move |key| Self::get_mut(self, key))
            .map(|value| value as &mut dyn Reflect)
    }

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (&dyn Reflect, &dyn Reflect)> + '_> {
        Box::new(
            self.iter()
                .map(|(k, v)| (k as &dyn Reflect, v as &dyn Reflect)),
        )
    }

    fn drain(&mut self) -> Vec<(Box<dyn Reflect>, Box<dyn Reflect>)> {
        // BTreeMap doesn't have a `drain` function. See
        // https://github.com/rust-lang/rust/issues/81074. So we have to fake one by popping
        // elements off one at a time.
        let mut result = Vec::with_capacity(self.len());
        while let Some((k, v)) = self.pop_first() {
            result.push((
                Box::new(k) as Box<dyn Reflect>,
                Box::new(v) as Box<dyn Reflect>,
            ));
        }
        result
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn Reflect, &mut dyn Reflect) -> bool) {
        self.retain(move |k, v| f(k, v));
    }

    fn insert_boxed(
        &mut self,
        key: Box<dyn Reflect>,
        value: Box<dyn Reflect>,
    ) -> Option<Box<dyn Reflect>> {
        let key = *key.downcast::<K>().ok()?;
        let value = *value.downcast::<V>().ok()?;
        self.insert(key, value)
            .map(|old_value| Box::new(old_value) as Box<dyn Reflect>)
    }

    fn remove(&mut self, key: &dyn Reflect) -> Option<Box<dyn Reflect>> {
        key.downcast_ref::<K>()
            .and_then(|key| self.remove(key))
            .map(|value| Box::new(value) as Box<dyn Reflect>)
    }
}

impl<K, V> Reflect for ::alloc::collections::BTreeMap<K, V>
where
    K: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
    V: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
{
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

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        map_partial_eq(self, value)
    }

    fn reflect_partial_cmp(&self, value: &dyn Reflect) -> Option<::core::cmp::Ordering> {
        map_partial_cmp(self, value)
    }

    impl_reflect_casts! {}
}

impl<K, V> Typed for ::alloc::collections::BTreeMap<K, V>
where
    K: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
    V: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
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
    K: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
    V: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.register_type_data::<ReflectFromPtr, Self>();
        registration.register_type_data::<ReflectFromDynamic, Self>();
        registration
    }

    fn register_type_dependencies(registry: &mut crate::TypeRegistry) {
        registry.register::<K>();
        registry.register::<V>();
    }
}

impl<K, V> FromDynamic for ::alloc::collections::BTreeMap<K, V>
where
    K: FromDynamic + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
    V: FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
{
    fn from_dynamic(dynamic: DynamicValue) -> Option<Self> {
        let dynamic = match dynamic {
            DynamicValue::Concrete(concrete) => return concrete.take().ok(),
            DynamicValue::Map(dynamic) => dynamic,
            _ => return None,
        };

        let mut new_map = Self::new();

        for (key, value) in dynamic.into_iter() {
            let new_key = K::from_dynamic(key)?;
            let new_value = V::from_dynamic(value)?;
            new_map.insert(new_key, new_value);
        }

        Some(new_map)
    }
}

impl_type_path!(::alloc::collections::BTreeMap<K, V>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::alloc::collections::BTreeMap<K, V>;
    <
        K: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration + Eq + Ord,
        V: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration
    >
);
