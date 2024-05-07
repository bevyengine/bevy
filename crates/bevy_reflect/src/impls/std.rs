// Temporary workaround for impl_reflect!(Option/Result false-positive
#![allow(unused_qualifications)]

use crate::{
    self as bevy_reflect, impl_type_path, map_apply, map_partial_eq, map_try_apply,
    prelude::ReflectDefault,
    reflect::impl_full_reflect,
    set_apply, set_partial_eq, set_try_apply,
    utility::{reflect_hasher, GenericTypeInfoCell, GenericTypePathCell, NonGenericTypeInfoCell},
    ApplyError, Array, ArrayInfo, ArrayIter, DynamicMap, DynamicSet, DynamicTypePath, FromReflect,
    FromType, Generics, GetTypeRegistration, List, ListInfo, ListIter, Map, MapInfo, MapIter,
    MaybeTyped, OpaqueInfo, PartialReflect, Reflect, ReflectDeserialize, ReflectFromPtr,
    ReflectFromReflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef, ReflectSerialize, Set,
    SetInfo, TypeInfo, TypeParamInfo, TypePath, TypeRegistration, TypeRegistry, Typed,
};
use alloc::{
    borrow::{Cow, ToOwned},
    boxed::Box,
    collections::VecDeque,
    format,
    vec::Vec,
};
use bevy_reflect_derive::{impl_reflect, impl_reflect_opaque};
use core::{
    any::Any,
    fmt,
    hash::{BuildHasher, Hash, Hasher},
    panic::Location,
};

#[cfg(feature = "std")]
use std::path::Path;

impl_reflect_opaque!(bool(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(char(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(u8(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_opaque!(u16(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_opaque!(u32(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_opaque!(u64(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_opaque!(u128(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(usize(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(i8(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_opaque!(i16(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_opaque!(i32(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_opaque!(i64(Debug, Hash, PartialEq, Serialize, Deserialize, Default));
impl_reflect_opaque!(i128(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(isize(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(f32(Debug, PartialEq, Serialize, Deserialize, Default));
impl_reflect_opaque!(f64(Debug, PartialEq, Serialize, Deserialize, Default));
impl_type_path!(str);
impl_reflect_opaque!(::alloc::string::String(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
#[cfg(feature = "std")]
impl_reflect_opaque!(::std::path::PathBuf(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(::core::any::TypeId(Debug, Hash, PartialEq,));
impl_reflect_opaque!(::alloc::collections::BTreeSet<T: Ord + Eq + Clone + Send + Sync>());
impl_reflect_opaque!(::core::ops::Range<T: Clone + Send + Sync>());
impl_reflect_opaque!(::core::ops::RangeInclusive<T: Clone + Send + Sync>());
impl_reflect_opaque!(::core::ops::RangeFrom<T: Clone + Send + Sync>());
impl_reflect_opaque!(::core::ops::RangeTo<T: Clone + Send + Sync>());
impl_reflect_opaque!(::core::ops::RangeToInclusive<T: Clone + Send + Sync>());
impl_reflect_opaque!(::core::ops::RangeFull());
impl_reflect_opaque!(::core::ops::Bound<T: Clone + Send + Sync>());
impl_reflect_opaque!(::bevy_utils::Duration(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
#[cfg(any(target_arch = "wasm32", feature = "std"))]
impl_reflect_opaque!(::bevy_utils::Instant(Debug, Hash, PartialEq));
impl_reflect_opaque!(::core::num::NonZeroI128(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroU128(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroIsize(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroUsize(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroI64(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroU64(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroU32(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroI32(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroI16(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroU16(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroU8(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::NonZeroI8(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
impl_reflect_opaque!(::core::num::Wrapping<T: Clone + Send + Sync>());
impl_reflect_opaque!(::core::num::Saturating<T: Clone + Send + Sync>());
impl_reflect_opaque!(::alloc::sync::Arc<T: Send + Sync + ?Sized>);

// `Serialize` and `Deserialize` only for platforms supported by serde:
// https://github.com/serde-rs/serde/blob/3ffb86fc70efd3d329519e2dddfa306cc04f167c/serde/src/de/impls.rs#L1732
#[cfg(all(any(unix, windows), feature = "std"))]
impl_reflect_opaque!(::std::ffi::OsString(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize
));
#[cfg(all(not(any(unix, windows)), feature = "std"))]
impl_reflect_opaque!(::std::ffi::OsString(Debug, Hash, PartialEq));
impl_reflect_opaque!(::alloc::collections::BinaryHeap<T: Clone>);

macro_rules! impl_reflect_for_atomic {
    ($ty:ty, $ordering:expr) => {
        impl_type_path!($ty);

        const _: () = {
            #[cfg(feature = "functions")]
            crate::func::macros::impl_function_traits!($ty);

            #[allow(unused_mut)]
            impl GetTypeRegistration for $ty
            where
                $ty: Any + Send + Sync,
            {
                fn get_type_registration() -> TypeRegistration {
                    let mut registration = TypeRegistration::of::<Self>();
                    registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
                    registration.insert::<ReflectFromReflect>(FromType::<Self>::from_type());
                    registration.insert::<ReflectDefault>(FromType::<Self>::from_type());

                    // Serde only supports atomic types when the "std" feature is enabled
                    #[cfg(feature = "std")]
                    {
                        registration.insert::<ReflectSerialize>(FromType::<Self>::from_type());
                        registration.insert::<ReflectDeserialize>(FromType::<Self>::from_type());
                    }

                    registration
                }
            }

            impl Typed for $ty
            where
                $ty: Any + Send + Sync,
            {
                fn type_info() -> &'static TypeInfo {
                    static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
                    CELL.get_or_set(|| {
                        let info = OpaqueInfo::new::<Self>();
                        TypeInfo::Opaque(info)
                    })
                }
            }

            impl PartialReflect for $ty
            where
                $ty: Any + Send + Sync,
            {
                #[inline]
                fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
                    Some(<Self as Typed>::type_info())
                }
                #[inline]
                fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
                    self
                }
                #[inline]
                fn as_partial_reflect(&self) -> &dyn PartialReflect {
                    self
                }
                #[inline]
                fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
                    self
                }
                #[inline]
                fn try_into_reflect(
                    self: Box<Self>,
                ) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
                    Ok(self)
                }
                #[inline]
                fn try_as_reflect(&self) -> Option<&dyn Reflect> {
                    Some(self)
                }
                #[inline]
                fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
                    Some(self)
                }
                #[inline]
                fn clone_value(&self) -> Box<dyn PartialReflect> {
                    Box::new(<$ty>::new(self.load($ordering)))
                }
                #[inline]
                fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
                    if let Some(value) = value.try_downcast_ref::<Self>() {
                        *self = <$ty>::new(value.load($ordering));
                    } else {
                        return Err(ApplyError::MismatchedTypes {
                            from_type: Into::into(DynamicTypePath::reflect_type_path(value)),
                            to_type: Into::into(<Self as TypePath>::type_path()),
                        });
                    }
                    Ok(())
                }
                #[inline]
                fn reflect_kind(&self) -> ReflectKind {
                    ReflectKind::Opaque
                }
                #[inline]
                fn reflect_ref(&self) -> ReflectRef {
                    ReflectRef::Opaque(self)
                }
                #[inline]
                fn reflect_mut(&mut self) -> ReflectMut {
                    ReflectMut::Opaque(self)
                }
                #[inline]
                fn reflect_owned(self: Box<Self>) -> ReflectOwned {
                    ReflectOwned::Opaque(self)
                }
                fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Debug::fmt(self, f)
                }
            }

            impl FromReflect for $ty
            where
                $ty: Any + Send + Sync,
            {
                fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
                    Some(<$ty>::new(
                        reflect.try_downcast_ref::<$ty>()?.load($ordering),
                    ))
                }
            }
        };

        impl_full_reflect!(for $ty where $ty: Any + Send + Sync);
    };
}

impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicIsize,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicUsize,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicI64,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicU64,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicI32,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicU32,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicI16,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicU16,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicI8,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicU8,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicBool,
    ::core::sync::atomic::Ordering::SeqCst
);

macro_rules! impl_reflect_for_veclike {
    ($ty:ty, $insert:expr, $remove:expr, $push:expr, $pop:expr, $sub:ty) => {
        impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration> List for $ty {
            #[inline]
            fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
                <$sub>::get(self, index).map(|value| value as &dyn PartialReflect)
            }

            #[inline]
            fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
                <$sub>::get_mut(self, index).map(|value| value as &mut dyn PartialReflect)
            }

            fn insert(&mut self, index: usize, value: Box<dyn PartialReflect>) {
                let value = value.try_take::<T>().unwrap_or_else(|value| {
                    T::from_reflect(&*value).unwrap_or_else(|| {
                        panic!(
                            "Attempted to insert invalid value of type {}.",
                            value.reflect_type_path()
                        )
                    })
                });
                $insert(self, index, value);
            }

            fn remove(&mut self, index: usize) -> Box<dyn PartialReflect> {
                Box::new($remove(self, index))
            }

            fn push(&mut self, value: Box<dyn PartialReflect>) {
                let value = T::take_from_reflect(value).unwrap_or_else(|value| {
                    panic!(
                        "Attempted to push invalid value of type {}.",
                        value.reflect_type_path()
                    )
                });
                $push(self, value);
            }

            fn pop(&mut self) -> Option<Box<dyn PartialReflect>> {
                $pop(self).map(|value| Box::new(value) as Box<dyn PartialReflect>)
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
            fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
                self.drain(..)
                    .map(|value| Box::new(value) as Box<dyn PartialReflect>)
                    .collect()
            }
        }

        impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration> PartialReflect for $ty {
            #[inline]
            fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
                Some(<Self as Typed>::type_info())
            }

            fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
                self
            }

            #[inline]
            fn as_partial_reflect(&self) -> &dyn PartialReflect {
                self
            }

            #[inline]
            fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
                self
            }

            fn try_into_reflect(
                self: Box<Self>,
            ) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
                Ok(self)
            }

            fn try_as_reflect(&self) -> Option<&dyn Reflect> {
                Some(self)
            }

            fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
                Some(self)
            }

            fn reflect_kind(&self) -> ReflectKind {
                ReflectKind::List
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

            fn clone_value(&self) -> Box<dyn PartialReflect> {
                Box::new(self.clone_dynamic())
            }

            fn reflect_hash(&self) -> Option<u64> {
                crate::list_hash(self)
            }

            fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
                crate::list_partial_eq(self, value)
            }

            fn apply(&mut self, value: &dyn PartialReflect) {
                crate::list_apply(self, value);
            }

            fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
                crate::list_try_apply(self, value)
            }
        }

        impl_full_reflect!(<T> for $ty where T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration);

        impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration> Typed for $ty {
            fn type_info() -> &'static TypeInfo {
                static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    TypeInfo::List(
                        ListInfo::new::<Self, T>().with_generics(Generics::from_iter([
                            TypeParamInfo::new::<T>("T")
                        ]))
                    )
                })
            }
        }

        impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration> GetTypeRegistration
            for $ty
        {
            fn get_type_registration() -> TypeRegistration {
                let mut registration = TypeRegistration::of::<$ty>();
                registration.insert::<ReflectFromPtr>(FromType::<$ty>::from_type());
                registration
            }

            fn register_type_dependencies(registry: &mut TypeRegistry) {
                registry.register::<T>();
            }
        }

        impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration> FromReflect for $ty {
            fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
                let ref_list = reflect.reflect_ref().as_list().ok()?;

                let mut new_list = Self::with_capacity(ref_list.len());

                for field in ref_list.iter() {
                    $push(&mut new_list, T::from_reflect(field)?);
                }

                Some(new_list)
            }
        }
    };
}

impl_reflect_for_veclike!(Vec<T>, Vec::insert, Vec::remove, Vec::push, Vec::pop, [T]);
impl_type_path!(::alloc::vec::Vec<T>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(Vec<T>; <T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration>);

impl_reflect_for_veclike!(
    VecDeque<T>,
    VecDeque::insert,
    VecDeque::remove,
    VecDeque::push_back,
    VecDeque::pop_back,
    VecDeque::<T>
);
impl_type_path!(::alloc::collections::VecDeque<T>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(VecDeque<T>; <T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration>);

macro_rules! impl_reflect_for_hashmap {
    ($ty:path) => {
        impl<K, V, S> Map for $ty
        where
            K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
            V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
            S: TypePath + BuildHasher + Send + Sync,
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

            fn get_at(&self, index: usize) -> Option<(&dyn PartialReflect, &dyn PartialReflect)> {
                self.iter()
                    .nth(index)
                    .map(|(key, value)| (key as &dyn PartialReflect, value as &dyn PartialReflect))
            }

            fn get_at_mut(
                &mut self,
                index: usize,
            ) -> Option<(&dyn PartialReflect, &mut dyn PartialReflect)> {
                self.iter_mut().nth(index).map(|(key, value)| {
                    (key as &dyn PartialReflect, value as &mut dyn PartialReflect)
                })
            }

            fn len(&self) -> usize {
                Self::len(self)
            }

            fn iter(&self) -> MapIter {
                MapIter::new(self)
            }

            fn drain(&mut self) -> Vec<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
                self.drain()
                    .map(|(key, value)| {
                        (
                            Box::new(key) as Box<dyn PartialReflect>,
                            Box::new(value) as Box<dyn PartialReflect>,
                        )
                    })
                    .collect()
            }

            fn clone_dynamic(&self) -> DynamicMap {
                let mut dynamic_map = DynamicMap::default();
                dynamic_map.set_represented_type(self.get_represented_type_info());
                for (k, v) in self {
                    let key = K::from_reflect(k).unwrap_or_else(|| {
                        panic!(
                            "Attempted to clone invalid key of type {}.",
                            k.reflect_type_path()
                        )
                    });
                    dynamic_map.insert_boxed(Box::new(key), v.clone_value());
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
                    .and_then(|key| self.remove(key))
                    .map(|value| Box::new(value) as Box<dyn PartialReflect>)
            }
        }

        impl<K, V, S> PartialReflect for $ty
        where
            K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
            V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
            S: TypePath + BuildHasher + Send + Sync,
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

            fn try_into_reflect(
                self: Box<Self>,
            ) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
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

            fn reflect_ref(&self) -> ReflectRef {
                ReflectRef::Map(self)
            }

            fn reflect_mut(&mut self) -> ReflectMut {
                ReflectMut::Map(self)
            }

            fn reflect_owned(self: Box<Self>) -> ReflectOwned {
                ReflectOwned::Map(self)
            }

            fn clone_value(&self) -> Box<dyn PartialReflect> {
                Box::new(self.clone_dynamic())
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
            <K, V, S> for $ty
            where
                K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
                V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
                S: TypePath + BuildHasher + Send + Sync,
        );

        impl<K, V, S> Typed for $ty
        where
            K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
            V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
            S: TypePath + BuildHasher + Send + Sync,
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

        impl<K, V, S> GetTypeRegistration for $ty
        where
            K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
            V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
            S: TypePath + BuildHasher + Send + Sync,
        {
            fn get_type_registration() -> TypeRegistration {
                let mut registration = TypeRegistration::of::<Self>();
                registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
                registration
            }

            fn register_type_dependencies(registry: &mut TypeRegistry) {
                registry.register::<K>();
                registry.register::<V>();
            }
        }

        impl<K, V, S> FromReflect for $ty
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
    };
}

#[cfg(feature = "std")]
impl_reflect_for_hashmap!(::std::collections::HashMap<K, V, S>);
impl_type_path!(::core::hash::BuildHasherDefault<H>);
#[cfg(feature = "std")]
impl_type_path!(::std::collections::hash_map::RandomState);
#[cfg(feature = "std")]
impl_type_path!(::std::collections::HashMap<K, V, S>);
#[cfg(all(feature = "functions", feature = "std"))]
crate::func::macros::impl_function_traits!(::std::collections::HashMap<K, V, S>;
    <
        K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
        V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

impl_reflect_for_hashmap!(bevy_utils::hashbrown::HashMap<K, V, S>);
impl_type_path!(::bevy_utils::hashbrown::HashMap<K, V, S>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::bevy_utils::hashbrown::HashMap<K, V, S>;
    <
        K: FromReflect + MaybeTyped + TypePath + GetTypeRegistration + Eq + Hash,
        V: FromReflect + MaybeTyped + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

macro_rules! impl_reflect_for_hashset {
    ($ty:path) => {
        impl<V, S> Set for $ty
        where
            V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
            S: TypePath + BuildHasher + Send + Sync,
        {
            fn get(&self, value: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
                value
                    .try_downcast_ref::<V>()
                    .and_then(|value| Self::get(self, value))
                    .map(|value| value as &dyn PartialReflect)
            }

            fn len(&self) -> usize {
                Self::len(self)
            }

            fn iter(&self) -> Box<dyn Iterator<Item = &dyn PartialReflect> + '_> {
                let iter = self.iter().map(|v| v as &dyn PartialReflect);
                Box::new(iter)
            }

            fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
                self.drain()
                    .map(|value| Box::new(value) as Box<dyn PartialReflect>)
                    .collect()
            }

            fn clone_dynamic(&self) -> DynamicSet {
                let mut dynamic_set = DynamicSet::default();
                dynamic_set.set_represented_type(self.get_represented_type_info());
                for v in self {
                    dynamic_set.insert_boxed(v.clone_value());
                }
                dynamic_set
            }

            fn insert_boxed(&mut self, value: Box<dyn PartialReflect>) -> bool {
                let value = V::take_from_reflect(value).unwrap_or_else(|value| {
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
                    .try_downcast_ref::<V>()
                    .or_else(|| {
                        from_reflect = V::from_reflect(value);
                        from_reflect.as_ref()
                    })
                    .map_or(false, |value| self.remove(value))
            }

            fn contains(&self, value: &dyn PartialReflect) -> bool {
                let mut from_reflect = None;
                value
                    .try_downcast_ref::<V>()
                    .or_else(|| {
                        from_reflect = V::from_reflect(value);
                        from_reflect.as_ref()
                    })
                    .map_or(false, |value| self.contains(value))
            }
        }

        impl<V, S> PartialReflect for $ty
        where
            V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
            S: TypePath + BuildHasher + Send + Sync,
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
            fn try_into_reflect(
                self: Box<Self>,
            ) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
                Ok(self)
            }

            fn try_as_reflect(&self) -> Option<&dyn Reflect> {
                Some(self)
            }

            fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
                Some(self)
            }

            fn apply(&mut self, value: &dyn PartialReflect) {
                set_apply(self, value);
            }

            fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
                set_try_apply(self, value)
            }

            fn reflect_kind(&self) -> ReflectKind {
                ReflectKind::Set
            }

            fn reflect_ref(&self) -> ReflectRef {
                ReflectRef::Set(self)
            }

            fn reflect_mut(&mut self) -> ReflectMut {
                ReflectMut::Set(self)
            }

            fn reflect_owned(self: Box<Self>) -> ReflectOwned {
                ReflectOwned::Set(self)
            }

            fn clone_value(&self) -> Box<dyn PartialReflect> {
                Box::new(self.clone_dynamic())
            }

            fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
                set_partial_eq(self, value)
            }
        }

        impl<V, S> Typed for $ty
        where
            V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
            S: TypePath + BuildHasher + Send + Sync,
        {
            fn type_info() -> &'static TypeInfo {
                static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    TypeInfo::Set(
                        SetInfo::new::<Self, V>().with_generics(Generics::from_iter([
                            TypeParamInfo::new::<V>("V")
                        ]))
                    )
                })
            }
        }

        impl<V, S> GetTypeRegistration for $ty
        where
            V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
            S: TypePath + BuildHasher + Send + Sync,
        {
            fn get_type_registration() -> TypeRegistration {
                let mut registration = TypeRegistration::of::<Self>();
                registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
                registration
            }

            fn register_type_dependencies(registry: &mut TypeRegistry) {
                registry.register::<V>();
            }
        }

        impl_full_reflect!(
            <V, S> for $ty
            where
                V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
                S: TypePath + BuildHasher + Send + Sync,
        );

        impl<V, S> FromReflect for $ty
        where
            V: FromReflect + TypePath + GetTypeRegistration + Eq + Hash,
            S: TypePath + BuildHasher + Default + Send + Sync,
        {
            fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
                let ref_set = reflect.reflect_ref().as_set().ok()?;

                let mut new_set = Self::with_capacity_and_hasher(ref_set.len(), S::default());

                for value in ref_set.iter() {
                    let new_value = V::from_reflect(value)?;
                    new_set.insert(new_value);
                }

                Some(new_set)
            }
        }
    };
}

impl_type_path!(::bevy_utils::NoOpHash);
impl_type_path!(::bevy_utils::FixedHasher);

#[cfg(feature = "std")]
impl_reflect_for_hashset!(::std::collections::HashSet<V,S>);
#[cfg(feature = "std")]
impl_type_path!(::std::collections::HashSet<V, S>);
#[cfg(all(feature = "functions", feature = "std"))]
crate::func::macros::impl_function_traits!(::std::collections::HashSet<V, S>;
    <
        V: Hash + Eq + FromReflect + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

impl_reflect_for_hashset!(::bevy_utils::hashbrown::HashSet<V,S>);
impl_type_path!(::bevy_utils::hashbrown::HashSet<V, S>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::bevy_utils::hashbrown::HashSet<V, S>;
    <
        V: Hash + Eq + FromReflect + TypePath + GetTypeRegistration,
        S: TypePath + BuildHasher + Default + Send + Sync
    >
);

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

    fn get_at(&self, index: usize) -> Option<(&dyn PartialReflect, &dyn PartialReflect)> {
        self.iter()
            .nth(index)
            .map(|(key, value)| (key as &dyn PartialReflect, value as &dyn PartialReflect))
    }

    fn get_at_mut(
        &mut self,
        index: usize,
    ) -> Option<(&dyn PartialReflect, &mut dyn PartialReflect)> {
        self.iter_mut()
            .nth(index)
            .map(|(key, value)| (key as &dyn PartialReflect, value as &mut dyn PartialReflect))
    }

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn iter(&self) -> MapIter {
        MapIter::new(self)
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

    fn clone_dynamic(&self) -> DynamicMap {
        let mut dynamic_map = DynamicMap::default();
        dynamic_map.set_represented_type(self.get_represented_type_info());
        for (k, v) in self {
            let key = K::from_reflect(k).unwrap_or_else(|| {
                panic!(
                    "Attempted to clone invalid key of type {}.",
                    k.reflect_type_path()
                )
            });
            dynamic_map.insert_boxed(Box::new(key), v.clone_value());
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

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Map(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Map(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Map(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(self.clone_dynamic())
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

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> Array for [T; N] {
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        <[T]>::get(self, index).map(|value| value as &dyn PartialReflect)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        <[T]>::get_mut(self, index).map(|value| value as &mut dyn PartialReflect)
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
    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>> {
        self.into_iter()
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
            .collect()
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> PartialReflect
    for [T; N]
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

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    #[inline]
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::Array
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
    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(self.clone_dynamic())
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        crate::array_hash(self)
    }

    #[inline]
    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        crate::array_partial_eq(self, value)
    }

    fn apply(&mut self, value: &dyn PartialReflect) {
        crate::array_apply(self, value);
    }

    #[inline]
    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        crate::array_try_apply(self, value)
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> Reflect for [T; N] {
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
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }
}

impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> FromReflect
    for [T; N]
{
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        let ref_array = reflect.reflect_ref().as_array().ok()?;

        let mut temp_vec = Vec::with_capacity(ref_array.len());

        for field in ref_array.iter() {
            temp_vec.push(T::from_reflect(field)?);
        }

        temp_vec.try_into().ok()
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> Typed for [T; N] {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::Array(ArrayInfo::new::<Self, T>(N)))
    }
}

impl<T: TypePath, const N: usize> TypePath for [T; N] {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("[{t}; {N}]", t = T::type_path()))
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("[{t}; {N}]", t = T::short_type_path()))
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> GetTypeRegistration
    for [T; N]
{
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<[T; N]>()
    }

    fn register_type_dependencies(registry: &mut TypeRegistry) {
        registry.register::<T>();
    }
}

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!([T; N]; <T: Reflect + MaybeTyped + TypePath + GetTypeRegistration> [const N: usize]);

impl_reflect! {
    #[type_path = "core::option"]
    enum Option<T> {
        None,
        Some(T),
    }
}

impl_reflect! {
    #[type_path = "core::result"]
    enum Result<T, E> {
        Ok(T),
        Err(E),
    }
}

impl<T: TypePath + ?Sized> TypePath for &'static T {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("&{}", T::type_path()))
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("&{}", T::short_type_path()))
    }
}

impl<T: TypePath + ?Sized> TypePath for &'static mut T {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("&mut {}", T::type_path()))
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("&mut {}", T::short_type_path()))
    }
}

impl PartialReflect for Cow<'static, str> {
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
        ReflectKind::Opaque
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Opaque(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Opaque(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Opaque(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(self.clone())
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = reflect_hasher();
        Hash::hash(&Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            Some(PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
        fmt::Debug::fmt(self, f)
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            self.clone_from(value);
        } else {
            return Err(ApplyError::MismatchedTypes {
                from_type: value.reflect_type_path().into(),
                // If we invoke the reflect_type_path on self directly the borrow checker complains that the lifetime of self must outlive 'static
                to_type: Self::type_path().into(),
            });
        }
        Ok(())
    }
}

impl_full_reflect!(for Cow<'static, str>);

impl Typed for Cow<'static, str> {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
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
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        Some(reflect.try_downcast_ref::<Cow<'static, str>>()?.clone())
    }
}

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(Cow<'static, str>);

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

impl<T: FromReflect + MaybeTyped + Clone + TypePath + GetTypeRegistration> List
    for Cow<'static, [T]>
{
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        self.as_ref().get(index).map(|x| x as &dyn PartialReflect)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        self.to_mut()
            .get_mut(index)
            .map(|x| x as &mut dyn PartialReflect)
    }

    fn insert(&mut self, index: usize, element: Box<dyn PartialReflect>) {
        let value = T::take_from_reflect(element).unwrap_or_else(|value| {
            panic!(
                "Attempted to insert invalid value of type {}.",
                value.reflect_type_path()
            );
        });
        self.to_mut().insert(index, value);
    }

    fn remove(&mut self, index: usize) -> Box<dyn PartialReflect> {
        Box::new(self.to_mut().remove(index))
    }

    fn push(&mut self, value: Box<dyn PartialReflect>) {
        let value = T::take_from_reflect(value).unwrap_or_else(|value| {
            panic!(
                "Attempted to push invalid value of type {}.",
                value.reflect_type_path()
            )
        });
        self.to_mut().push(value);
    }

    fn pop(&mut self) -> Option<Box<dyn PartialReflect>> {
        self.to_mut()
            .pop()
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
    }

    fn len(&self) -> usize {
        self.as_ref().len()
    }

    fn iter(&self) -> ListIter {
        ListIter::new(self)
    }

    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
        self.to_mut()
            .drain(..)
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
            .collect()
    }
}

impl<T: FromReflect + MaybeTyped + Clone + TypePath + GetTypeRegistration> PartialReflect
    for Cow<'static, [T]>
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
        ReflectKind::List
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

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(List::clone_dynamic(self))
    }

    fn reflect_hash(&self) -> Option<u64> {
        crate::list_hash(self)
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        crate::list_partial_eq(self, value)
    }

    fn apply(&mut self, value: &dyn PartialReflect) {
        crate::list_apply(self, value);
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        crate::list_try_apply(self, value)
    }
}

impl_full_reflect!(
    <T> for Cow<'static, [T]>
    where
        T: FromReflect + Clone + MaybeTyped + TypePath + GetTypeRegistration,
);

impl<T: FromReflect + MaybeTyped + Clone + TypePath + GetTypeRegistration> Typed
    for Cow<'static, [T]>
{
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::List(ListInfo::new::<Self, T>()))
    }
}

impl<T: FromReflect + MaybeTyped + Clone + TypePath + GetTypeRegistration> GetTypeRegistration
    for Cow<'static, [T]>
{
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<Cow<'static, [T]>>()
    }

    fn register_type_dependencies(registry: &mut TypeRegistry) {
        registry.register::<T>();
    }
}

impl<T: FromReflect + MaybeTyped + Clone + TypePath + GetTypeRegistration> FromReflect
    for Cow<'static, [T]>
{
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        let ref_list = reflect.reflect_ref().as_list().ok()?;

        let mut temp_vec = Vec::with_capacity(ref_list.len());

        for field in ref_list.iter() {
            temp_vec.push(T::from_reflect(field)?);
        }

        Some(temp_vec.into())
    }
}

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(Cow<'static, [T]>; <T: FromReflect + MaybeTyped + Clone + TypePath + GetTypeRegistration>);

impl PartialReflect for &'static str {
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

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Opaque(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Opaque(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Opaque(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(*self)
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = reflect_hasher();
        Hash::hash(&Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            Some(PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            self.clone_from(value);
        } else {
            return Err(ApplyError::MismatchedTypes {
                from_type: value.reflect_type_path().into(),
                to_type: Self::type_path().into(),
            });
        }
        Ok(())
    }
}

impl Reflect for &'static str {
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

impl Typed for &'static str {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

impl GetTypeRegistration for &'static str {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration.insert::<ReflectFromReflect>(FromType::<Self>::from_type());
        registration
    }
}

impl FromReflect for &'static str {
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        reflect.try_downcast_ref::<Self>().copied()
    }
}

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(&'static str);

#[cfg(feature = "std")]
impl PartialReflect for &'static Path {
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
        ReflectKind::Opaque
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Opaque(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Opaque(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Opaque(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(*self)
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = reflect_hasher();
        Hash::hash(&Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            Some(PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            self.clone_from(value);
            Ok(())
        } else {
            Err(ApplyError::MismatchedTypes {
                from_type: value.reflect_type_path().into(),
                to_type: <Self as DynamicTypePath>::reflect_type_path(self).into(),
            })
        }
    }
}

#[cfg(feature = "std")]
impl Reflect for &'static Path {
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

#[cfg(feature = "std")]
impl Typed for &'static Path {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

#[cfg(feature = "std")]
impl GetTypeRegistration for &'static Path {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration
    }
}

#[cfg(feature = "std")]
impl FromReflect for &'static Path {
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        reflect.try_downcast_ref::<Self>().copied()
    }
}

#[cfg(all(feature = "functions", feature = "std"))]
crate::func::macros::impl_function_traits!(&'static Path);

#[cfg(feature = "std")]
impl PartialReflect for Cow<'static, Path> {
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
        ReflectKind::Opaque
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Opaque(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Opaque(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Opaque(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(self.clone())
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = reflect_hasher();
        Hash::hash(&Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            Some(PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            self.clone_from(value);
            Ok(())
        } else {
            Err(ApplyError::MismatchedTypes {
                from_type: value.reflect_type_path().into(),
                to_type: <Self as DynamicTypePath>::reflect_type_path(self).into(),
            })
        }
    }
}

#[cfg(feature = "std")]
impl Reflect for Cow<'static, Path> {
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

#[cfg(feature = "std")]
impl Typed for Cow<'static, Path> {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

#[cfg(feature = "std")]
impl_type_path!(::std::path::Path);
impl_type_path!(::alloc::borrow::Cow<'a: 'static, T: ToOwned + ?Sized>);

#[cfg(feature = "std")]
impl FromReflect for Cow<'static, Path> {
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        Some(reflect.try_downcast_ref::<Self>()?.clone())
    }
}

#[cfg(feature = "std")]
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

#[cfg(all(feature = "functions", feature = "std"))]
crate::func::macros::impl_function_traits!(Cow<'static, Path>);

impl TypePath for &'static Location<'static> {
    fn type_path() -> &'static str {
        "core::panic::Location"
    }

    fn short_type_path() -> &'static str {
        "Location"
    }
}

impl PartialReflect for &'static Location<'static> {
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
        ReflectKind::Opaque
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Opaque(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Opaque(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Opaque(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(*self)
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = reflect_hasher();
        Hash::hash(&Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            Some(PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            self.clone_from(value);
            Ok(())
        } else {
            Err(ApplyError::MismatchedTypes {
                from_type: value.reflect_type_path().into(),
                to_type: <Self as DynamicTypePath>::reflect_type_path(self).into(),
            })
        }
    }
}

impl Reflect for &'static Location<'static> {
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

impl Typed for &'static Location<'static> {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

impl GetTypeRegistration for &'static Location<'static> {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration
    }
}

impl FromReflect for &'static Location<'static> {
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        reflect.try_downcast_ref::<Self>().copied()
    }
}

#[cfg(all(feature = "functions", feature = "std"))]
crate::func::macros::impl_function_traits!(&'static Location<'static>);

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_reflect, Enum, FromReflect, PartialReflect, Reflect, ReflectSerialize,
        TypeInfo, TypeRegistry, Typed, VariantInfo, VariantType,
    };
    use alloc::collections::BTreeMap;
    use bevy_utils::{Duration, HashMap, Instant};
    use core::f32::consts::{PI, TAU};
    use static_assertions::assert_impl_all;
    use std::path::Path;

    #[test]
    fn can_serialize_duration() {
        let mut type_registry = TypeRegistry::default();
        type_registry.register::<Duration>();

        let reflect_serialize = type_registry
            .get_type_data::<ReflectSerialize>(core::any::TypeId::of::<Duration>())
            .unwrap();
        let _serializable = reflect_serialize.get_serializable(&Duration::ZERO);
    }

    #[test]
    fn should_partial_eq_char() {
        let a: &dyn PartialReflect = &'x';
        let b: &dyn PartialReflect = &'x';
        let c: &dyn PartialReflect = &'o';
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_i32() {
        let a: &dyn PartialReflect = &123_i32;
        let b: &dyn PartialReflect = &123_i32;
        let c: &dyn PartialReflect = &321_i32;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_f32() {
        let a: &dyn PartialReflect = &PI;
        let b: &dyn PartialReflect = &PI;
        let c: &dyn PartialReflect = &TAU;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_string() {
        let a: &dyn PartialReflect = &String::from("Hello");
        let b: &dyn PartialReflect = &String::from("Hello");
        let c: &dyn PartialReflect = &String::from("World");
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_vec() {
        let a: &dyn PartialReflect = &vec![1, 2, 3];
        let b: &dyn PartialReflect = &vec![1, 2, 3];
        let c: &dyn PartialReflect = &vec![3, 2, 1];
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_hash_map() {
        let mut a = <HashMap<_, _>>::default();
        a.insert(0usize, 1.23_f64);
        let b = a.clone();
        let mut c = <HashMap<_, _>>::default();
        c.insert(0usize, 3.21_f64);

        let a: &dyn PartialReflect = &a;
        let b: &dyn PartialReflect = &b;
        let c: &dyn PartialReflect = &c;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_btree_map() {
        let mut a = BTreeMap::new();
        a.insert(0usize, 1.23_f64);
        let b = a.clone();
        let mut c = BTreeMap::new();
        c.insert(0usize, 3.21_f64);

        let a: &dyn Reflect = &a;
        let b: &dyn Reflect = &b;
        let c: &dyn Reflect = &c;
        assert!(a
            .reflect_partial_eq(b.as_partial_reflect())
            .unwrap_or_default());
        assert!(!a
            .reflect_partial_eq(c.as_partial_reflect())
            .unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_option() {
        let a: &dyn PartialReflect = &Some(123);
        let b: &dyn PartialReflect = &Some(123);
        assert_eq!(Some(true), a.reflect_partial_eq(b));
    }

    #[test]
    fn option_should_impl_enum() {
        assert_impl_all!(Option<()>: Enum);

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
                .and_then(|field| field.try_downcast_mut::<usize>())
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
        let mut value = None::<Foo>;
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "None apply onto None");

        // === Some on None === //
        let patch = Some(Foo(123));
        let mut value = None::<Foo>;
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "Some apply onto None");

        // === None on Some === //
        let patch = None::<Foo>;
        let mut value = Some(Foo(321));
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "None apply onto Some");

        // === Some on Some === //
        let patch = Some(Foo(123));
        let mut value = Some(Foo(321));
        PartialReflect::apply(&mut value, &patch);

        assert_eq!(patch, value, "Some apply onto Some");
    }

    #[test]
    fn option_should_impl_typed() {
        assert_impl_all!(Option<()>: Typed);

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
        let a: &dyn PartialReflect = &core::num::NonZero::<usize>::new(42).unwrap();
        let b: &dyn PartialReflect = &core::num::NonZero::<usize>::new(42).unwrap();
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        let forty_two: core::num::NonZero<usize> = FromReflect::from_reflect(a).unwrap();
        assert_eq!(forty_two, core::num::NonZero::<usize>::new(42).unwrap());
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

    #[test]
    fn type_id_should_from_reflect() {
        let type_id = core::any::TypeId::of::<usize>();
        let output = <core::any::TypeId as FromReflect>::from_reflect(&type_id).unwrap();
        assert_eq!(type_id, output);
    }

    #[test]
    fn static_str_should_from_reflect() {
        let expected = "Hello, World!";
        let output = <&'static str as FromReflect>::from_reflect(&expected).unwrap();
        assert_eq!(expected, output);
    }
}
