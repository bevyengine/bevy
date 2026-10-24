//! Traits and types used to power [map-like] operations via reflection.
//!
//! [map-like]: https://doc.rust-lang.org/book/ch08-03-hash-maps.html
use core::{
    cmp::Ordering,
    fmt::{Debug, Formatter},
};

use crate::{
    dynamic::{DynamicMap, DynamicValue, DynamicValueRef},
    generics::impl_generic_info_methods,
    ty::impl_type_methods,
    Generics, MaybeTyped, PartialReflect, Reflect, ReflectRef, Type, TypeInfo, TypePath,
};
use alloc::{boxed::Box, vec::Vec};

/// A trait used to power [map-like] operations via [reflection].
///
/// Maps contain zero or more entries of a key and its associated value,
/// and correspond to types like [`HashMap`] and [`BTreeMap`].
/// The order of these entries is not guaranteed by this trait.
///
/// # Hashing and equality
///
/// All keys are expected to return a valid hash value from [`PartialReflect::reflect_hash`] and be
/// comparable using [`PartialReflect::reflect_partial_eq`].
/// If using the [`#[derive(Reflect)]`](derive@crate::Reflect) macro, this can be done by adding
/// `#[reflect(Hash, PartialEq)]` to the entire struct or enum.
/// The ordering is expected to be total, that is as if the reflected type implements the [`Eq`] trait.
/// This is true even for manual implementors who do not hash or compare values,
/// as it is still relied on by [`DynamicMap`].
///
/// # Example
///
/// ```
/// use bevy_reflect::{PartialReflect, Reflect, map::Map};
/// use std::collections::HashMap;
///
///
/// let foo: &mut dyn Map = &mut HashMap::<u32, bool>::new();
/// foo.insert_boxed(Box::new(123_u32), Box::new(true));
/// assert_eq!(foo.len(), 1);
///
/// let field: &dyn PartialReflect = foo.get(&123_u32).unwrap();
/// assert_eq!(field.try_downcast_ref::<bool>(), Some(&true));
/// ```
///
/// [`HashMap`]: std::collections::HashMap
/// [`BTreeMap`]: alloc::collections::BTreeMap
/// [map-like]: https://doc.rust-lang.org/book/ch08-03-hash-maps.html
/// [reflection]: crate
// Prevents unexpectedly importing this trait when trying to call, for example, `HashMap::get`
#[rust_analyzer::completions(ignore_flyimport_methods)]
pub trait Map: PartialReflect {
    /// Constructs a key for this map from a dynamic value representation.
    fn key_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn PartialReflect>>;

    /// Constructs a value for this map from a dynamic value representation.
    fn value_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn PartialReflect>>;

    /// Returns a reference to the value associated with the given key.
    ///
    /// If no value is associated with `key`, returns `None`.
    fn get(&self, key: &dyn PartialReflect) -> Option<&dyn PartialReflect>;

    /// Returns a mutable reference to the value associated with the given key.
    ///
    /// If no value is associated with `key`, returns `None`.
    fn get_mut(&mut self, key: &dyn PartialReflect) -> Option<&mut dyn PartialReflect>;

    /// Returns the number of elements in the map.
    fn len(&self) -> usize;

    /// Returns `true` if the list contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the key-value pairs of the map.
    fn iter(&self) -> Box<dyn Iterator<Item = (&dyn PartialReflect, &dyn PartialReflect)> + '_>;

    /// Drain the key-value pairs of this map to get a vector of owned values.
    ///
    /// After calling this function, `self` will be empty.
    fn drain(&mut self) -> Vec<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)>;

    /// Retain only the elements specified by the predicate.
    ///
    /// In other words, remove all pairs `(k, v)` such that `f(&k, &mut v)` returns `false`.
    fn retain(&mut self, f: &mut dyn FnMut(&dyn PartialReflect, &mut dyn PartialReflect) -> bool);

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    /// If the map did have this key present, the value is updated, and the old value is returned.
    fn insert_boxed(
        &mut self,
        key: Box<dyn PartialReflect>,
        value: Box<dyn PartialReflect>,
    ) -> Option<Box<dyn PartialReflect>>;

    /// Removes an entry from the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    /// If the map did have this key present, the removed value is returned.
    fn remove(&mut self, key: &dyn PartialReflect) -> Option<Box<dyn PartialReflect>>;

    /// Will return `None` if [`TypeInfo`] is not available.
    fn get_represented_map_info(&self) -> Option<&'static MapInfo> {
        self.get_represented_type_info()?.as_map().ok()
    }
}

/// A container for compile-time map info.
#[derive(Clone, Debug)]
pub struct MapInfo {
    ty: Type,
    generics: Generics,
    key_info: fn() -> Option<&'static TypeInfo>,
    key_ty: Type,
    value_info: fn() -> Option<&'static TypeInfo>,
    value_ty: Type,
    #[cfg(feature = "reflect_documentation")]
    docs: Option<&'static str>,
}

impl MapInfo {
    /// Create a new [`MapInfo`].
    pub fn new<
        TMap: Map + TypePath,
        TKey: Reflect + MaybeTyped + TypePath,
        TValue: Reflect + MaybeTyped + TypePath,
    >() -> Self {
        Self {
            ty: Type::of::<TMap>(),
            generics: Generics::new(),
            key_info: TKey::maybe_type_info,
            key_ty: Type::of::<TKey>(),
            value_info: TValue::maybe_type_info,
            value_ty: Type::of::<TValue>(),
            #[cfg(feature = "reflect_documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this map.
    #[cfg(feature = "reflect_documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    impl_type_methods!(ty);

    /// The [`TypeInfo`] of the key type.
    ///
    /// Returns `None` if the key type does not contain static type information,
    /// such as for dynamic types.
    pub fn key_info(&self) -> Option<&'static TypeInfo> {
        (self.key_info)()
    }

    /// The [type] of the key type.
    ///
    /// [type]: Type
    pub fn key_ty(&self) -> Type {
        self.key_ty
    }

    /// The [`TypeInfo`] of the value type.
    ///
    /// Returns `None` if the value type does not contain static type information,
    /// such as for dynamic types.
    pub fn value_info(&self) -> Option<&'static TypeInfo> {
        (self.value_info)()
    }

    /// The [type] of the value type.
    ///
    /// [type]: Type
    pub fn value_ty(&self) -> Type {
        self.value_ty
    }

    /// The docstring of this map, if any.
    #[cfg(feature = "reflect_documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
}

pub(crate) trait MapLike {
    fn get(&self, key: DynamicValueRef<'_>) -> Option<DynamicValueRef<'_>>;

    fn len(&self) -> usize;

    fn iter(&self) -> impl Iterator<Item = (DynamicValueRef<'_>, DynamicValueRef<'_>)>;
}

impl MapLike for DynamicMap {
    fn get(&self, key: DynamicValueRef<'_>) -> Option<DynamicValueRef<'_>> {
        self.get(key).map(DynamicValue::as_ref)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn iter(&self) -> impl Iterator<Item = (DynamicValueRef<'_>, DynamicValueRef<'_>)> {
        self.iter().map(|(k, v)| (k.as_ref(), v.as_ref()))
    }
}

impl MapLike for dyn Map {
    fn get(&self, key: DynamicValueRef<'_>) -> Option<DynamicValueRef<'_>> {
        self.get(key.as_concrete()?)
            .map(|value| DynamicValueRef::from(value.try_as_reflect().unwrap()))
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn iter(&self) -> impl Iterator<Item = (DynamicValueRef<'_>, DynamicValueRef<'_>)> {
        self.iter().map(|(k, v)| {
            (
                // TODO: Don't unwrap here.
                DynamicValueRef::from(k.try_as_reflect().unwrap()),
                DynamicValueRef::from(v.try_as_reflect().unwrap()),
            )
        })
    }
}

/// Compares a [`Map`] with a [`PartialReflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a map;
/// - `b` is the same length as `a`;
/// - For each key-value pair in `a`, `b` contains a value for the given key,
///   and [`PartialReflect::reflect_partial_eq`] returns `Some(true)` for the two values.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline(never)]
pub fn map_partial_eq(a: &dyn Map, b: &dyn PartialReflect) -> Option<bool> {
    let ReflectRef::Map(b) = b.reflect_ref() else {
        return Some(false);
    };
    map_like_partial_eq(a, b)
}

/// Same as above, but for any two [`MapLike`] types.
pub(crate) fn map_like_partial_eq<L: MapLike + ?Sized, R: MapLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<bool> {
    if a.len() != b.len() {
        return Some(false);
    }

    for (key, value) in a.iter() {
        if let Some(map_value) = b.get(key) {
            let eq_result = value.dynamic_partial_eq(map_value);
            if let failed @ (Some(false) | None) = eq_result {
                return failed;
            }
        } else {
            return Some(false);
        }
    }

    Some(true)
}

/// Lexicographically compares two [`Map`] values according to their iteration order
/// (suitable for ordered maps like `BTreeMap`).
///
/// For each entry pair `(a_k, a_v)` and `(b_k, b_v)` in the iteration order,
/// compare `a_k` to `b_k` using `reflect_partial_cmp`, returning the first
/// non-equal ordering. If keys are equal, compare values `a_v` and `b_v `similarly. If all
/// compared entries are equal, the shorter map is `Less` and longer is `Greater`.
///
/// Returns [`None`] if the comparison couldn't be performed (kinds mismatch or
/// an element comparison returns `None`).
#[inline]
pub fn map_partial_cmp(a: &dyn Map, b: &dyn PartialReflect) -> Option<Ordering> {
    let ReflectRef::Map(b) = b.reflect_ref() else {
        return None;
    };
    map_like_partial_cmp(a, b)
}

/// Same as above, but for any two [`MapLike`] types.
pub(crate) fn map_like_partial_cmp<L: MapLike + ?Sized, R: MapLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<Ordering> {
    let mut a_iter = a.iter();
    let mut b_iter = b.iter();

    loop {
        match (a_iter.next(), b_iter.next()) {
            (Some((a_k, a_v)), Some((b_k, b_v))) => {
                match a_k.dynamic_partial_cmp(b_k) {
                    None => return None,
                    Some(Ordering::Equal) => {}
                    Some(ord) => return Some(ord),
                }

                match a_v.dynamic_partial_cmp(b_v) {
                    None => return None,
                    Some(Ordering::Equal) => {}
                    Some(ord) => return Some(ord),
                }
            }
            (None, None) => return Some(::core::cmp::Ordering::Equal),
            (None, Some(_)) => return Some(::core::cmp::Ordering::Less),
            (Some(_), None) => return Some(::core::cmp::Ordering::Greater),
        }
    }
}

/// The default debug formatter for [`Map`] types.
///
/// # Example
/// ```
/// # use std::collections::HashMap;
/// use bevy_reflect::Reflect;
///
/// let mut my_map = HashMap::new();
/// my_map.insert(123, String::from("Hello"));
/// println!("{:#?}", &my_map as &dyn Reflect);
///
/// // Output:
///
/// // {
/// //   123: "Hello",
/// // }
/// ```
#[inline]
pub fn map_debug(dyn_map: &dyn Map, f: &mut Formatter<'_>) -> core::fmt::Result {
    let mut debug = f.debug_map();
    for (key, value) in dyn_map.iter() {
        debug.entry(&key as &dyn Debug, &value as &dyn Debug);
    }
    debug.finish()
}
