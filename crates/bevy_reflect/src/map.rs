use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;

use bevy_utils::{Entry, HashMap};

use crate::utility::NonGenericTypeInfoCell;
use crate::{DynamicInfo, Reflect, ReflectMut, ReflectOwned, ReflectRef, TypeInfo, Typed};

/// An ordered mapping between [`Reflect`] values.
///
/// Because the values are reflected, the underlying types of keys and values
/// may differ between entries.
///
///`ReflectValue` `Keys` are assumed to return a non-`None` hash. The ordering
/// of `Map` entries is not guaranteed to be stable across runs or between
/// instances.
///
/// This trait corresponds to types like [`std::collections::HashMap`].
pub trait Map: Reflect {
    /// Returns a reference to the value associated with the given key.
    ///
    /// If no value is associated with `key`, returns `None`.
    fn get(&self, key: &dyn Reflect) -> Option<&dyn Reflect>;

    /// Returns a mutable reference to the value associated with the given key.
    ///
    /// If no value is associated with `key`, returns `None`.
    fn get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect>;

    /// Returns the key-value pair at `index` by reference, or `None` if out of bounds.
    fn get_at(&self, index: usize) -> Option<(&dyn Reflect, &dyn Reflect)>;

    /// Returns the number of elements in the map.
    fn len(&self) -> usize;

    /// Returns `true` if the list contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the key-value pairs of the map.
    fn iter(&self) -> MapIter;

    /// Drain the key-value pairs of this map to get a vector of owned values.
    fn drain(self: Box<Self>) -> Vec<(Box<dyn Reflect>, Box<dyn Reflect>)>;

    /// Clones the map, producing a [`DynamicMap`].
    fn clone_dynamic(&self) -> DynamicMap;

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    /// If the map did have this key present, the value is updated, and the old value is returned.
    fn insert_boxed(
        &mut self,
        key: Box<dyn Reflect>,
        value: Box<dyn Reflect>,
    ) -> Option<Box<dyn Reflect>>;
}

/// A container for compile-time map info.
#[derive(Clone, Debug)]
pub struct MapInfo {
    type_name: &'static str,
    type_id: TypeId,
    key_type_name: &'static str,
    key_type_id: TypeId,
    value_type_name: &'static str,
    value_type_id: TypeId,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl MapInfo {
    /// Create a new [`MapInfo`].
    pub fn new<TMap: Map, TKey: Hash + Reflect, TValue: Reflect>() -> Self {
        Self {
            type_name: std::any::type_name::<TMap>(),
            type_id: TypeId::of::<TMap>(),
            key_type_name: std::any::type_name::<TKey>(),
            key_type_id: TypeId::of::<TKey>(),
            value_type_name: std::any::type_name::<TValue>(),
            value_type_id: TypeId::of::<TValue>(),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this map.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// The [type name] of the map.
    ///
    /// [type name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The [`TypeId`] of the map.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the map type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// The [type name] of the key.
    ///
    /// [type name]: std::any::type_name
    pub fn key_type_name(&self) -> &'static str {
        self.key_type_name
    }

    /// The [`TypeId`] of the key.
    pub fn key_type_id(&self) -> TypeId {
        self.key_type_id
    }

    /// Check if the given type matches the key type.
    pub fn key_is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.key_type_id
    }

    /// The [type name] of the value.
    ///
    /// [type name]: std::any::type_name
    pub fn value_type_name(&self) -> &'static str {
        self.value_type_name
    }

    /// The [`TypeId`] of the value.
    pub fn value_type_id(&self) -> TypeId {
        self.value_type_id
    }

    /// Check if the given type matches the value type.
    pub fn value_is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.value_type_id
    }

    /// The docstring of this map, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }
}

const HASH_ERROR: &str = "the given key does not support hashing";

/// An ordered mapping between reflected values.
#[derive(Default)]
pub struct DynamicMap {
    name: String,
    values: Vec<(Box<dyn Reflect>, Box<dyn Reflect>)>,
    indices: HashMap<u64, usize>,
}

impl DynamicMap {
    /// Returns the type name of the map.
    ///
    /// The value returned by this method is the same value returned by
    /// [`Reflect::type_name`].
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the type name of the map.
    ///
    /// The value set by this method is the same value returned by
    /// [`Reflect::type_name`].
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Inserts a typed key-value pair into the map.
    pub fn insert<K: Reflect, V: Reflect>(&mut self, key: K, value: V) {
        self.insert_boxed(Box::new(key), Box::new(value));
    }
}

impl Map for DynamicMap {
    fn get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
        self.indices
            .get(&key.reflect_hash().expect(HASH_ERROR))
            .map(|index| &*self.values.get(*index).unwrap().1)
    }

    fn get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
        self.indices
            .get(&key.reflect_hash().expect(HASH_ERROR))
            .cloned()
            .map(move |index| &mut *self.values.get_mut(index).unwrap().1)
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn clone_dynamic(&self) -> DynamicMap {
        DynamicMap {
            name: self.name.clone(),
            values: self
                .values
                .iter()
                .map(|(key, value)| (key.clone_value(), value.clone_value()))
                .collect(),
            indices: self.indices.clone(),
        }
    }

    fn iter(&self) -> MapIter {
        MapIter {
            map: self,
            index: 0,
        }
    }

    fn get_at(&self, index: usize) -> Option<(&dyn Reflect, &dyn Reflect)> {
        self.values
            .get(index)
            .map(|(key, value)| (&**key, &**value))
    }

    fn insert_boxed(
        &mut self,
        key: Box<dyn Reflect>,
        mut value: Box<dyn Reflect>,
    ) -> Option<Box<dyn Reflect>> {
        match self.indices.entry(key.reflect_hash().expect(HASH_ERROR)) {
            Entry::Occupied(entry) => {
                let (_old_key, old_value) = self.values.get_mut(*entry.get()).unwrap();
                std::mem::swap(old_value, &mut value);
                Some(value)
            }
            Entry::Vacant(entry) => {
                entry.insert(self.values.len());
                self.values.push((key, value));
                None
            }
        }
    }

    fn drain(self: Box<Self>) -> Vec<(Box<dyn Reflect>, Box<dyn Reflect>)> {
        self.values
    }
}

impl Reflect for DynamicMap {
    fn type_name(&self) -> &str {
        &self.name
    }

    #[inline]
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

    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
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

    fn debug(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynamicMap(")?;
        map_debug(self, f)?;
        write!(f, ")")
    }
}

impl Debug for DynamicMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

impl Typed for DynamicMap {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Dynamic(DynamicInfo::new::<Self>()))
    }
}

/// An iterator over the key-value pairs of a [`Map`].
pub struct MapIter<'a> {
    pub(crate) map: &'a dyn Map,
    pub(crate) index: usize,
}

impl<'a> Iterator for MapIter<'a> {
    type Item = (&'a dyn Reflect, &'a dyn Reflect);

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.map.get_at(self.index);
        self.index += 1;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.map.len();
        (size, Some(size))
    }
}

impl IntoIterator for DynamicMap {
    type Item = (Box<dyn Reflect>, Box<dyn Reflect>);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a> ExactSizeIterator for MapIter<'a> {}

/// Compares a [`Map`] with a [`Reflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a map;
/// - `b` is the same length as `a`;
/// - For each key-value pair in `a`, `b` contains a value for the given key,
///   and [`Reflect::reflect_partial_eq`] returns `Some(true)` for the two values.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn map_partial_eq<M: Map>(a: &M, b: &dyn Reflect) -> Option<bool> {
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
/// # use bevy_utils::HashMap;
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
pub fn map_debug(dyn_map: &dyn Map, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
pub fn map_apply<M: Map>(a: &mut M, b: &dyn Reflect) {
    if let ReflectRef::Map(map_value) = b.reflect_ref() {
        for (key, b_value) in map_value.iter() {
            if let Some(a_value) = a.get_mut(key) {
                a_value.apply(b_value);
            } else {
                a.insert_boxed(key.clone_value(), b_value.clone_value());
            }
        }
    } else {
        panic!("Attempted to apply a non-map type to a map type.");
    }
}

#[cfg(test)]
mod tests {
    use super::DynamicMap;

    #[test]
    fn test_into_iter() {
        let expected = vec!["foo", "bar", "baz"];

        let mut map = DynamicMap::default();
        map.insert(0usize, expected[0].to_string());
        map.insert(1usize, expected[1].to_string());
        map.insert(2usize, expected[2].to_string());

        for (index, item) in map.into_iter().enumerate() {
            let key = item.0.take::<usize>().expect("couldn't downcast to usize");
            let value = item
                .1
                .take::<String>()
                .expect("couldn't downcast to String");
            assert_eq!(index, key);
            assert_eq!(expected[index], value);
        }
    }
}
