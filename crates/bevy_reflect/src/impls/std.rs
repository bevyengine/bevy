use crate::{
    map_partial_eq, serde::Serializable, DynamicMap, Enum, EnumVariant, EnumVariantMut,
    GetTypeRegistration, List, ListIter, Map, MapIter, Reflect, ReflectDeserialize, ReflectMut,
    ReflectRef, TypeRegistration, VariantInfo, VariantInfoIter,
};

use bevy_reflect_derive::impl_reflect_value;
use bevy_utils::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    borrow::Cow,
    hash::{Hash, Hasher},
    ops::Range,
};

impl_reflect_value!(bool(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(u8(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(u16(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(u32(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(u64(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(u128(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(usize(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(i8(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(i16(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(i32(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(i64(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(i128(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(isize(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(f32(Serialize, Deserialize));
impl_reflect_value!(f64(Serialize, Deserialize));
impl_reflect_value!(String(Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(HashSet<T: Serialize + Hash + Eq + Clone + for<'de> Deserialize<'de> + Send + Sync + 'static>(Serialize, Deserialize));
impl_reflect_value!(Range<T: Serialize + Clone + for<'de> Deserialize<'de> + Send + Sync + 'static>(Serialize, Deserialize));

impl<T: Reflect> List for Vec<T> {
    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        <[T]>::get(self, index).map(|value| value as &dyn Reflect)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        <[T]>::get_mut(self, index).map(|value| value as &mut dyn Reflect)
    }

    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    fn iter(&self) -> ListIter {
        ListIter {
            list: self,
            index: 0,
        }
    }

    fn push(&mut self, value: Box<dyn Reflect>) {
        let value = value.take::<T>().unwrap_or_else(|value| {
            panic!(
                "Attempted to push invalid value of type {}.",
                value.type_name()
            )
        });
        Vec::push(self, value);
    }
}

impl<T: Reflect> Reflect for Vec<T> {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn any(&self) -> &dyn Any {
        self
    }

    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        crate::list_apply(self, value);
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::List(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::List(self)
    }

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::list_partial_eq(self, value)
    }

    fn serializable(&self) -> Option<Serializable> {
        None
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }
}

impl<K: Reflect + Clone + Eq + Hash, V: Reflect + Clone> Map for HashMap<K, V> {
    fn get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
        key.downcast_ref::<K>()
            .and_then(|key| HashMap::get(self, key))
            .map(|value| value as &dyn Reflect)
    }

    fn get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
        key.downcast_ref::<K>()
            .and_then(move |key| HashMap::get_mut(self, key))
            .map(|value| value as &mut dyn Reflect)
    }

    fn get_at(&self, index: usize) -> Option<(&dyn Reflect, &dyn Reflect)> {
        self.iter()
            .nth(index)
            .map(|(key, value)| (key as &dyn Reflect, value as &dyn Reflect))
    }

    fn len(&self) -> usize {
        HashMap::len(self)
    }

    fn iter(&self) -> MapIter {
        MapIter {
            map: self,
            index: 0,
        }
    }

    fn clone_dynamic(&self) -> DynamicMap {
        let mut dynamic_map = DynamicMap::default();
        for (k, v) in HashMap::iter(self) {
            dynamic_map.insert_boxed(k.clone_value(), v.clone_value());
        }
        dynamic_map
    }
}

impl<K: Reflect + Clone + Eq + Hash, V: Reflect + Clone> Reflect for HashMap<K, V> {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn any(&self) -> &dyn Any {
        self
    }

    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        if let ReflectRef::Map(map_value) = value.reflect_ref() {
            for (key, value) in map_value.iter() {
                if let Some(v) = Map::get_mut(self, key) {
                    v.apply(value)
                }
            }
        } else {
            panic!("Attempted to apply a non-map type to a map type.");
        }
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Map(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Map(self)
    }

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        map_partial_eq(self, value)
    }

    fn serializable(&self) -> Option<Serializable> {
        None
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }
}

impl Reflect for Cow<'static, str> {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn any(&self) -> &dyn Any {
        self
    }

    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        let value = value.any();
        if let Some(value) = value.downcast_ref::<Self>() {
            *self = value.clone();
        } else {
            panic!("Value is not a {}.", std::any::type_name::<Self>());
        }
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Value(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Value(self)
    }

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone())
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = crate::ReflectHasher::default();
        Hash::hash(&std::any::Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        let value = value.any();
        if let Some(value) = value.downcast_ref::<Self>() {
            Some(std::cmp::PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }

    fn serializable(&self) -> Option<Serializable> {
        Some(Serializable::Borrowed(self))
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }
}

impl<T: Reflect + Clone + Send + Sync + 'static> GetTypeRegistration for Option<T> {
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Option<T>>()
    }
}
impl<T: Reflect + Clone + Send + Sync + 'static> Enum for Option<T> {
    fn variant(&self) -> EnumVariant<'_> {
        match self {
            Option::Some(new_type) => EnumVariant::NewType(new_type as &dyn Reflect),
            Option::None => EnumVariant::Unit,
        }
    }

    fn variant_mut(&mut self) -> EnumVariantMut<'_> {
        match self {
            Option::Some(new_type) => EnumVariantMut::NewType(new_type as &mut dyn Reflect),
            Option::None => EnumVariantMut::Unit,
        }
    }

    fn variant_info(&self) -> VariantInfo<'_> {
        let index = match self {
            Option::Some(_) => 0usize,
            Option::None => 1usize,
        };
        VariantInfo {
            index,
            name: self.get_index_name(index).unwrap(),
        }
    }

    fn get_index_name(&self, index: usize) -> Option<&'_ str> {
        match index {
            0usize => Some("Option::Some"),
            1usize => Some("Option::None"),
            _ => None,
        }
    }

    fn get_index_from_name(&self, name: &str) -> Option<usize> {
        match name {
            "Option::Some" => Some(0usize),
            "Option::None" => Some(1usize),
            _ => None,
        }
    }

    fn iter_variants_info(&self) -> VariantInfoIter<'_> {
        VariantInfoIter::new(self)
    }
}
impl<T: Reflect + Clone + Send + Sync + 'static> Reflect for Option<T> {
    #[inline]
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    #[inline]
    fn any(&self) -> &dyn std::any::Any {
        self
    }

    #[inline]
    fn any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone())
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    #[inline]
    fn apply(&mut self, value: &dyn Reflect) {
        let value = value.any();
        if let Some(value) = value.downcast_ref::<Self>() {
            *self = value.clone();
        } else {
            {
                panic!("Enum is not {}.", &std::any::type_name::<Self>());
            };
        }
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Enum(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Enum(self)
    }

    fn serializable(&self) -> Option<Serializable> {
        None
    }

    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::enum_partial_eq(self, value)
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }
}
