use crate as bevy_reflect;
use crate::{
    map_partial_eq, serde::Serializable, DynamicMap, FromReflect, FromType, GetTypeRegistration,
    List, ListIter, Map, MapIter, Reflect, ReflectDeserialize, ReflectMut, ReflectRef,
    TypeRegistration,
};

use bevy_reflect_derive::{impl_from_reflect_value, impl_reflect_value};
use bevy_utils::{Duration, HashMap, HashSet};
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
impl_reflect_value!(Option<T: Serialize + Clone + for<'de> Deserialize<'de> + Reflect + 'static>(Serialize, Deserialize));
impl_reflect_value!(HashSet<T: Serialize + Hash + Eq + Clone + for<'de> Deserialize<'de> + Send + Sync + 'static>(Serialize, Deserialize));
impl_reflect_value!(Range<T: Serialize + Clone + for<'de> Deserialize<'de> + Send + Sync + 'static>(Serialize, Deserialize));
impl_reflect_value!(Duration(Hash, PartialEq, Serialize, Deserialize));

impl_from_reflect_value!(bool);
impl_from_reflect_value!(u8);
impl_from_reflect_value!(u16);
impl_from_reflect_value!(u32);
impl_from_reflect_value!(u64);
impl_from_reflect_value!(u128);
impl_from_reflect_value!(usize);
impl_from_reflect_value!(i8);
impl_from_reflect_value!(i16);
impl_from_reflect_value!(i32);
impl_from_reflect_value!(i64);
impl_from_reflect_value!(i128);
impl_from_reflect_value!(isize);
impl_from_reflect_value!(f32);
impl_from_reflect_value!(f64);
impl_from_reflect_value!(String);
impl_from_reflect_value!(
    Option<T: Serialize + Clone + for<'de> Deserialize<'de> + Reflect + 'static>
);
impl_from_reflect_value!(
    HashSet<T: Serialize + Hash + Eq + Clone + for<'de> Deserialize<'de> + Send + Sync + 'static>
);
impl_from_reflect_value!(
    Range<T: Serialize + Clone + for<'de> Deserialize<'de> + Send + Sync + 'static>
);
impl_from_reflect_value!(Duration);

impl<T: FromReflect> List for Vec<T> {
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
            T::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to push invalid value of type {}.",
                    value.type_name()
                )
            })
        });
        Vec::push(self, value);
    }
}

// SAFE: any and any_mut both return self
unsafe impl<T: FromReflect> Reflect for Vec<T> {
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

impl<T: FromReflect + for<'de> Deserialize<'de>> GetTypeRegistration for Vec<T> {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Vec<T>>();
        registration.insert::<ReflectDeserialize>(FromType::<Vec<T>>::from_type());
        registration
    }
}

impl<T: FromReflect> FromReflect for Vec<T> {
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
        if let ReflectRef::List(ref_list) = reflect.reflect_ref() {
            let mut new_list = Self::with_capacity(ref_list.len());
            for field in ref_list.iter() {
                new_list.push(T::from_reflect(field)?);
            }
            Some(new_list)
        } else {
            None
        }
    }
}

impl<K: Reflect + Eq + Hash, V: Reflect> Map for HashMap<K, V> {
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
        Self::len(self)
    }

    fn iter(&self) -> MapIter {
        MapIter {
            map: self,
            index: 0,
        }
    }

    fn clone_dynamic(&self) -> DynamicMap {
        let mut dynamic_map = DynamicMap::default();
        dynamic_map.set_name(self.type_name().to_string());
        for (k, v) in self {
            dynamic_map.insert_boxed(k.clone_value(), v.clone_value());
        }
        dynamic_map
    }
}

// SAFE: any and any_mut both return self
unsafe impl<K: Reflect + Eq + Hash, V: Reflect> Reflect for HashMap<K, V> {
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

impl<K, V> GetTypeRegistration for HashMap<K, V>
where
    K: Reflect + Clone + Eq + Hash + for<'de> Deserialize<'de>,
    V: Reflect + Clone + for<'de> Deserialize<'de>,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectDeserialize>(FromType::<Self>::from_type());
        registration
    }
}

impl<K: FromReflect + Eq + Hash, V: FromReflect> FromReflect for HashMap<K, V> {
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
        if let ReflectRef::Map(ref_map) = reflect.reflect_ref() {
            let mut new_map = Self::with_capacity(ref_map.len());
            for (key, value) in ref_map.iter() {
                let new_key = K::from_reflect(key)?;
                let new_value = V::from_reflect(value)?;
                new_map.insert(new_key, new_value);
            }
            Some(new_map)
        } else {
            None
        }
    }
}

// SAFE: any and any_mut both return self
unsafe impl Reflect for Cow<'static, str> {
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
}

impl GetTypeRegistration for Cow<'static, str> {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Cow<'static, str>>();
        registration.insert::<ReflectDeserialize>(FromType::<Cow<'static, str>>::from_type());
        registration
    }
}

impl FromReflect for Cow<'static, str> {
    fn from_reflect(reflect: &dyn crate::Reflect) -> Option<Self> {
        Some(reflect.any().downcast_ref::<Cow<'static, str>>()?.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::Reflect;

    #[test]
    fn can_serialize_duration() {
        assert!(std::time::Duration::ZERO.serializable().is_some());
    }
}
