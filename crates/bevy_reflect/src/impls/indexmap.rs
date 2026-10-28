use crate::{
    dynamic::DynamicValue,
    set::{Set, SetInfo},
    utility::GenericTypeInfoCell,
    FromDynamic, Generics, GetTypeRegistration, Reflect, ReflectCloneError, ReflectFromPtr,
    ReflectMut, ReflectOwned, ReflectRef, TypeInfo, TypeParamInfo, TypePath, TypeRegistration,
};
use bevy_platform::prelude::{Box, Vec};
use bevy_reflect::{
    map::{Map, MapInfo},
    MaybeTyped, ReflectFromDynamic, ReflectKind, TypeRegistry, Typed,
};
use bevy_reflect_derive::impl_type_path;
use core::{any::Any, hash::BuildHasher, hash::Hash};
use indexmap::{IndexMap, IndexSet};

impl<K, V, S> Map for IndexMap<K, V, S>
where
    K: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
    V: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
    S: TypePath + BuildHasher + Default + Send + Sync,
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
        self.drain(..)
            .map(|(key, value)| {
                (
                    Box::new(key) as Box<dyn Reflect>,
                    Box::new(value) as Box<dyn Reflect>,
                )
            })
            .collect()
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn Reflect, &mut dyn Reflect) -> bool) {
        self.retain(move |key, value| f(key, value));
    }

    fn insert_boxed(
        &mut self,
        key: Box<dyn Reflect>,
        value: Box<dyn Reflect>,
    ) -> Option<Box<dyn Reflect>> {
        let key = *key.downcast::<K>().unwrap_or_else(|key| {
            panic!(
                "Attempted to insert invalid key of type {}.",
                key.reflect_type_path()
            )
        });
        let value = *value.downcast::<V>().unwrap_or_else(|value| {
            panic!(
                "Attempted to insert invalid value of type {}.",
                value.reflect_type_path()
            )
        });
        self.insert(key, value)
            .map(|old_value| Box::new(old_value) as Box<dyn Reflect>)
    }

    fn remove(&mut self, key: &dyn Reflect) -> Option<Box<dyn Reflect>> {
        key.downcast_ref::<K>()
            .and_then(|key| self.shift_remove(key))
            .map(|value| Box::new(value) as Box<dyn Reflect>)
    }
}

impl<K, V, S> Reflect for IndexMap<K, V, S>
where
    K: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
    V: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
    S: TypePath + BuildHasher + Default + Send + Sync,
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
        let mut map = Self::with_capacity_and_hasher(self.len(), S::default());
        for (key, value) in self.iter() {
            let key = key.reflect_clone_and_take()?;
            let value = value.reflect_clone_and_take()?;
            map.insert(key, value);
        }

        Ok(Box::new(map))
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::map::map_partial_eq(self, value)
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

impl<K, V, S> Typed for IndexMap<K, V, S>
where
    K: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
    V: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
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

impl<K, V, S> FromDynamic for IndexMap<K, V, S>
where
    K: FromDynamic + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
    V: FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
    fn from_dynamic(dynamic: DynamicValue) -> Option<Self> {
        let dynamic = match dynamic {
            DynamicValue::Map(dynamic) => dynamic,
            DynamicValue::Concrete(concrete) => return concrete.take().ok(),
            _ => return None,
        };

        let mut new_map = Self::with_capacity_and_hasher(dynamic.len(), S::default());

        for (key, value) in dynamic.into_iter() {
            let new_key = K::from_dynamic(key)?;
            let new_value = V::from_dynamic(value)?;
            new_map.insert(new_key, new_value);
        }

        Some(new_map)
    }
}

impl<K, V, S> GetTypeRegistration for IndexMap<K, V, S>
where
    K: Hash + Eq + Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
    V: Reflect + FromDynamic + MaybeTyped + TypePath + GetTypeRegistration,
    S: TypePath + BuildHasher + Send + Sync + Default,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.register_type_data::<ReflectFromPtr, Self>();
        registration.register_type_data::<ReflectFromDynamic, Self>();
        registration
    }

    fn register_type_dependencies(registry: &mut TypeRegistry) {
        registry.register::<K>();
        registry.register::<V>();
    }
}

impl_type_path!(::indexmap::IndexMap<K, V, S>);

impl<T, S> Set for IndexSet<T, S>
where
    T: Reflect + FromDynamic + TypePath + GetTypeRegistration + Eq + Hash,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
    fn value_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn Reflect>> {
        T::from_dynamic(dynamic).map(|value| Box::new(value) as _)
    }

    fn get(&self, value: &dyn Reflect) -> Option<&dyn Reflect> {
        value
            .downcast_ref::<T>()
            .and_then(|value| Self::get(self, value))
            .map(|value| value as &dyn Reflect)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &dyn Reflect> + '_> {
        let iter = self.iter().map(|v| v as &dyn Reflect);
        Box::new(iter)
    }

    fn drain(&mut self) -> Vec<Box<dyn Reflect>> {
        self.drain(..)
            .map(|value| Box::new(value) as Box<dyn Reflect>)
            .collect()
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn Reflect) -> bool) {
        self.retain(move |value| f(value));
    }

    fn insert_boxed(&mut self, value: Box<dyn Reflect>) -> bool {
        let value = *value.downcast::<T>().unwrap_or_else(|value| {
            panic!(
                "Attempted to insert invalid value of type {}.",
                value.reflect_type_path()
            )
        });
        self.insert(value)
    }

    fn remove(&mut self, value: &dyn Reflect) -> bool {
        value
            .downcast_ref::<T>()
            .is_some_and(|value| self.shift_remove(value))
    }

    fn contains(&self, value: &dyn Reflect) -> bool {
        value
            .downcast_ref::<T>()
            .is_some_and(|value| self.contains(value))
    }
}

impl<T, S> Reflect for IndexSet<T, S>
where
    T: Reflect + FromDynamic + TypePath + GetTypeRegistration + Eq + Hash,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::Set
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Set(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Set(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Set(self)
    }

    fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
        Ok(Box::new(
            self.iter()
                .map(Reflect::reflect_clone_and_take)
                .collect::<Result<Self, ReflectCloneError>>()?,
        ))
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::set::set_partial_eq(self, value)
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

impl<T, S> Typed for IndexSet<T, S>
where
    T: Reflect + FromDynamic + TypePath + GetTypeRegistration + Eq + Hash,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            TypeInfo::Set(
                SetInfo::new::<Self, T>()
                    .with_generics(Generics::from_iter([TypeParamInfo::new::<T>("T")])),
            )
        })
    }
}

impl<T, S> FromDynamic for IndexSet<T, S>
where
    T: FromDynamic + TypePath + GetTypeRegistration + Eq + Hash,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
    fn from_dynamic(dynamic: DynamicValue) -> Option<Self> {
        let dynamic = match dynamic {
            DynamicValue::Set(dynamic) => dynamic,
            DynamicValue::Concrete(concrete) => return concrete.take().ok(),
            _ => return None,
        };

        let mut new_set = Self::with_capacity_and_hasher(dynamic.len(), S::default());

        for value in dynamic.into_iter() {
            new_set.insert(T::from_dynamic(value)?);
        }

        Some(new_set)
    }
}

impl<T, S> GetTypeRegistration for IndexSet<T, S>
where
    T: Reflect + FromDynamic + TypePath + GetTypeRegistration + Eq + Hash,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.register_type_data::<ReflectFromPtr, Self>();
        registration
    }
}

impl_type_path!(::indexmap::IndexSet<T, S>);
