use crate::std_traits::ReflectDefault;
use crate::{self as bevy_reflect, ReflectFromPtr, ReflectOwned};
use crate::{
    map_apply, map_partial_eq, Array, ArrayInfo, ArrayIter, DynamicEnum, DynamicMap, Enum,
    EnumInfo, FromReflect, FromType, GetTypeRegistration, List, ListInfo, Map, MapInfo, MapIter,
    Reflect, ReflectDeserialize, ReflectMut, ReflectRef, ReflectSerialize, TupleVariantInfo,
    TypeInfo, TypeRegistration, Typed, UnitVariantInfo, UnnamedField, ValueInfo, VariantFieldIter,
    VariantInfo, VariantType,
};

use crate::utility::{GenericTypeInfoCell, NonGenericTypeInfoCell};
use bevy_reflect_derive::{impl_from_reflect_value, impl_reflect_value};
use bevy_utils::{Duration, Instant};
use bevy_utils::{HashMap, HashSet};
#[cfg(any(unix, windows))]
use std::ffi::OsString;
use std::{
    any::Any,
    borrow::Cow,
    hash::{Hash, Hasher},
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
    ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
    path::PathBuf,
};

impl_reflect_value!(bool(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(char(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(u8(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(u16(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(u32(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(u64(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(u128(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(usize(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(i8(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(i16(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(i32(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(i64(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(i128(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(isize(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(f32(Debug, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(f64(Debug, PartialEq, Serialize, Deserialize, Default));
impl_reflect_value!(String(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(PathBuf(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(Result<T: Clone + Reflect + 'static, E: Clone + Reflect + 'static>());
impl_reflect_value!(HashSet<T: Hash + Eq + Clone + Send + Sync + 'static>());
impl_reflect_value!(Range<T: Clone + Send + Sync + 'static>());
impl_reflect_value!(RangeInclusive<T: Clone + Send + Sync + 'static>());
impl_reflect_value!(RangeFrom<T: Clone + Send + Sync + 'static>());
impl_reflect_value!(RangeTo<T: Clone + Send + Sync + 'static>());
impl_reflect_value!(RangeToInclusive<T: Clone + Send + Sync + 'static>());
impl_reflect_value!(RangeFull());
impl_reflect_value!(Duration(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(Instant(Debug, Hash, PartialEq));
impl_reflect_value!(NonZeroI128(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroU128(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroIsize(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroUsize(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroI64(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroU64(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroU32(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroI32(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroI16(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroU16(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroU8(Debug, Hash, PartialEq, Serialize, Deserialize));
impl_reflect_value!(NonZeroI8(Debug, Hash, PartialEq, Serialize, Deserialize));

// Only for platforms that can serialize it as in serde:
// https://github.com/serde-rs/serde/blob/3ffb86fc70efd3d329519e2dddfa306cc04f167c/serde/src/de/impls.rs#L1732
#[cfg(any(unix, windows))]
impl_reflect_value!(OsString(Debug, Hash, PartialEq, Serialize, Deserialize));

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
impl_from_reflect_value!(HashSet<T: Hash + Eq + Clone + Send + Sync + 'static>);
impl_from_reflect_value!(Range<T: Clone + Send + Sync + 'static>);
impl_from_reflect_value!(RangeInclusive<T: Clone + Send + Sync + 'static>);
impl_from_reflect_value!(RangeFrom<T: Clone + Send + Sync + 'static>);
impl_from_reflect_value!(RangeTo<T: Clone + Send + Sync + 'static>);
impl_from_reflect_value!(RangeToInclusive<T: Clone + Send + Sync + 'static>);
impl_from_reflect_value!(RangeFull);
impl_from_reflect_value!(Duration);
impl_from_reflect_value!(Instant);
impl_from_reflect_value!(NonZeroI128);
impl_from_reflect_value!(NonZeroU128);
impl_from_reflect_value!(NonZeroIsize);
impl_from_reflect_value!(NonZeroUsize);
impl_from_reflect_value!(NonZeroI64);
impl_from_reflect_value!(NonZeroU64);
impl_from_reflect_value!(NonZeroU32);
impl_from_reflect_value!(NonZeroI32);
impl_from_reflect_value!(NonZeroI16);
impl_from_reflect_value!(NonZeroU16);
impl_from_reflect_value!(NonZeroU8);
impl_from_reflect_value!(NonZeroI8);

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

    #[inline]
    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
        self.into_iter()
            .map(|value| Box::new(value) as Box<dyn Reflect>)
            .collect()
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

    fn pop(&mut self) -> Option<Box<dyn Reflect>> {
        self.pop().map(|value| Box::new(value) as Box<dyn Reflect>)
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

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
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

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::List(self)
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

impl<T: FromReflect> GetTypeRegistration for Vec<T> {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Vec<T>>();
        registration.insert::<ReflectFromPtr>(FromType::<Vec<T>>::from_type());
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

impl<K: FromReflect + Eq + Hash, V: FromReflect> Map for HashMap<K, V> {
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

    fn drain(self: Box<Self>) -> Vec<(Box<dyn Reflect>, Box<dyn Reflect>)> {
        self.into_iter()
            .map(|(key, value)| {
                (
                    Box::new(key) as Box<dyn Reflect>,
                    Box::new(value) as Box<dyn Reflect>,
                )
            })
            .collect()
    }

    fn clone_dynamic(&self) -> DynamicMap {
        let mut dynamic_map = DynamicMap::default();
        dynamic_map.set_name(self.type_name().to_string());
        for (k, v) in self {
            dynamic_map.insert_boxed(k.clone_value(), v.clone_value());
        }
        dynamic_map
    }

    fn insert_boxed(
        &mut self,
        key: Box<dyn Reflect>,
        value: Box<dyn Reflect>,
    ) -> Option<Box<dyn Reflect>> {
        let key = key.take::<K>().unwrap_or_else(|key| {
            K::from_reflect(&*key).unwrap_or_else(|| {
                panic!(
                    "Attempted to insert invalid key of type {}.",
                    key.type_name()
                )
            })
        });
        let value = value.take::<V>().unwrap_or_else(|value| {
            V::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to insert invalid value of type {}.",
                    value.type_name()
                )
            })
        });
        self.insert(key, value)
            .map(|old_value| Box::new(old_value) as Box<dyn Reflect>)
    }
}

impl<K: FromReflect + Eq + Hash, V: FromReflect> Reflect for HashMap<K, V> {
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

    #[inline]
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        map_apply(self, value);
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

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Map(self)
    }

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        map_partial_eq(self, value)
    }
}

impl<K: FromReflect + Eq + Hash, V: FromReflect> Typed for HashMap<K, V> {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::Map(MapInfo::new::<Self, K, V>()))
    }
}

impl<K, V> GetTypeRegistration for HashMap<K, V>
where
    K: FromReflect + Eq + Hash,
    V: FromReflect,
{
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<HashMap<K, V>>();
        registration.insert::<ReflectFromPtr>(FromType::<HashMap<K, V>>::from_type());
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

    #[inline]
    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
        self.into_iter()
            .map(|value| Box::new(value) as Box<dyn Reflect>)
            .collect()
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
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
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
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Array(self)
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
            impl<T: Reflect > GetTypeRegistration for [T; $N] {
                fn get_type_registration() -> TypeRegistration {
                    TypeRegistration::of::<[T; $N]>()
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

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
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

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Value(self)
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

impl<T: FromReflect> GetTypeRegistration for Option<T> {
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Option<T>>()
    }
}

impl<T: FromReflect> Enum for Option<T> {
    fn field(&self, _name: &str) -> Option<&dyn Reflect> {
        None
    }

    fn field_at(&self, index: usize) -> Option<&dyn Reflect> {
        match self {
            Some(value) if index == 0 => Some(value),
            _ => None,
        }
    }

    fn field_mut(&mut self, _name: &str) -> Option<&mut dyn Reflect> {
        None
    }

    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        match self {
            Some(value) if index == 0 => Some(value),
            _ => None,
        }
    }

    fn index_of(&self, _name: &str) -> Option<usize> {
        None
    }

    fn name_at(&self, _index: usize) -> Option<&str> {
        None
    }

    fn iter_fields(&self) -> VariantFieldIter {
        VariantFieldIter::new(self)
    }

    #[inline]
    fn field_len(&self) -> usize {
        match self {
            Some(..) => 1,
            None => 0,
        }
    }

    #[inline]
    fn variant_name(&self) -> &str {
        match self {
            Some(..) => "Some",
            None => "None",
        }
    }

    fn variant_index(&self) -> usize {
        match self {
            None => 0,
            Some(..) => 1,
        }
    }

    #[inline]
    fn variant_type(&self) -> VariantType {
        match self {
            Some(..) => VariantType::Tuple,
            None => VariantType::Unit,
        }
    }

    fn clone_dynamic(&self) -> DynamicEnum {
        DynamicEnum::from_ref::<Self>(self)
    }
}

impl<T: FromReflect> Reflect for Option<T> {
    #[inline]
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    #[inline]
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
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    #[inline]
    fn apply(&mut self, value: &dyn Reflect) {
        if let ReflectRef::Enum(value) = value.reflect_ref() {
            if self.variant_name() == value.variant_name() {
                // Same variant -> just update fields
                for (index, field) in value.iter_fields().enumerate() {
                    if let Some(v) = self.field_at_mut(index) {
                        v.apply(field.value());
                    }
                }
            } else {
                // New variant -> perform a switch
                match value.variant_name() {
                    "Some" => {
                        let field = value
                            .field_at(0)
                            .unwrap_or_else(|| {
                                panic!(
                                    "Field in `Some` variant of {} should exist",
                                    std::any::type_name::<Option<T>>()
                                )
                            })
                            .clone_value()
                            .take::<T>()
                            .unwrap_or_else(|value| {
                                T::from_reflect(&*value).unwrap_or_else(|| {
                                    panic!(
                                        "Field in `Some` variant of {} should be of type {}",
                                        std::any::type_name::<Option<T>>(),
                                        std::any::type_name::<T>()
                                    )
                                })
                            });
                        *self = Some(field);
                    }
                    "None" => {
                        *self = None;
                    }
                    _ => panic!("Enum is not a {}.", std::any::type_name::<Self>()),
                }
            }
        }
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Enum(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Enum(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Enum(self)
    }

    #[inline]
    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(Enum::clone_dynamic(self))
    }

    fn reflect_hash(&self) -> Option<u64> {
        crate::enum_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::enum_partial_eq(self, value)
    }
}

impl<T: FromReflect> FromReflect for Option<T> {
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
        if let ReflectRef::Enum(dyn_enum) = reflect.reflect_ref() {
            match dyn_enum.variant_name() {
                "Some" => {
                    let field = dyn_enum
                        .field_at(0)
                        .unwrap_or_else(|| {
                            panic!(
                                "Field in `Some` variant of {} should exist",
                                std::any::type_name::<Option<T>>()
                            )
                        })
                        .clone_value()
                        .take::<T>()
                        .unwrap_or_else(|value| {
                            T::from_reflect(&*value).unwrap_or_else(|| {
                                panic!(
                                    "Field in `Some` variant of {} should be of type {}",
                                    std::any::type_name::<Option<T>>(),
                                    std::any::type_name::<T>()
                                )
                            })
                        });
                    Some(Some(field))
                }
                "None" => Some(None),
                name => panic!(
                    "variant with name `{}` does not exist on enum `{}`",
                    name,
                    std::any::type_name::<Self>()
                ),
            }
        } else {
            None
        }
    }
}

impl<T: FromReflect> Typed for Option<T> {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            let none_variant = VariantInfo::Unit(UnitVariantInfo::new("None"));
            let some_variant =
                VariantInfo::Tuple(TupleVariantInfo::new("Some", &[UnnamedField::new::<T>(0)]));
            TypeInfo::Enum(EnumInfo::new::<Self>(
                "Option",
                &[none_variant, some_variant],
            ))
        })
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
        registration.insert::<ReflectFromPtr>(FromType::<Cow<'static, str>>::from_type());
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
    use crate as bevy_reflect;
    use crate::{
        Enum, FromReflect, Reflect, ReflectSerialize, TypeInfo, TypeRegistry, Typed, VariantInfo,
        VariantType,
    };
    use bevy_utils::HashMap;
    use bevy_utils::{Duration, Instant};
    use std::f32::consts::{PI, TAU};

    #[test]
    fn can_serialize_duration() {
        let mut type_registry = TypeRegistry::default();
        type_registry.register::<Duration>();

        let reflect_serialize = type_registry
            .get_type_data::<ReflectSerialize>(std::any::TypeId::of::<Duration>())
            .unwrap();
        let _serializable = reflect_serialize.get_serializable(&Duration::ZERO);
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
    fn should_partial_eq_option() {
        let a: &dyn Reflect = &Some(123);
        let b: &dyn Reflect = &Some(123);
        assert_eq!(Some(true), a.reflect_partial_eq(b));
    }

    #[test]
    fn option_should_impl_enum() {
        let mut value = Some(123usize);

        assert!(value
            .reflect_partial_eq(&Some(123usize))
            .unwrap_or_default());
        assert!(!value
            .reflect_partial_eq(&Some(321usize))
            .unwrap_or_default());

        assert_eq!("Some", value.variant_name());
        assert_eq!("core::option::Option<usize>::Some", value.variant_path());

        if value.is_variant(VariantType::Tuple) {
            if let Some(field) = value
                .field_at_mut(0)
                .and_then(|field| field.downcast_mut::<usize>())
            {
                *field = 321;
            }
        } else {
            panic!("expected `VariantType::Tuple`");
        }

        assert_eq!(Some(321), value);
    }

    #[test]
    fn option_should_from_reflect() {
        #[derive(Reflect, FromReflect, PartialEq, Debug)]
        struct Foo(usize);

        let expected = Some(Foo(123));
        let output = <Option<Foo> as FromReflect>::from_reflect(&expected).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn option_should_apply() {
        #[derive(Reflect, FromReflect, PartialEq, Debug)]
        struct Foo(usize);

        // === None on None === //
        let patch = None::<Foo>;
        let mut value = None;
        Reflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "None apply onto None");

        // === Some on None === //
        let patch = Some(Foo(123));
        let mut value = None;
        Reflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "Some apply onto None");

        // === None on Some === //
        let patch = None::<Foo>;
        let mut value = Some(Foo(321));
        Reflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "None apply onto Some");

        // === Some on Some === //
        let patch = Some(Foo(123));
        let mut value = Some(Foo(321));
        Reflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "Some apply onto Some");
    }

    #[test]
    fn option_should_impl_typed() {
        type MyOption = Option<i32>;
        let info = MyOption::type_info();
        if let TypeInfo::Enum(info) = info {
            assert_eq!(
                "None",
                info.variant_at(0).unwrap().name(),
                "Expected `None` to be variant at index `0`"
            );
            assert_eq!(
                "Some",
                info.variant_at(1).unwrap().name(),
                "Expected `Some` to be variant at index `1`"
            );
            assert_eq!("Some", info.variant("Some").unwrap().name());
            if let VariantInfo::Tuple(variant) = info.variant("Some").unwrap() {
                assert!(
                    variant.field_at(0).unwrap().is::<i32>(),
                    "Expected `Some` variant to contain `i32`"
                );
                assert!(
                    variant.field_at(1).is_none(),
                    "Expected `Some` variant to only contain 1 field"
                );
            } else {
                panic!("Expected `VariantInfo::Tuple`");
            }
        } else {
            panic!("Expected `TypeInfo::Enum`");
        }
    }
    #[test]
    fn nonzero_usize_impl_reflect_from_reflect() {
        let a: &dyn Reflect = &std::num::NonZeroUsize::new(42).unwrap();
        let b: &dyn Reflect = &std::num::NonZeroUsize::new(42).unwrap();
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        let forty_two: std::num::NonZeroUsize = crate::FromReflect::from_reflect(a).unwrap();
        assert_eq!(forty_two, std::num::NonZeroUsize::new(42).unwrap());
    }

    #[test]
    fn instant_should_from_reflect() {
        let expected = Instant::now();
        let output = <Instant as FromReflect>::from_reflect(&expected).unwrap();
        assert_eq!(expected, output);
    }
}
