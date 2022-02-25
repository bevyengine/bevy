use std::any::Any;

use bevy_utils::{Entry, HashMap};

use crate::{serde::Serializable, Reflect, ReflectMut, ReflectRef};

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

    /// Clones the map, producing a [`DynamicMap`].
    fn clone_dynamic(&self) -> DynamicMap;
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

    /// Inserts a key-value pair of [`Reflect`] values into the map.
    pub fn insert_boxed(&mut self, key: Box<dyn Reflect>, value: Box<dyn Reflect>) {
        match self.indices.entry(key.reflect_hash().expect(HASH_ERROR)) {
            Entry::Occupied(entry) => {
                self.values[*entry.get()] = (key, value);
            }
            Entry::Vacant(entry) => {
                entry.insert(self.values.len());
                self.values.push((key, value));
            }
        }
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
}

// SAFE: any and any_mut both return self
unsafe impl Reflect for DynamicMap {
    fn type_name(&self) -> &str {
        &self.name
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
                if let Some(v) = self.get_mut(key) {
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

impl<'a> ExactSizeIterator for MapIter<'a> {}

/// Compares a [`Map`] with a [`Reflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a map;
/// - `b` is the same length as `a`;
/// - For each key-value pair in `a`, `b` contains a value for the given key,
///   and [`Reflect::reflect_partial_eq`] returns `Some(true)` for the two values.
#[inline]
pub fn map_partial_eq<M: Map>(a: &M, b: &dyn Reflect) -> Option<bool> {
    let map = if let ReflectRef::Map(map) = b.reflect_ref() {
        map
    } else {
        return Some(false);
    };

    if a.len() != map.len() {
        return Some(false);
    }

    for (key, value) in a.iter() {
        if let Some(map_value) = map.get(key) {
            if let Some(false) | None = value.reflect_partial_eq(map_value) {
                return Some(false);
            }
        } else {
            return Some(false);
        }
    }

    Some(true)
}
