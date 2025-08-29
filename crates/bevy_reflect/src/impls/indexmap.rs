use crate::{
    utility::GenericTypeInfoCell, FromReflect, FromType, Generics, GetTypeRegistration,
    PartialReflect, Reflect, ReflectCloneError, ReflectFromPtr, ReflectMut, ReflectOwned,
    ReflectRef, Set, SetInfo, TypeInfo, TypeParamInfo, TypePath, TypeRegistration,
};
use bevy_platform::prelude::{Box, Vec};
use bevy_reflect::{
    DynamicMap, Map, MapInfo, MaybeTyped, ReflectFromReflect, ReflectKind, TypeRegistry, Typed,
};
use bevy_reflect_derive::impl_type_path;
use core::{any::Any, hash::BuildHasher, hash::Hash};
use indexmap::{IndexMap, IndexSet};

impl<K, V, S> Map for IndexMap<K, V, S>
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
        self.drain(..)
            .map(|(key, value)| {
                (
                    Box::new(key) as Box<dyn PartialReflect>,
                    Box::new(value) as Box<dyn PartialReflect>,
                )
            })
            .collect()
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn PartialReflect, &mut dyn PartialReflect) -> bool) {
        self.retain(move |key, value| f(key, value));
    }

    fn to_dynamic_map(&self) -> DynamicMap {
        let mut dynamic_map = DynamicMap::default();
        dynamic_map.set_represented_type(PartialReflect::get_represented_type_info(self));
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
            .and_then(|key| self.shift_remove(key))
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
    }
}

impl<K, V, S> PartialReflect for IndexMap<K, V, S>
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

    fn apply(&mut self, value: &dyn PartialReflect) {
        crate::map_apply(self, value);
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), crate::ApplyError> {
        crate::map_try_apply(self, value)
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
        let mut map = Self::with_capacity_and_hasher(self.len(), S::default());
        for (key, value) in self.iter() {
            let key = key.reflect_clone_and_take()?;
            let value = value.reflect_clone_and_take()?;
            map.insert(key, value);
        }

        Ok(Box::new(map))
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        crate::map_partial_eq(self, value)
    }
}

impl<K, V, S> Reflect for IndexMap<K, V, S>
where
    K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
    V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
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

impl<K, V, S> FromReflect for IndexMap<K, V, S>
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

impl<K, V, S> GetTypeRegistration for IndexMap<K, V, S>
where
    K: Hash + Eq + FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
    V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
    S: TypePath + BuildHasher + Send + Sync + Default,
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

impl_type_path!(::indexmap::IndexMap<K, V, S>);

impl<T, S> Set for IndexSet<T, S>
where
    T: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
    fn get(&self, value: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
        value
            .try_downcast_ref::<T>()
            .and_then(|value| Self::get(self, value))
            .map(|value| value as &dyn PartialReflect)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &dyn PartialReflect> + '_> {
        let iter = self.iter().map(|v| v as &dyn PartialReflect);
        Box::new(iter)
    }

    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
        self.drain(..)
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
            .collect()
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn PartialReflect) -> bool) {
        self.retain(move |value| f(value));
    }

    fn insert_boxed(&mut self, value: Box<dyn PartialReflect>) -> bool {
        let value = T::take_from_reflect(value).unwrap_or_else(|value| {
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
            .try_downcast_ref::<T>()
            .or_else(|| {
                from_reflect = T::from_reflect(value);
                from_reflect.as_ref()
            })
            .is_some_and(|value| self.shift_remove(value))
    }

    fn contains(&self, value: &dyn PartialReflect) -> bool {
        let mut from_reflect = None;
        value
            .try_downcast_ref::<T>()
            .or_else(|| {
                from_reflect = T::from_reflect(value);
                from_reflect.as_ref()
            })
            .is_some_and(|value| self.contains(value))
    }
}

impl<T, S> PartialReflect for IndexSet<T, S>
where
    T: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
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
    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    fn apply(&mut self, value: &dyn PartialReflect) {
        crate::set_apply(self, value);
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), crate::ApplyError> {
        crate::set_try_apply(self, value)
    }

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
                .map(PartialReflect::reflect_clone_and_take)
                .collect::<Result<Self, ReflectCloneError>>()?,
        ))
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        crate::set_partial_eq(self, value)
    }
}

impl<T, S> Reflect for IndexSet<T, S>
where
    T: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
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
    T: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
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

impl<T, S> FromReflect for IndexSet<T, S>
where
    T: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        let ref_set = reflect.reflect_ref().as_set().ok()?;

        let mut new_set = Self::with_capacity_and_hasher(ref_set.len(), S::default());

        for field in ref_set.iter() {
            new_set.insert(T::from_reflect(field)?);
        }

        Some(new_set)
    }
}

impl<T, S> GetTypeRegistration for IndexSet<T, S>
where
    T: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
    S: TypePath + BuildHasher + Default + Send + Sync,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration
    }
}

impl_type_path!(::indexmap::IndexSet<T, S>);
