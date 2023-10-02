use crate::std_traits::ReflectDefault;
use crate::{self as bevy_reflect, ReflectFromPtr, ReflectFromReflect, ReflectOwned};
use crate::{
    impl_type_path, map_apply, map_partial_eq, Array, ArrayInfo, ArrayIter, DynamicEnum,
    DynamicMap, Enum, EnumInfo, FromReflect, FromType, GetTypeRegistration, List, ListInfo,
    ListIter, Map, MapInfo, MapIter, Reflect, ReflectDeserialize, ReflectMut, ReflectRef,
    ReflectSerialize, TupleVariantInfo, TypeInfo, TypePath, TypeRegistration, Typed,
    UnitVariantInfo, UnnamedField, ValueInfo, VariantFieldIter, VariantInfo, VariantType,
};

use crate::utility::{
    reflect_hasher, GenericTypeInfoCell, GenericTypePathCell, NonGenericTypeInfoCell,
};
use bevy_reflect_derive::impl_reflect_value;
use std::fmt;
use std::{
    any::Any,
    borrow::Cow,
    collections::VecDeque,
    hash::{BuildHasher, Hash, Hasher},
    path::Path,
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
impl_reflect_value!(::std::path::PathBuf(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(
    ::core::result::Result < T: Clone + Reflect + TypePath,
    E: Clone + Reflect + TypePath > ()
);
impl_reflect_value!(::bevy_utils::HashSet<T: Hash + Eq + Clone + Send + Sync>());
impl_reflect_value!(::core::ops::Range<T: Clone + Send + Sync>());
impl_reflect_value!(::core::ops::RangeInclusive<T: Clone + Send + Sync>());
impl_reflect_value!(::core::ops::RangeFrom<T: Clone + Send + Sync>());
impl_reflect_value!(::core::ops::RangeTo<T: Clone + Send + Sync>());
impl_reflect_value!(::core::ops::RangeToInclusive<T: Clone + Send + Sync>());
impl_reflect_value!(::core::ops::RangeFull());
impl_reflect_value!(::bevy_utils::Duration(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_value!(::bevy_utils::Instant(Debug, Hash, PartialEq));
impl_reflect_value!(::core::num::NonZeroI128(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroU128(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroIsize(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroUsize(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroI64(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroU64(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroU32(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroI32(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroI16(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroU16(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroU8(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::core::num::NonZeroI8(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_value!(::std::sync::Arc<T: Send + Sync>);

// `Serialize` and `Deserialize` only for platforms supported by serde:
// https://github.com/serde-rs/serde/blob/3ffb86fc70efd3d329519e2dddfa306cc04f167c/serde/src/de/impls.rs#L1732
#[cfg(any(unix, windows))]
impl_reflect_value!(::std::ffi::OsString(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
#[cfg(not(any(unix, windows)))]
impl_reflect_value!(::std::ffi::OsString(Debug, Hash, PartialEq));

macro_rules! impl_reflect_for_veclike {
    ($ty:path, $insert:expr, $remove:expr, $push:expr, $pop:expr, $sub:ty) => {
        impl<T: FromReflect + TypePath> List for $ty {
            #[inline]
            fn get(&self, index: usize) -> Option<&dyn Reflect> {
                <$sub>::get(self, index).map(|value| value as &dyn Reflect)
            }

            #[inline]
            fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
                <$sub>::get_mut(self, index).map(|value| value as &mut dyn Reflect)
            }

            fn insert(&mut self, index: usize, value: Box<dyn Reflect>) {
                let value = value.take::<T>().unwrap_or_else(|value| {
                    T::from_reflect(&*value).unwrap_or_else(|| {
                        panic!(
                            "Attempted to insert invalid value of type {}.",
                            value.type_name()
                        )
                    })
                });
                $insert(self, index, value);
            }

            fn remove(&mut self, index: usize) -> Box<dyn Reflect> {
                Box::new($remove(self, index))
            }

            fn push(&mut self, value: Box<dyn Reflect>) {
                let value = T::take_from_reflect(value).unwrap_or_else(|value| {
                    panic!(
                        "Attempted to push invalid value of type {}.",
                        value.type_name()
                    )
                });
                $push(self, value);
            }

            fn pop(&mut self) -> Option<Box<dyn Reflect>> {
                $pop(self).map(|value| Box::new(value) as Box<dyn Reflect>)
            }

            #[inline]
            fn len(&self) -> usize {
                <$sub>::len(self)
            }

            #[inline]
            fn iter(&self) -> ListIter {
                ListIter::new(self)
            }

            #[inline]
            fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
                self.into_iter()
                    .map(|value| Box::new(value) as Box<dyn Reflect>)
                    .collect()
            }
        }

        impl<T: FromReflect + TypePath> Reflect for $ty {
            fn type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
                Some(<Self as Typed>::type_info())
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
                Box::new(self.clone_dynamic())
            }

            fn reflect_hash(&self) -> Option<u64> {
                crate::list_hash(self)
            }

            fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
                crate::list_partial_eq(self, value)
            }
        }

        impl<T: FromReflect + TypePath> Typed for $ty {
            fn type_info() -> &'static TypeInfo {
                static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
                CELL.get_or_insert::<Self, _>(|| TypeInfo::List(ListInfo::new::<Self, T>()))
            }
        }

        impl_type_path!($ty where T: FromReflect);

        impl<T: FromReflect + TypePath> GetTypeRegistration for $ty {
            fn get_type_registration() -> TypeRegistration {
                let mut registration = TypeRegistration::of::<$ty>();
                registration.insert::<ReflectFromPtr>(FromType::<$ty>::from_type());
                registration
            }
        }

        impl<T: FromReflect + TypePath> FromReflect for $ty {
            fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
                if let ReflectRef::List(ref_list) = reflect.reflect_ref() {
                    let mut new_list = Self::with_capacity(ref_list.len());
                    for field in ref_list.iter() {
                        $push(&mut new_list, T::from_reflect(field)?);
                    }
                    Some(new_list)
                } else {
                    None
                }
            }
        }
    };
}

impl_reflect_for_veclike!(
    ::alloc::vec::Vec<T>,
    Vec::insert,
    Vec::remove,
    Vec::push,
    Vec::pop,
    [T]
);
impl_reflect_for_veclike!(
    ::alloc::collections::VecDeque<T>,
    VecDeque::insert,
    VecDeque::remove,
    VecDeque::push_back,
    VecDeque::pop_back,
    VecDeque::<T>
);

macro_rules! impl_reflect_for_hashmap {
    ($ty:path) => {
        impl<K, V, S> Map for $ty
        where
            K: FromReflect + TypePath + Eq + Hash,
            V: FromReflect + TypePath,
            S: TypePath + BuildHasher + Send + Sync,
        {
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

            fn get_at(&self, index: usize) -> Option<(&dyn Reflect, &dyn Reflect)> {
                self.iter()
                    .nth(index)
                    .map(|(key, value)| (key as &dyn Reflect, value as &dyn Reflect))
            }

            fn get_at_mut(&mut self, index: usize) -> Option<(&dyn Reflect, &mut dyn Reflect)> {
                self.iter_mut()
                    .nth(index)
                    .map(|(key, value)| (key as &dyn Reflect, value as &mut dyn Reflect))
            }

            fn len(&self) -> usize {
                Self::len(self)
            }

            fn iter(&self) -> MapIter {
                MapIter::new(self)
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
                dynamic_map.set_represented_type(self.get_represented_type_info());
                for (k, v) in self {
                    let key = K::from_reflect(k).unwrap_or_else(|| {
                        panic!("Attempted to clone invalid key of type {}.", k.type_name())
                    });
                    dynamic_map.insert_boxed(Box::new(key), v.clone_value());
                }
                dynamic_map
            }

            fn insert_boxed(
                &mut self,
                key: Box<dyn Reflect>,
                value: Box<dyn Reflect>,
            ) -> Option<Box<dyn Reflect>> {
                let key = K::take_from_reflect(key).unwrap_or_else(|key| {
                    panic!(
                        "Attempted to insert invalid key of type {}.",
                        key.type_name()
                    )
                });
                let value = V::take_from_reflect(value).unwrap_or_else(|value| {
                    panic!(
                        "Attempted to insert invalid value of type {}.",
                        value.type_name()
                    )
                });
                self.insert(key, value)
                    .map(|old_value| Box::new(old_value) as Box<dyn Reflect>)
            }

            fn remove(&mut self, key: &dyn Reflect) -> Option<Box<dyn Reflect>> {
                let mut from_reflect = None;
                key.downcast_ref::<K>()
                    .or_else(|| {
                        from_reflect = K::from_reflect(key);
                        from_reflect.as_ref()
                    })
                    .and_then(|key| self.remove(key))
                    .map(|value| Box::new(value) as Box<dyn Reflect>)
            }
        }

        impl<K, V, S> Reflect for $ty
        where
            K: FromReflect + TypePath + Eq + Hash,
            V: FromReflect + TypePath,
            S: TypePath + BuildHasher + Send + Sync,
        {
            fn type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
                Some(<Self as Typed>::type_info())
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

        impl<K, V, S> Typed for $ty
        where
            K: FromReflect + TypePath + Eq + Hash,
            V: FromReflect + TypePath,
            S: TypePath + BuildHasher + Send + Sync,
        {
            fn type_info() -> &'static TypeInfo {
                static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
                CELL.get_or_insert::<Self, _>(|| TypeInfo::Map(MapInfo::new::<Self, K, V>()))
            }
        }

        impl<K, V, S> GetTypeRegistration for $ty
        where
            K: FromReflect + TypePath + Eq + Hash,
            V: FromReflect + TypePath,
            S: TypePath + BuildHasher + Send + Sync,
        {
            fn get_type_registration() -> TypeRegistration {
                let mut registration = TypeRegistration::of::<Self>();
                registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
                registration
            }
        }

        impl<K, V, S> FromReflect for $ty
        where
            K: FromReflect + TypePath + Eq + Hash,
            V: FromReflect + TypePath,
            S: TypePath + BuildHasher + Default + Send + Sync,
        {
            fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
                if let ReflectRef::Map(ref_map) = reflect.reflect_ref() {
                    let mut new_map = Self::with_capacity_and_hasher(ref_map.len(), S::default());
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
    };
}

impl_reflect_for_hashmap!(::std::collections::HashMap<K, V, S>);
impl_type_path!(::std::collections::hash_map::RandomState);
impl_type_path!(
    ::std::collections::HashMap<K, V, S>
    where
        K: FromReflect + Eq + Hash + ?Sized,
        V: FromReflect + ?Sized,
        S: BuildHasher + Send + Sync + 'static,
);

impl_reflect_for_hashmap!(bevy_utils::hashbrown::HashMap<K, V, S>);
impl_type_path!(::bevy_utils::hashbrown::hash_map::DefaultHashBuilder);
impl_type_path!(
    ::bevy_utils::hashbrown::HashMap<K, V, S>
    where
        K: FromReflect + Eq + Hash + ?Sized,
        V: FromReflect + ?Sized,
        S: BuildHasher + Send + Sync + 'static,
);

impl<T: Reflect + TypePath, const N: usize> Array for [T; N] {
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
        ArrayIter::new(self)
    }

    #[inline]
    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
        self.into_iter()
            .map(|value| Box::new(value) as Box<dyn Reflect>)
            .collect()
    }
}

impl<T: Reflect + TypePath, const N: usize> Reflect for [T; N] {
    #[inline]
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
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

impl<T: FromReflect + TypePath, const N: usize> FromReflect for [T; N] {
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

impl<T: Reflect + TypePath, const N: usize> Typed for [T; N] {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::Array(ArrayInfo::new::<Self, T>(N)))
    }
}

impl<T: Reflect + TypePath, const N: usize> TypePath for [T; N] {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("[{t}; {N}]", t = T::type_path()))
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("[{t}; {N}]", t = T::short_type_path()))
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
            impl<T: Reflect + TypePath> GetTypeRegistration for [T; $N] {
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

impl<T: FromReflect + TypePath> GetTypeRegistration for Option<T> {
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Option<T>>()
    }
}

impl<T: FromReflect + TypePath> Enum for Option<T> {
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

impl<T: FromReflect + TypePath> Reflect for Option<T> {
    #[inline]
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
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
                        let field = T::take_from_reflect(
                            value
                                .field_at(0)
                                .unwrap_or_else(|| {
                                    panic!(
                                        "Field in `Some` variant of {} should exist",
                                        std::any::type_name::<Option<T>>()
                                    )
                                })
                                .clone_value(),
                        )
                        .unwrap_or_else(|_| {
                            panic!(
                                "Field in `Some` variant of {} should be of type {}",
                                std::any::type_name::<Option<T>>(),
                                std::any::type_name::<T>()
                            )
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

impl<T: FromReflect + TypePath> FromReflect for Option<T> {
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
        if let ReflectRef::Enum(dyn_enum) = reflect.reflect_ref() {
            match dyn_enum.variant_name() {
                "Some" => {
                    let field = T::take_from_reflect(
                        dyn_enum
                            .field_at(0)
                            .unwrap_or_else(|| {
                                panic!(
                                    "Field in `Some` variant of {} should exist",
                                    std::any::type_name::<Option<T>>()
                                )
                            })
                            .clone_value(),
                    )
                    .unwrap_or_else(|_| {
                        panic!(
                            "Field in `Some` variant of {} should be of type {}",
                            std::any::type_name::<Option<T>>(),
                            std::any::type_name::<T>()
                        )
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

impl<T: FromReflect + TypePath> Typed for Option<T> {
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

impl_type_path!(::core::option::Option<T: FromReflect + TypePath>);

impl Reflect for Cow<'static, str> {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
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
        let mut hasher = reflect_hasher();
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

impl TypePath for Cow<'static, str> {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| "std::borrow::Cow::<str>".to_owned())
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| "Cow<str>".to_owned())
    }

    fn type_ident() -> Option<&'static str> {
        Some("Cow")
    }

    fn crate_name() -> Option<&'static str> {
        Some("std")
    }

    fn module_path() -> Option<&'static str> {
        Some("std::borrow")
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

impl<T: PathOnly> PathOnly for [T] where [T]: ToOwned {}

impl<T: TypePath> TypePath for [T]
where
    [T]: ToOwned,
{
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("[{}]", <T>::type_path()))
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("[{}]", <T>::short_type_path()))
    }
}

impl<T: ToOwned> PathOnly for T {}

impl<T: FromReflect + Clone + TypePath> List for Cow<'static, [T]> {
    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        self.as_ref().get(index).map(|x| x as &dyn Reflect)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        self.to_mut().get_mut(index).map(|x| x as &mut dyn Reflect)
    }

    fn len(&self) -> usize {
        self.as_ref().len()
    }

    fn iter(&self) -> crate::ListIter {
        crate::ListIter::new(self)
    }

    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
        // into_owned() is not unnecessary here because it avoids cloning whenever you have a Cow::Owned already
        #[allow(clippy::unnecessary_to_owned)]
        self.into_owned()
            .into_iter()
            .map(|value| value.clone_value())
            .collect()
    }

    fn insert(&mut self, index: usize, element: Box<dyn Reflect>) {
        let value = element.take::<T>().unwrap_or_else(|value| {
            T::from_reflect(&*value).unwrap_or_else(|| {
                panic!(
                    "Attempted to insert invalid value of type {}.",
                    value.type_name()
                )
            })
        });
        self.to_mut().insert(index, value);
    }

    fn remove(&mut self, index: usize) -> Box<dyn Reflect> {
        Box::new(self.to_mut().remove(index))
    }

    fn push(&mut self, value: Box<dyn Reflect>) {
        let value = T::take_from_reflect(value).unwrap_or_else(|value| {
            panic!(
                "Attempted to push invalid value of type {}.",
                value.type_name()
            )
        });
        self.to_mut().push(value);
    }

    fn pop(&mut self) -> Option<Box<dyn Reflect>> {
        self.to_mut()
            .pop()
            .map(|value| Box::new(value) as Box<dyn Reflect>)
    }
}

impl<T: FromReflect + Clone + TypePath> Reflect for Cow<'static, [T]> {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
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
        crate::list_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::list_partial_eq(self, value)
    }

    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
    }
}

impl<T: FromReflect + Clone + TypePath> Typed for Cow<'static, [T]> {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::List(ListInfo::new::<Self, T>()))
    }
}

impl<T: FromReflect + Clone + TypePath> GetTypeRegistration for Cow<'static, [T]> {
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Cow<'static, [T]>>()
    }
}

impl<T: FromReflect + Clone + TypePath> FromReflect for Cow<'static, [T]> {
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
        if let ReflectRef::List(ref_list) = reflect.reflect_ref() {
            let mut temp_vec = Vec::with_capacity(ref_list.len());
            for field in ref_list.iter() {
                temp_vec.push(T::from_reflect(field)?);
            }
            temp_vec.try_into().ok()
        } else {
            None
        }
    }
}

impl Reflect for &'static Path {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
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
        if let Some(&value) = value.downcast_ref::<Self>() {
            *self = value;
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
        Box::new(*self)
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = reflect_hasher();
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

impl Typed for &'static Path {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Value(ValueInfo::new::<Self>()))
    }
}

impl TypePath for &'static Path {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| "&std::path::Path".to_owned())
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| "&Path".to_owned())
    }
}

impl GetTypeRegistration for &'static Path {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration
    }
}

impl FromReflect for &'static Path {
    fn from_reflect(reflect: &dyn crate::Reflect) -> Option<Self> {
        reflect.as_any().downcast_ref::<Self>().copied()
    }
}

impl Reflect for Cow<'static, Path> {
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
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
        let mut hasher = reflect_hasher();
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

    fn debug(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self, f)
    }
}

impl Typed for Cow<'static, Path> {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Value(ValueInfo::new::<Self>()))
    }
}

trait PathOnly: ToOwned {}
impl PathOnly for Path {}
impl_type_path!(::alloc::borrow::Cow<'a: 'static, T: PathOnly + ?Sized>);
impl_type_path!(::std::path::Path);

impl FromReflect for Cow<'static, Path> {
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
        Some(reflect.as_any().downcast_ref::<Self>()?.clone())
    }
}

impl GetTypeRegistration for Cow<'static, Path> {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectDeserialize>(FromType::<Self>::from_type());
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration.insert::<ReflectSerialize>(FromType::<Self>::from_type());
        registration.insert::<ReflectFromReflect>(FromType::<Self>::from_type());
        registration
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
    use std::path::Path;

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
        #[derive(Reflect, PartialEq, Debug)]
        struct Foo(usize);

        let expected = Some(Foo(123));
        let output = <Option<Foo> as FromReflect>::from_reflect(&expected).unwrap();

        assert_eq!(expected, output);
    }

    #[test]
    fn option_should_apply() {
        #[derive(Reflect, PartialEq, Debug)]
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

    #[test]
    fn path_should_from_reflect() {
        let path = Path::new("hello_world.rs");
        let output = <&'static Path as FromReflect>::from_reflect(&path).unwrap();
        assert_eq!(path, output);
    }
}
