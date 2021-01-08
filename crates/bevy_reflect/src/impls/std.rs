use crate::{
    map_partial_eq, serde::Serializable, DynamicMap, List, ListIter, Map, MapIter, Reflect,
    ReflectDeserialize, ReflectMut, ReflectRef,
};

use bevy_reflect_derive::impl_reflect_value;
use bevy_utils::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::{any::Any, hash::Hash, ops::Range};

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
impl_reflect_value!(Option<T: Serialize + Clone + for<'de> Deserialize<'de> + Reflect + 'static>(Serialize, Deserialize));
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
}
