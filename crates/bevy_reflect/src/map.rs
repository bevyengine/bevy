use core::fmt::{Debug, Formatter};

use bevy_platform::collections::HashTable;
use bevy_reflect_derive::impl_type_path;

use crate::{
    generics::impl_generic_info_methods, type_info::impl_type_methods, ApplyError, Generics,
    MaybeTyped, PartialReflect, Reflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef, Type,
    TypeInfo, TypePath,
};
use alloc::{boxed::Box, format, vec::Vec};

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
/// use bevy_reflect::{PartialReflect, Reflect, Map};
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
pub trait Map: PartialReflect {
    /// Returns a reference to the value associated with the given key.
    ///
    /// If no value is associated with `key`, returns `None`.
    fn get(&self, key: &dyn PartialReflect) -> Option<&dyn PartialReflect>;

    /// Returns a mutable reference to the value associated with the given key.
    ///
    /// If no value is associated with `key`, returns `None`.
    fn get_mut(&mut self, key: &dyn PartialReflect) -> Option<&mut dyn PartialReflect>;

    /// Returns the key-value pair at `index` by reference, or `None` if out of bounds.
    fn get_at(&self, index: usize) -> Option<(&dyn PartialReflect, &dyn PartialReflect)>;

    /// Returns the key-value pair at `index` by reference where the value is a mutable reference, or `None` if out of bounds.
    fn get_at_mut(
        &mut self,
        index: usize,
    ) -> Option<(&dyn PartialReflect, &mut dyn PartialReflect)>;

    /// Returns the number of elements in the map.
    fn len(&self) -> usize;

    /// Returns `true` if the list contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the key-value pairs of the map.
    fn iter(&self) -> MapIter;

    /// Drain the key-value pairs of this map to get a vector of owned values.
    ///
    /// After calling this function, `self` will be empty.
    fn drain(&mut self) -> Vec<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)>;

    /// Clones the map, producing a [`DynamicMap`].
    #[deprecated(since = "0.16.0", note = "use `to_dynamic_map` instead")]
    fn clone_dynamic(&self) -> DynamicMap {
        self.to_dynamic_map()
    }

    /// Creates a new [`DynamicMap`] from this map.
    fn to_dynamic_map(&self) -> DynamicMap {
        let mut map = DynamicMap::default();
        map.set_represented_type(self.get_represented_type_info());
        for (key, value) in self.iter() {
            map.insert_boxed(key.to_dynamic(), value.to_dynamic());
        }
        map
    }

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
    #[cfg(feature = "documentation")]
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
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this map.
    #[cfg(feature = "documentation")]
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
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
}

#[macro_export]
macro_rules! hash_error {
    ( $key:expr ) => {{
        let type_path = (*$key).reflect_type_path();
        if !$key.is_dynamic() {
            format!(
                "the given key of type `{}` does not support hashing",
                type_path
            )
        } else {
            match (*$key).get_represented_type_info() {
                // Handle dynamic types that do not represent a type (i.e a plain `DynamicStruct`):
                None => format!("the dynamic type `{}` does not support hashing", type_path),
                // Handle dynamic types that do represent a type (i.e. a `DynamicStruct` proxying `Foo`):
                Some(s) => format!(
                    "the dynamic type `{}` (representing `{}`) does not support hashing",
                    type_path,
                    s.type_path()
                ),
            }
        }
    }}
}

/// An ordered mapping between reflected values.
#[derive(Default)]
pub struct DynamicMap {
    represented_type: Option<&'static TypeInfo>,
    values: Vec<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)>,
    indices: HashTable<usize>,
}

impl DynamicMap {
    /// Sets the [type] to be represented by this `DynamicMap`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Map`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Map(_)),
                "expected TypeInfo::Map but received: {:?}",
                represented_type
            );
        }

        self.represented_type = represented_type;
    }

    /// Inserts a typed key-value pair into the map.
    pub fn insert<K: PartialReflect, V: PartialReflect>(&mut self, key: K, value: V) {
        self.insert_boxed(Box::new(key), Box::new(value));
    }

    fn internal_hash(value: &dyn PartialReflect) -> u64 {
        value.reflect_hash().expect(&hash_error!(value))
    }

    fn internal_eq<'a>(
        value: &'a dyn PartialReflect,
        values: &'a [(Box<dyn PartialReflect>, Box<dyn PartialReflect>)],
    ) -> impl FnMut(&usize) -> bool + 'a {
        |&index| {
            value
            .reflect_partial_eq(&*values[index].0)
            .expect("underlying type does not reflect `PartialEq` and hence doesn't support equality checks")
        }
    }
}

impl Map for DynamicMap {
    fn get(&self, key: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
        let hash = Self::internal_hash(key);
        let eq = Self::internal_eq(key, &self.values);
        self.indices
            .find(hash, eq)
            .map(|&index| &*self.values[index].1)
    }

    fn get_mut(&mut self, key: &dyn PartialReflect) -> Option<&mut dyn PartialReflect> {
        let hash = Self::internal_hash(key);
        let eq = Self::internal_eq(key, &self.values);
        self.indices
            .find(hash, eq)
            .map(|&index| &mut *self.values[index].1)
    }

    fn get_at(&self, index: usize) -> Option<(&dyn PartialReflect, &dyn PartialReflect)> {
        self.values
            .get(index)
            .map(|(key, value)| (&**key, &**value))
    }

    fn get_at_mut(
        &mut self,
        index: usize,
    ) -> Option<(&dyn PartialReflect, &mut dyn PartialReflect)> {
        self.values
            .get_mut(index)
            .map(|(key, value)| (&**key, &mut **value))
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn iter(&self) -> MapIter {
        MapIter::new(self)
    }

    fn drain(&mut self) -> Vec<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
        self.values.drain(..).collect()
    }

    fn insert_boxed(
        &mut self,
        key: Box<dyn PartialReflect>,
        value: Box<dyn PartialReflect>,
    ) -> Option<Box<dyn PartialReflect>> {
        assert_eq!(
            key.reflect_partial_eq(&*key),
            Some(true),
            "keys inserted in `Map`-like types are expected to reflect `PartialEq`"
        );

        let hash = Self::internal_hash(&*key);
        let eq = Self::internal_eq(&*key, &self.values);
        match self.indices.find(hash, eq) {
            Some(&index) => {
                let (key_ref, value_ref) = &mut self.values[index];
                *key_ref = key;
                let old_value = core::mem::replace(value_ref, value);
                Some(old_value)
            }
            None => {
                let index = self.values.len();
                self.values.push((key, value));
                self.indices.insert_unique(hash, index, |&index| {
                    Self::internal_hash(&*self.values[index].0)
                });
                None
            }
        }
    }

    fn remove(&mut self, key: &dyn PartialReflect) -> Option<Box<dyn PartialReflect>> {
        let hash = Self::internal_hash(key);
        let eq = Self::internal_eq(key, &self.values);
        match self.indices.find_entry(hash, eq) {
            Ok(entry) => {
                let (index, _) = entry.remove();
                let (_, old_value) = self.values.swap_remove(index);

                // The `swap_remove` might have moved the last element of `values`
                // to `index`, so we might need to fix up its index in `indices`.
                // If the removed element was also the last element there's nothing to
                // fixup and this will return `None`, otherwise it returns the key
                // whose index needs to be fixed up.
                if let Some((moved_key, _)) = self.values.get(index) {
                    let hash = Self::internal_hash(&**moved_key);
                    let moved_index = self
                        .indices
                        .find_mut(hash, |&moved_index| moved_index == self.values.len())
                        .expect("key inserted in a `DynamicMap` is no longer present, this means its reflected `Hash` might be incorrect");
                    *moved_index = index;
                }

                Some(old_value)
            }
            Err(_) => None,
        }
    }
}

impl PartialReflect for DynamicMap {
    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.represented_type
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

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Err(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        None
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        None
    }

    fn apply(&mut self, value: &dyn PartialReflect) {
        map_apply(self, value);
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        map_try_apply(self, value)
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

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        map_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicMap(")?;
        map_debug(self, f)?;
        write!(f, ")")
    }

    #[inline]
    fn is_dynamic(&self) -> bool {
        true
    }
}

impl_type_path!((in bevy_reflect) DynamicMap);

impl Debug for DynamicMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.debug(f)
    }
}

/// An iterator over the key-value pairs of a [`Map`].
pub struct MapIter<'a> {
    map: &'a dyn Map,
    index: usize,
}

impl MapIter<'_> {
    /// Creates a new [`MapIter`].
    #[inline]
    pub const fn new(map: &dyn Map) -> MapIter {
        MapIter { map, index: 0 }
    }
}

impl<'a> Iterator for MapIter<'a> {
    type Item = (&'a dyn PartialReflect, &'a dyn PartialReflect);

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.map.get_at(self.index);
        self.index += value.is_some() as usize;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.map.len();
        (size, Some(size))
    }
}

impl FromIterator<(Box<dyn PartialReflect>, Box<dyn PartialReflect>)> for DynamicMap {
    fn from_iter<I: IntoIterator<Item = (Box<dyn PartialReflect>, Box<dyn PartialReflect>)>>(
        items: I,
    ) -> Self {
        let mut map = Self::default();
        for (key, value) in items.into_iter() {
            map.insert_boxed(key, value);
        }
        map
    }
}

impl<K: Reflect, V: Reflect> FromIterator<(K, V)> for DynamicMap {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(items: I) -> Self {
        let mut map = Self::default();
        for (key, value) in items.into_iter() {
            map.insert(key, value);
        }
        map
    }
}

impl IntoIterator for DynamicMap {
    type Item = (Box<dyn PartialReflect>, Box<dyn PartialReflect>);
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a> IntoIterator for &'a DynamicMap {
    type Item = (&'a dyn PartialReflect, &'a dyn PartialReflect);
    type IntoIter = MapIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> ExactSizeIterator for MapIter<'a> {}

/// Compares a [`Map`] with a [`PartialReflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a map;
/// - `b` is the same length as `a`;
/// - For each key-value pair in `a`, `b` contains a value for the given key,
///   and [`PartialReflect::reflect_partial_eq`] returns `Some(true)` for the two values.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn map_partial_eq<M: Map + ?Sized>(a: &M, b: &dyn PartialReflect) -> Option<bool> {
    let ReflectRef::Map(map) = b.reflect_ref() else {
        return Some(false);
    };

    if a.len() != map.len() {
        return Some(false);
    }

    for (key, value) in a.iter() {
        if let Some(map_value) = map.get(key) {
            let eq_result = value.reflect_partial_eq(map_value);
            if let failed @ (Some(false) | None) = eq_result {
                return failed;
            }
        } else {
            return Some(false);
        }
    }

    Some(true)
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

/// Applies the elements of reflected map `b` to the corresponding elements of map `a`.
///
/// If a key from `b` does not exist in `a`, the value is cloned and inserted.
///
/// # Panics
///
/// This function panics if `b` is not a reflected map.
#[inline]
pub fn map_apply<M: Map>(a: &mut M, b: &dyn PartialReflect) {
    if let Err(err) = map_try_apply(a, b) {
        panic!("{err}");
    }
}

/// Tries to apply the elements of reflected map `b` to the corresponding elements of map `a`
/// and returns a Result.
///
/// If a key from `b` does not exist in `a`, the value is cloned and inserted.
///
/// # Errors
///
/// This function returns an [`ApplyError::MismatchedKinds`] if `b` is not a reflected map or if
/// applying elements to each other fails.
#[inline]
pub fn map_try_apply<M: Map>(a: &mut M, b: &dyn PartialReflect) -> Result<(), ApplyError> {
    let map_value = b.reflect_ref().as_map()?;

    for (key, b_value) in map_value.iter() {
        if let Some(a_value) = a.get_mut(key) {
            a_value.try_apply(b_value)?;
        } else {
            a.insert_boxed(key.to_dynamic(), b_value.to_dynamic());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DynamicMap, Map};
    use alloc::{
        borrow::ToOwned,
        string::{String, ToString},
    };

    #[test]
    fn test_into_iter() {
        let expected = ["foo", "bar", "baz"];

        let mut map = DynamicMap::default();
        map.insert(0usize, expected[0].to_string());
        map.insert(1usize, expected[1].to_string());
        map.insert(2usize, expected[2].to_string());

        for (index, item) in map.into_iter().enumerate() {
            let key = item
                .0
                .try_take::<usize>()
                .expect("couldn't downcast to usize");
            let value = item
                .1
                .try_take::<String>()
                .expect("couldn't downcast to String");
            assert_eq!(index, key);
            assert_eq!(expected[index], value);
        }
    }

    #[test]
    fn test_map_get_at() {
        let values = ["first", "second", "third"];
        let mut map = DynamicMap::default();
        map.insert(0usize, values[0].to_string());
        map.insert(1usize, values[1].to_string());
        map.insert(1usize, values[2].to_string());

        let (key_r, value_r) = map.get_at(1).expect("Item wasn't found");
        let value = value_r
            .try_downcast_ref::<String>()
            .expect("Couldn't downcast to String");
        let key = key_r
            .try_downcast_ref::<usize>()
            .expect("Couldn't downcast to usize");
        assert_eq!(key, &1usize);
        assert_eq!(value, &values[2].to_owned());

        assert!(map.get_at(2).is_none());
        map.remove(&1usize);
        assert!(map.get_at(1).is_none());
    }

    #[test]
    fn test_map_get_at_mut() {
        let values = ["first", "second", "third"];
        let mut map = DynamicMap::default();
        map.insert(0usize, values[0].to_string());
        map.insert(1usize, values[1].to_string());
        map.insert(1usize, values[2].to_string());

        let (key_r, value_r) = map.get_at_mut(1).expect("Item wasn't found");
        let value = value_r
            .try_downcast_mut::<String>()
            .expect("Couldn't downcast to String");
        let key = key_r
            .try_downcast_ref::<usize>()
            .expect("Couldn't downcast to usize");
        assert_eq!(key, &1usize);
        assert_eq!(value, &mut values[2].to_owned());

        value.clone_from(&values[0].to_owned());

        assert_eq!(
            map.get(&1usize)
                .expect("Item wasn't found")
                .try_downcast_ref::<String>()
                .expect("Couldn't downcast to String"),
            &values[0].to_owned()
        );

        assert!(map.get_at(2).is_none());
    }

    #[test]
    fn next_index_increment() {
        let values = ["first", "last"];
        let mut map = DynamicMap::default();
        map.insert(0usize, values[0]);
        map.insert(1usize, values[1]);

        let mut iter = map.iter();
        let size = iter.len();

        for _ in 0..2 {
            let prev_index = iter.index;
            assert!(iter.next().is_some());
            assert_eq!(prev_index, iter.index - 1);
        }

        // When None we should no longer increase index
        for _ in 0..2 {
            assert!(iter.next().is_none());
            assert_eq!(size, iter.index);
        }
    }

    #[test]
    fn remove() {
        let mut map = DynamicMap::default();
        map.insert(0, 0);
        map.insert(1, 1);

        assert_eq!(map.remove(&0).unwrap().try_downcast_ref(), Some(&0));
        assert!(map.get(&0).is_none());
        assert_eq!(map.get(&1).unwrap().try_downcast_ref(), Some(&1));

        assert_eq!(map.remove(&1).unwrap().try_downcast_ref(), Some(&1));
        assert!(map.get(&1).is_none());

        assert!(map.remove(&1).is_none());
        assert!(map.get(&1).is_none());
    }
}
