use crate as bevy_reflect;
use crate::{
    map_partial_eq, Array, ArrayInfo, ArrayIter, DynamicMap, FromReflect, FromType,
    GetTypeRegistration, List, ListInfo, Map, MapInfo, MapIter, Reflect, ReflectDeserialize,
    ReflectMut, ReflectRef, ReflectSerialize, TypeInfo, TypeRegistration, Typed, ValueInfo,
};

use crate::utility::{GenericTypeInfoCell, NonGenericTypeInfoCell};
use bevy_reflect_derive::{impl_from_reflect_value, impl_reflect_value};
use bevy_utils::{Duration, HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    borrow::Cow,
    hash::{Hash, Hasher},
    ops::Range,
};

impl_reflect_value!(bool(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(char(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(u8(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(u16(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(u32(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(u64(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(u128(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(usize(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(i8(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(i16(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(i32(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(i64(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(i128(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(isize(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(f32(Debug, PartialEq, Serialize, Deserialize));
impl_reflect_value!(f64(Debug, PartialEq, Serialize, Deserialize));
impl_reflect_value!(String(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(Option<T: Serialize + Clone + for<'de> Deserialize<'de> + Reflect + 'static>(Serialize, Deserialize));
impl_reflect_value!(HashSet<T: Serialize + Hash + Eq + Clone + for<'de> Deserialize<'de> + Send + Sync + 'static>(Serialize, Deserialize));
impl_reflect_value!(Range<T: Serialize + Clone + for<'de> Deserialize<'de> + Send + Sync + 'static>(Serialize, Deserialize));
impl_reflect_value!(Duration(Debug, Hash, PartialEq, Serialize, Deserialize));

impl_from_reflect_value!(bool);
impl_from_reflect_value!(char);
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

impl<T: FromReflect> Array for Vec<T> {
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        <[T]>::get(self, index).map(|value| value as &dyn Reflect)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        <[T]>::get_mut(self, index).map(|value| value as &mut dyn Reflect)
    }

    #[inline]
    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    #[inline]
    fn iter(&self) -> ArrayIter {
        ArrayIter {
            array: self,
            index: 0,
        }
    }
}

impl<T: FromReflect> List for Vec<T> {
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

impl<T: FromReflect> Reflect for Vec<T> {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_type_info(&self) -> &'static TypeInfo {
        <Self as Typed>::type_info()
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

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
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
        Box::new(List::clone_dynamic(self))
    }

    fn reflect_hash(&self) -> Option<u64> {
        crate::array_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::list_partial_eq(self, value)
    }
}

impl<T: FromReflect> Typed for Vec<T> {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::List(ListInfo::new::<Self, T>()))
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

impl<K: Reflect + Eq + Hash, V: Reflect> Reflect for HashMap<K, V> {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_type_info(&self) -> &'static TypeInfo {
        <Self as Typed>::type_info()
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

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        if let ReflectRef::Map(map_value) = value.reflect_ref() {
            for (key, value) in map_value.iter() {
                if let Some(v) = Map::get_mut(self, key) {
                    v.apply(value);
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

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        map_partial_eq(self, value)
    }
}

impl<K: Reflect + Eq + Hash, V: Reflect> Typed for HashMap<K, V> {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::Map(MapInfo::new::<Self, K, V>()))
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

impl<T: Reflect, const N: usize> Array for [T; N] {
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        <[T]>::get(self, index).map(|value| value as &dyn Reflect)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        <[T]>::get_mut(self, index).map(|value| value as &mut dyn Reflect)
    }

    #[inline]
    fn len(&self) -> usize {
        N
    }

    #[inline]
    fn iter(&self) -> ArrayIter {
        ArrayIter {
            array: self,
            index: 0,
        }
    }
}

impl<T: Reflect, const N: usize> Reflect for [T; N] {
    #[inline]
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_type_info(&self) -> &'static TypeInfo {
        <Self as Typed>::type_info()
    }

    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    #[inline]
    fn apply(&mut self, value: &dyn Reflect) {
        crate::array_apply(self, value);
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Array(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Array(self)
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        crate::array_hash(self)
    }

    #[inline]
    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::array_partial_eq(self, value)
    }
}

impl<T: FromReflect, const N: usize> FromReflect for [T; N] {
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
        if let ReflectRef::Array(ref_array) = reflect.reflect_ref() {
            let mut temp_vec = Vec::with_capacity(ref_array.len());
            for field in ref_array.iter() {
                temp_vec.push(T::from_reflect(field)?);
            }
            temp_vec.try_into().ok()
        } else {
            None
        }
    }
}

impl<T: Reflect, const N: usize> Typed for [T; N] {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::Array(ArrayInfo::new::<Self, T>(N)))
    }
}

// TODO:
// `FromType::from_type` requires `Deserialize<'de>` to be implemented for `T`.
// Currently serde only supports `Deserialize<'de>` for arrays up to size 32.
// This can be changed to use const generics once serde utilizes const generics for arrays.
// Tracking issue: https://github.com/serde-rs/serde/issues/1937
macro_rules! impl_array_get_type_registration {
    ($($N:expr)+) => {
        $(
            impl<T: Reflect + for<'de> Deserialize<'de>> GetTypeRegistration for [T; $N] {
                fn get_type_registration() -> TypeRegistration {
                    let mut registration = TypeRegistration::of::<[T; $N]>();
                    registration.insert::<ReflectDeserialize>(FromType::<[T; $N]>::from_type());
                    registration
                }
            }
        )+
    };
}

impl_array_get_type_registration! {
     0  1  2  3  4  5  6  7  8  9
    10 11 12 13 14 15 16 17 18 19
    20 21 22 23 24 25 26 27 28 29
    30 31 32
}

impl Reflect for Cow<'static, str> {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_type_info(&self) -> &'static TypeInfo {
        <Self as Typed>::type_info()
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

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        let value = value.as_any();
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
        let value = value.as_any();
        if let Some(value) = value.downcast_ref::<Self>() {
            Some(std::cmp::PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }
}

impl Typed for Cow<'static, str> {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Value(ValueInfo::new::<Self>()))
    }
}

impl GetTypeRegistration for Cow<'static, str> {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Cow<'static, str>>();
        registration.insert::<ReflectDeserialize>(FromType::<Cow<'static, str>>::from_type());
        registration.insert::<ReflectSerialize>(FromType::<Cow<'static, str>>::from_type());
        registration
    }
}

impl FromReflect for Cow<'static, str> {
    fn from_reflect(reflect: &dyn crate::Reflect) -> Option<Self> {
        Some(
            reflect
                .as_any()
                .downcast_ref::<Cow<'static, str>>()?
                .clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{Reflect, ReflectSerialize, TypeRegistry};
    use bevy_utils::HashMap;
    use std::f32::consts::{PI, TAU};

    #[test]
    fn can_serialize_duration() {
        let mut type_registry = TypeRegistry::default();
        type_registry.register::<std::time::Duration>();

        let reflect_serialize = type_registry
            .get_type_data::<ReflectSerialize>(std::any::TypeId::of::<std::time::Duration>())
            .unwrap();
        let _serializable = reflect_serialize.get_serializable(&std::time::Duration::ZERO);
    }

    #[test]
    fn should_partial_eq_char() {
        let a: &dyn Reflect = &'x';
        let b: &dyn Reflect = &'x';
        let c: &dyn Reflect = &'o';
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_i32() {
        let a: &dyn Reflect = &123_i32;
        let b: &dyn Reflect = &123_i32;
        let c: &dyn Reflect = &321_i32;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_f32() {
        let a: &dyn Reflect = &PI;
        let b: &dyn Reflect = &PI;
        let c: &dyn Reflect = &TAU;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_string() {
        let a: &dyn Reflect = &String::from("Hello");
        let b: &dyn Reflect = &String::from("Hello");
        let c: &dyn Reflect = &String::from("World");
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_vec() {
        let a: &dyn Reflect = &vec![1, 2, 3];
        let b: &dyn Reflect = &vec![1, 2, 3];
        let c: &dyn Reflect = &vec![3, 2, 1];
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_hash_map() {
        let mut a = HashMap::new();
        a.insert(0usize, 1.23_f64);
        let b = a.clone();
        let mut c = HashMap::new();
        c.insert(0usize, 3.21_f64);

        let a: &dyn Reflect = &a;
        let b: &dyn Reflect = &b;
        let c: &dyn Reflect = &c;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_not_partial_eq_option() {
        // Option<T> does not contain a `PartialEq` implementation, so it should return `None`
        let a: &dyn Reflect = &Some(123);
        let b: &dyn Reflect = &Some(123);
        assert_eq!(None, a.reflect_partial_eq(b));
    }
}
