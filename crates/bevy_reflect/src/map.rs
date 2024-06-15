use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};

use bevy_reflect_derive::impl_type_path;
use bevy_utils::{Entry, HashMap};

use crate::{
    self as bevy_reflect, ApplyError, Reflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef,
    TypeInfo, TypePath, TypePathTable,
};

/// A trait used to power [map-like] operations via [reflection].
///
/// Maps contain zero or more entries of a key and its associated value,
/// and correspond to types like [`HashMap`].
/// The order of these entries is not guaranteed by this trait.
///
/// # Hashing
///
/// All keys are expected to return a valid hash value from [`Reflect::reflect_hash`].
/// If using the [`#[derive(Reflect)]`](derive@crate::Reflect) macro, this can be done by adding `#[reflect(Hash)]`
/// to the entire struct or enum.
/// This is true even for manual implementors who do not use the hashed value,
/// as it is still relied on by [`DynamicMap`].
///
/// # Example
///
/// ```
/// use bevy_reflect::{Reflect, Map};
/// use bevy_utils::HashMap;
///
///
/// let foo: &mut dyn Map = &mut HashMap::<u32, bool>::new();
/// foo.insert_boxed(Box::new(123_u32), Box::new(true));
/// assert_eq!(foo.len(), 1);
///
/// let field: &dyn Reflect = foo.get(&123_u32).unwrap();
/// assert_eq!(field.downcast_ref::<bool>(), Some(&true));
/// ```
///
/// [map-like]: https://doc.rust-lang.org/book/ch08-03-hash-maps.html
/// [reflection]: crate
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

    /// Returns the key-value pair at `index` by reference where the value is a mutable reference, or `None` if out of bounds.
    fn get_at_mut(&mut self, index: usize) -> Option<(&dyn Reflect, &mut dyn Reflect)>;

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

    /// Removes an entry from the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    /// If the map did have this key present, the removed value is returned.
    fn remove(&mut self, key: &dyn Reflect) -> Option<Box<dyn Reflect>>;
}

/// A container for compile-time map info.
#[derive(Clone, Debug)]
pub struct MapInfo {
    type_path: TypePathTable,
    type_id: TypeId,
    key_type_path: TypePathTable,
    key_type_id: TypeId,
    value_type_path: TypePathTable,
    value_type_id: TypeId,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl MapInfo {
    /// Create a new [`MapInfo`].
    pub fn new<TMap: Map + TypePath, TKey: Reflect + TypePath, TValue: Reflect + TypePath>() -> Self
    {
        Self {
            type_path: TypePathTable::of::<TMap>(),
            type_id: TypeId::of::<TMap>(),
            key_type_path: TypePathTable::of::<TKey>(),
            key_type_id: TypeId::of::<TKey>(),
            value_type_path: TypePathTable::of::<TValue>(),
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

    /// A representation of the type path of the map.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn type_path_table(&self) -> &TypePathTable {
        &self.type_path
    }

    /// The [stable, full type path] of the map.
    ///
    /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
    ///
    /// [stable, full type path]: TypePath
    /// [`type_path_table`]: Self::type_path_table
    pub fn type_path(&self) -> &'static str {
        self.type_path_table().path()
    }

    /// The [`TypeId`] of the map.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the map type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
    }

    /// A representation of the type path of the key type.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn key_type_path_table(&self) -> &TypePathTable {
        &self.key_type_path
    }

    /// The [`TypeId`] of the key.
    pub fn key_type_id(&self) -> TypeId {
        self.key_type_id
    }

    /// Check if the given type matches the key type.
    pub fn key_is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.key_type_id
    }

    /// A representation of the type path of the value type.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn value_type_path_table(&self) -> &TypePathTable {
        &self.value_type_path
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
        .as_str()
    }}
}

/// An ordered mapping between reflected values.
#[derive(Default)]
pub struct DynamicMap {
    represented_type: Option<&'static TypeInfo>,
    values: Vec<(Box<dyn Reflect>, Box<dyn Reflect>)>,
    indices: HashMap<u64, usize>,
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
    pub fn insert<K: Reflect, V: Reflect>(&mut self, key: K, value: V) {
        self.insert_boxed(Box::new(key), Box::new(value));
    }
}

impl Map for DynamicMap {
    fn get(&self, key: &dyn Reflect) -> Option<&dyn Reflect> {
        self.indices
            .get(&key.reflect_hash().expect(hash_error!(key)))
            .map(|index| &*self.values.get(*index).unwrap().1)
    }

    fn get_mut(&mut self, key: &dyn Reflect) -> Option<&mut dyn Reflect> {
        self.indices
            .get(&key.reflect_hash().expect(hash_error!(key)))
            .cloned()
            .map(move |index| &mut *self.values.get_mut(index).unwrap().1)
    }

    fn get_at(&self, index: usize) -> Option<(&dyn Reflect, &dyn Reflect)> {
        self.values
            .get(index)
            .map(|(key, value)| (&**key, &**value))
    }

    fn get_at_mut(&mut self, index: usize) -> Option<(&dyn Reflect, &mut dyn Reflect)> {
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

    fn drain(self: Box<Self>) -> Vec<(Box<dyn Reflect>, Box<dyn Reflect>)> {
        self.values
    }

    fn clone_dynamic(&self) -> DynamicMap {
        DynamicMap {
            represented_type: self.represented_type,
            values: self
                .values
                .iter()
                .map(|(key, value)| (key.clone_value(), value.clone_value()))
                .collect(),
            indices: self.indices.clone(),
        }
    }

    fn insert_boxed(
        &mut self,
        key: Box<dyn Reflect>,
        mut value: Box<dyn Reflect>,
    ) -> Option<Box<dyn Reflect>> {
        match self
            .indices
            .entry(key.reflect_hash().expect(hash_error!(key)))
        {
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

    fn remove(&mut self, key: &dyn Reflect) -> Option<Box<dyn Reflect>> {
        let index = self
            .indices
            .remove(&key.reflect_hash().expect(hash_error!(key)))?;
        let (_key, value) = self.values.remove(index);
        Some(value)
    }
}

impl Reflect for DynamicMap {
    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        self.represented_type
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

    fn try_apply(&mut self, value: &dyn Reflect) -> Result<(), ApplyError> {
        map_try_apply(self, value)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
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

    #[inline]
    fn is_dynamic(&self) -> bool {
        true
    }
}

impl_type_path!((in bevy_reflect) DynamicMap);

impl Debug for DynamicMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

/// An iterator over the key-value pairs of a [`Map`].
pub struct MapIter<'a> {
    map: &'a dyn Map,
    index: usize,
}

impl<'a> MapIter<'a> {
    /// Creates a new [`MapIter`].
    #[inline]
    pub const fn new(map: &'a dyn Map) -> MapIter {
        MapIter { map, index: 0 }
    }
}

impl<'a> Iterator for MapIter<'a> {
    type Item = (&'a dyn Reflect, &'a dyn Reflect);

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
pub fn map_debug(dyn_map: &dyn Map, f: &mut Formatter<'_>) -> std::fmt::Result {
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
pub fn map_try_apply<M: Map>(a: &mut M, b: &dyn Reflect) -> Result<(), ApplyError> {
    if let ReflectRef::Map(map_value) = b.reflect_ref() {
        for (key, b_value) in map_value.iter() {
            if let Some(a_value) = a.get_mut(key) {
                a_value.try_apply(b_value)?;
            } else {
                a.insert_boxed(key.clone_value(), b_value.clone_value());
            }
        }
    } else {
        return Err(ApplyError::MismatchedKinds {
            from_kind: b.reflect_kind(),
            to_kind: ReflectKind::Map,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::DynamicMap;
    use super::Map;
    use crate::reflect::Reflect;

    #[test]
    fn test_into_iter() {
        let expected = ["foo", "bar", "baz"];

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

    #[test]
    fn test_map_get_at() {
        let values = ["first", "second", "third"];
        let mut map = DynamicMap::default();
        map.insert(0usize, values[0].to_string());
        map.insert(1usize, values[1].to_string());
        map.insert(1usize, values[2].to_string());

        let (key_r, value_r) = map.get_at(1).expect("Item wasn't found");
        let value = value_r
            .downcast_ref::<String>()
            .expect("Couldn't downcast to String");
        let key = key_r
            .downcast_ref::<usize>()
            .expect("Couldn't downcast to usize");
        assert_eq!(key, &1usize);
        assert_eq!(value, &values[2].to_owned());

        assert!(map.get_at(2).is_none());
        map.remove(&1usize as &dyn Reflect);
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
            .downcast_mut::<String>()
            .expect("Couldn't downcast to String");
        let key = key_r
            .downcast_ref::<usize>()
            .expect("Couldn't downcast to usize");
        assert_eq!(key, &1usize);
        assert_eq!(value, &mut values[2].to_owned());

        value.clone_from(&values[0].to_owned());

        assert_eq!(
            map.get(&1usize as &dyn Reflect)
                .expect("Item wasn't found")
                .downcast_ref::<String>()
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
}
