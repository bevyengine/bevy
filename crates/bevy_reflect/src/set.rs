use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};

use bevy_reflect_derive::impl_type_path;
use bevy_utils::{Entry, HashMap};

use crate::{
    self as bevy_reflect, Reflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef, TypeInfo,
    TypePath, TypePathTable,
};

/// A trait used to power [set-like] operations via [reflection].
///
/// Set contain zero or more entries of a value, and correspond to types like [`HashSet`]. The
/// order of these entries is not guaranteed by this trait.
///
/// # Hashing
///
/// All keys are expected to return a valid hash value from [`Reflect::reflect_hash`].
/// If using the [`#[derive(Reflect)]`](derive@crate::Reflect) macro, this can be done by adding `#[reflect(Hash)]`
/// to the entire struct or enum.
/// This is true even for manual implementors who do not use the hashed value,
/// as it is still relied on by [`DynamicSet`].
///
/// # Example
///
/// ```
/// use bevy_reflect::{Reflect, Set};
/// use bevy_utils::HashSet;
///
///
/// let foo: &mut dyn Set = &mut HashSet::<u32>::new();
/// foo.insert_boxed(Box::new(123_u32));
/// assert_eq!(foo.len(), 1);
///
/// let field: &dyn Reflect = foo.get(&123_u32).unwrap();
/// assert_eq!(field.downcast_ref::<u32>(), Some(&123_u32));
/// ```
///
/// [set-like]: https://doc.rust-lang.org/stable/std/collections/struct.HashSet.html
/// [reflection]: crate
pub trait Set: Reflect {
    /// Returns a reference to the value.
    ///
    /// If no value is contained, returns `None`.
    fn get(&self, key: &dyn Reflect) -> Option<&dyn Reflect>;

    /// Returns the value at `index` by reference, or `None` if out of bounds.
    fn get_at(&self, index: usize) -> Option<&dyn Reflect>;

    /// Returns the number of elements in the set.
    fn len(&self) -> usize;

    /// Returns `true` if the list contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the values of the set.
    fn iter(&self) -> SetIter;

    /// Drain the values of this set to get a vector of owned values.
    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>>;

    /// Clones the set, producing a [`DynamicSet`].
    fn clone_dynamic(&self) -> DynamicSet;

    /// Inserts a value into the set.
    ///
    /// If the set did not have this value present, `None` is returned.
    /// If the set did have this value present, it is updated, and the old value is returned.
    fn insert_boxed(&mut self, value: Box<dyn Reflect>) -> Option<Box<dyn Reflect>>;

    /// Removes an value from the set.
    ///
    /// If the set did not have this value present, `None` is returned.
    /// If the set did have this value present, it is returned.
    fn remove(&mut self, key: &dyn Reflect) -> Option<Box<dyn Reflect>>;
}

/// A container for compile-time set info.
#[derive(Clone, Debug)]
pub struct SetInfo {
    type_path: TypePathTable,
    type_id: TypeId,
    value_type_path: TypePathTable,
    value_type_id: TypeId,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl SetInfo {
    /// Create a new [`SetInfo`].
    pub fn new<TSet: Set + TypePath, TValue: Reflect + TypePath>() -> Self {
        Self {
            type_path: TypePathTable::of::<TSet>(),
            type_id: TypeId::of::<TSet>(),
            value_type_path: TypePathTable::of::<TValue>(),
            value_type_id: TypeId::of::<TValue>(),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this set.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    /// A representation of the type path of the set.
    ///
    /// Provides dynamic access to all methods on [`TypePath`].
    pub fn type_path_table(&self) -> &TypePathTable {
        &self.type_path
    }

    /// The [stable, full type path] of the set.
    ///
    /// Use [`type_path_table`] if you need access to the other methods on [`TypePath`].
    ///
    /// [stable, full type path]: TypePath
    /// [`type_path_table`]: Self::type_path_table
    pub fn type_path(&self) -> &'static str {
        self.type_path_table().path()
    }

    /// The [`TypeId`] of the set.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the given type matches the set type.
    pub fn is<T: Any>(&self) -> bool {
        TypeId::of::<T>() == self.type_id
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

    /// The docstring of this set, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }
}

const HASH_ERROR: &str = "the given key does not support hashing";

/// An ordered mapping between reflected values.
#[derive(Default)]
pub struct DynamicSet {
    represented_type: Option<&'static TypeInfo>,
    values: Vec<Box<dyn Reflect>>,
    indices: HashMap<u64, usize>,
}

impl DynamicSet {
    /// Sets the [type] to be represented by this `DynamicSet`.
    ///
    /// # Panics
    ///
    /// Panics if the given [type] is not a [`TypeInfo::Set`].
    ///
    /// [type]: TypeInfo
    pub fn set_represented_type(&mut self, represented_type: Option<&'static TypeInfo>) {
        if let Some(represented_type) = represented_type {
            assert!(
                matches!(represented_type, TypeInfo::Set(_)),
                "expected TypeInfo::Set but received: {:?}",
                represented_type
            );
        }

        self.represented_type = represented_type;
    }

    /// Inserts a typed value into the set.
    pub fn insert<V: Reflect>(&mut self, value: V) {
        self.insert_boxed(Box::new(value));
    }
}

impl Set for DynamicSet {
    fn get(&self, value: &dyn Reflect) -> Option<&dyn Reflect> {
        self.indices
            .get(&value.reflect_hash().expect(HASH_ERROR))
            .map(|index| &**self.values.get(*index).unwrap())
    }

    fn get_at(&self, index: usize) -> Option<&dyn Reflect> {
        self.values.get(index).map(|value| &**value)
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn iter(&self) -> SetIter {
        SetIter::new(self)
    }

    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
        self.values
    }

    fn clone_dynamic(&self) -> DynamicSet {
        DynamicSet {
            represented_type: self.represented_type,
            values: self
                .values
                .iter()
                .map(|value| value.clone_value())
                .collect(),
            indices: self.indices.clone(),
        }
    }

    fn insert_boxed(&mut self, mut value: Box<dyn Reflect>) -> Option<Box<dyn Reflect>> {
        match self.indices.entry(value.reflect_hash().expect(HASH_ERROR)) {
            Entry::Occupied(entry) => {
                let old_value = self.values.get_mut(*entry.get()).unwrap();
                std::mem::swap(old_value, &mut value);
                Some(value)
            }
            Entry::Vacant(entry) => {
                entry.insert(self.values.len());
                self.values.push(value);
                None
            }
        }
    }

    fn remove(&mut self, key: &dyn Reflect) -> Option<Box<dyn Reflect>> {
        let index = self
            .indices
            .remove(&key.reflect_hash().expect(HASH_ERROR))?;
        let value = self.values.remove(index);
        Some(value)
    }
}

impl Reflect for DynamicSet {
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
        set_apply(self, value);
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
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

    fn clone_value(&self) -> Box<dyn Reflect> {
        Box::new(self.clone_dynamic())
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        set_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DynamicSet(")?;
        set_debug(self, f)?;
        write!(f, ")")
    }

    #[inline]
    fn is_dynamic(&self) -> bool {
        true
    }
}

impl_type_path!((in bevy_reflect) DynamicSet);

impl Debug for DynamicSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.debug(f)
    }
}

/// An iterator over the values of a [`Set`].
pub struct SetIter<'a> {
    set: &'a dyn Set,
    index: usize,
}

impl<'a> SetIter<'a> {
    /// Creates a new [`SetIter`].
    #[inline]
    pub const fn new(set: &'a dyn Set) -> SetIter {
        SetIter { set, index: 0 }
    }
}

impl<'a> Iterator for SetIter<'a> {
    type Item = &'a dyn Reflect;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.set.get_at(self.index);
        self.index += 1;
        value
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.set.len();
        (size, Some(size))
    }
}

impl IntoIterator for DynamicSet {
    type Item = Box<dyn Reflect>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a> ExactSizeIterator for SetIter<'a> {}

/// Compares a [`Set`] with a [`Reflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a set;
/// - `b` is the same length as `a`;
/// - For each value pair in `a`, `b` contains the value too,
///   and [`Reflect::reflect_partial_eq`] returns `Some(true)` for the two values.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn set_partial_eq<M: Set>(a: &M, b: &dyn Reflect) -> Option<bool> {
    let ReflectRef::Set(set) = b.reflect_ref() else {
        return Some(false);
    };

    if a.len() != set.len() {
        return Some(false);
    }

    for value in a.iter() {
        // REVIEW: just trying to do minimal adjustments here. Does this even make sense?
        if let Some(set_value) = set.get(value) {
            let eq_result = value.reflect_partial_eq(set_value);
            if let failed @ (Some(false) | None) = eq_result {
                return failed;
            }
        } else {
            return Some(false);
        }
    }

    Some(true)
}

/// The default debug formatter for [`Set`] types.
///
/// # Example
/// ```
/// # use bevy_utils::HashSet;
/// use bevy_reflect::Reflect;
///
/// let mut my_set = HashSet::new();
/// my_set.insert(String::from("Hello"));
/// println!("{:#?}", &my_set as &dyn Reflect);
///
/// // Output:
///
/// // {
/// //   "Hello",
/// // }
/// ```
#[inline]
pub fn set_debug(dyn_set: &dyn Set, f: &mut Formatter<'_>) -> std::fmt::Result {
    let mut debug = f.debug_set();
    for value in dyn_set.iter() {
        debug.entry(&value as &dyn Debug);
    }
    debug.finish()
}

/// Applies the elements of reflected set `b` to the corresponding elements of set `a`.
///
/// If a value from `b` does not exist in `a`, the value is cloned and inserted.
///
/// # Panics
///
/// This function panics if `b` is not a reflected set.
#[inline]
pub fn set_apply<M: Set>(a: &mut M, b: &dyn Reflect) {
    if let ReflectRef::Set(set_value) = b.reflect_ref() {
        for b_value in set_value.iter() {
            if a.get(b_value).is_none() {
                a.insert_boxed(b_value.clone_value());
            }
        }
    } else {
        panic!("Attempted to apply a non-set type to a set type.");
    }
}

#[cfg(test)]
mod tests {
    use super::DynamicSet;
    use super::Set;
    use crate::reflect::Reflect;

    #[test]
    fn test_into_iter() {
        let expected = ["foo", "bar", "baz"];

        let mut set = DynamicSet::default();
        set.insert(expected[0].to_string());
        set.insert(expected[1].to_string());
        set.insert(expected[2].to_string());

        for (index, item) in set.into_iter().enumerate() {
            let value = item.take::<String>().expect("couldn't downcast to String");
            assert_eq!(expected[index], value);
        }
    }

    #[test]
    fn test_set_get_at() {
        let values = ["first", "second", "second"];
        let mut set = DynamicSet::default();
        set.insert(values[0].to_string());
        set.insert(values[1].to_string());
        set.insert(values[2].to_string());

        let value_r = set.get_at(1).expect("Item wasn't found");
        let value = value_r
            .downcast_ref::<String>()
            .expect("Couldn't downcast to String");
        assert_eq!(value, &values[1].to_owned());

        assert!(set.get_at(2).is_none());
        set.remove(&String::from("first") as &dyn Reflect);
        assert!(set.get_at(1).is_none());
    }
}
