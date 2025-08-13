use alloc::{boxed::Box, format, vec::Vec};
use core::fmt::{Debug, Formatter};

use bevy_platform::collections::{hash_table::OccupiedEntry as HashTableOccupiedEntry, HashTable};
use bevy_reflect_derive::impl_type_path;

use crate::{
    generics::impl_generic_info_methods, hash_error, type_info::impl_type_methods, ApplyError,
    Generics, PartialReflect, Reflect, ReflectKind, ReflectMut, ReflectOwned, ReflectRef, Type,
    TypeInfo, TypePath,
};

/// A trait used to power [set-like] operations via [reflection].
///
/// Sets contain zero or more entries of a fixed type, and correspond to types
/// like [`HashSet`] and [`BTreeSet`].
/// The order of these entries is not guaranteed by this trait.
///
/// # Hashing and equality
///
/// All values are expected to return a valid hash value from [`PartialReflect::reflect_hash`] and be
/// comparable using [`PartialReflect::reflect_partial_eq`].
/// If using the [`#[derive(Reflect)]`](derive@crate::Reflect) macro, this can be done by adding
/// `#[reflect(Hash, PartialEq)]` to the entire struct or enum.
/// The ordering is expected to be total, that is as if the reflected type implements the [`Eq`] trait.
/// This is true even for manual implementors who do not hash or compare values,
/// as it is still relied on by [`DynamicSet`].
///
/// # Example
///
/// ```
/// use bevy_reflect::{PartialReflect, Set};
/// use std::collections::HashSet;
///
///
/// let foo: &mut dyn Set = &mut HashSet::<u32>::new();
/// foo.insert_boxed(Box::new(123_u32));
/// assert_eq!(foo.len(), 1);
///
/// let field: &dyn PartialReflect = foo.get(&123_u32).unwrap();
/// assert_eq!(field.try_downcast_ref::<u32>(), Some(&123_u32));
/// ```
///
/// [`HashSet`]: std::collections::HashSet
/// [`BTreeSet`]: alloc::collections::BTreeSet
/// [set-like]: https://doc.rust-lang.org/stable/std/collections/struct.HashSet.html
/// [reflection]: crate
pub trait Set: PartialReflect {
    /// Returns a reference to the value.
    ///
    /// If no value is contained, returns `None`.
    fn get(&self, value: &dyn PartialReflect) -> Option<&dyn PartialReflect>;

    /// Returns the number of elements in the set.
    fn len(&self) -> usize;

    /// Returns `true` if the list contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the values of the set.
    fn iter(&self) -> Box<dyn Iterator<Item = &dyn PartialReflect> + '_>;

    /// Drain the values of this set to get a vector of owned values.
    ///
    /// After calling this function, `self` will be empty.
    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>>;

    /// Retain only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` for which `f(&e)` returns `false`.
    fn retain(&mut self, f: &mut dyn FnMut(&dyn PartialReflect) -> bool);

    /// Creates a new [`DynamicSet`] from this set.
    fn to_dynamic_set(&self) -> DynamicSet {
        let mut set = DynamicSet::default();
        set.set_represented_type(self.get_represented_type_info());
        for value in self.iter() {
            set.insert_boxed(value.to_dynamic());
        }
        set
    }

    /// Inserts a value into the set.
    ///
    /// If the set did not have this value present, `true` is returned.
    /// If the set did have this value present, `false` is returned.
    fn insert_boxed(&mut self, value: Box<dyn PartialReflect>) -> bool;

    /// Removes a value from the set.
    ///
    /// If the set did not have this value present, `true` is returned.
    /// If the set did have this value present, `false` is returned.
    fn remove(&mut self, value: &dyn PartialReflect) -> bool;

    /// Checks if the given value is contained in the set
    fn contains(&self, value: &dyn PartialReflect) -> bool;
}

/// A container for compile-time set info.
#[derive(Clone, Debug)]
pub struct SetInfo {
    ty: Type,
    generics: Generics,
    value_ty: Type,
    #[cfg(feature = "documentation")]
    docs: Option<&'static str>,
}

impl SetInfo {
    /// Create a new [`SetInfo`].
    pub fn new<TSet: Set + TypePath, TValue: Reflect + TypePath>() -> Self {
        Self {
            ty: Type::of::<TSet>(),
            generics: Generics::new(),
            value_ty: Type::of::<TValue>(),
            #[cfg(feature = "documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this set.
    #[cfg(feature = "documentation")]
    pub fn with_docs(self, docs: Option<&'static str>) -> Self {
        Self { docs, ..self }
    }

    impl_type_methods!(ty);

    /// The [type] of the value.
    ///
    /// [type]: Type
    pub fn value_ty(&self) -> Type {
        self.value_ty
    }

    /// The docstring of this set, if any.
    #[cfg(feature = "documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
}

/// An unordered set of reflected values.
#[derive(Default)]
pub struct DynamicSet {
    represented_type: Option<&'static TypeInfo>,
    hash_table: HashTable<Box<dyn PartialReflect>>,
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
                "expected TypeInfo::Set but received: {represented_type:?}"
            );
        }

        self.represented_type = represented_type;
    }

    /// Inserts a typed value into the set.
    pub fn insert<V: Reflect>(&mut self, value: V) {
        self.insert_boxed(Box::new(value));
    }

    fn internal_hash(value: &dyn PartialReflect) -> u64 {
        value.reflect_hash().expect(&hash_error!(value))
    }

    fn internal_eq(
        value: &dyn PartialReflect,
    ) -> impl FnMut(&Box<dyn PartialReflect>) -> bool + '_ {
        |other| {
            value
                .reflect_partial_eq(&**other)
                .expect("Underlying type does not reflect `PartialEq` and hence doesn't support equality checks")
        }
    }
}

impl Set for DynamicSet {
    fn get(&self, value: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
        self.hash_table
            .find(Self::internal_hash(value), Self::internal_eq(value))
            .map(|value| &**value)
    }

    fn len(&self) -> usize {
        self.hash_table.len()
    }

    fn iter(&self) -> Box<dyn Iterator<Item = &dyn PartialReflect> + '_> {
        let iter = self.hash_table.iter().map(|v| &**v);
        Box::new(iter)
    }

    fn drain(&mut self) -> Vec<Box<dyn PartialReflect>> {
        self.hash_table.drain().collect::<Vec<_>>()
    }

    fn retain(&mut self, f: &mut dyn FnMut(&dyn PartialReflect) -> bool) {
        self.hash_table.retain(move |value| f(&**value));
    }

    fn insert_boxed(&mut self, value: Box<dyn PartialReflect>) -> bool {
        assert_eq!(
            value.reflect_partial_eq(&*value),
            Some(true),
            "Values inserted in `Set` like types are expected to reflect `PartialEq`"
        );
        match self
            .hash_table
            .find_mut(Self::internal_hash(&*value), Self::internal_eq(&*value))
        {
            Some(old) => {
                *old = value;
                false
            }
            None => {
                self.hash_table.insert_unique(
                    Self::internal_hash(value.as_ref()),
                    value,
                    |boxed| Self::internal_hash(boxed.as_ref()),
                );
                true
            }
        }
    }

    fn remove(&mut self, value: &dyn PartialReflect) -> bool {
        self.hash_table
            .find_entry(Self::internal_hash(value), Self::internal_eq(value))
            .map(HashTableOccupiedEntry::remove)
            .is_ok()
    }

    fn contains(&self, value: &dyn PartialReflect) -> bool {
        self.hash_table
            .find(Self::internal_hash(value), Self::internal_eq(value))
            .is_some()
    }
}

impl PartialReflect for DynamicSet {
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

    #[inline]
    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Err(self)
    }

    #[inline]
    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        None
    }

    #[inline]
    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        None
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

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Set(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Set(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Set(self)
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        set_partial_eq(self, value)
    }

    fn debug(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        self.debug(f)
    }
}

impl FromIterator<Box<dyn PartialReflect>> for DynamicSet {
    fn from_iter<I: IntoIterator<Item = Box<dyn PartialReflect>>>(values: I) -> Self {
        let mut this = Self {
            represented_type: None,
            hash_table: HashTable::new(),
        };

        for value in values {
            this.insert_boxed(value);
        }

        this
    }
}

impl<T: Reflect> FromIterator<T> for DynamicSet {
    fn from_iter<I: IntoIterator<Item = T>>(values: I) -> Self {
        let mut this = Self {
            represented_type: None,
            hash_table: HashTable::new(),
        };

        for value in values {
            this.insert(value);
        }

        this
    }
}

impl IntoIterator for DynamicSet {
    type Item = Box<dyn PartialReflect>;
    type IntoIter = bevy_platform::collections::hash_table::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.hash_table.into_iter()
    }
}

impl<'a> IntoIterator for &'a DynamicSet {
    type Item = &'a dyn PartialReflect;
    type IntoIter = core::iter::Map<
        bevy_platform::collections::hash_table::Iter<'a, Box<dyn PartialReflect>>,
        fn(&'a Box<dyn PartialReflect>) -> Self::Item,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.hash_table.iter().map(|v| v.as_ref())
    }
}

/// Compares a [`Set`] with a [`PartialReflect`] value.
///
/// Returns true if and only if all of the following are true:
/// - `b` is a set;
/// - `b` is the same length as `a`;
/// - For each value pair in `a`, `b` contains the value too,
///   and [`PartialReflect::reflect_partial_eq`] returns `Some(true)` for the two values.
///
/// Returns [`None`] if the comparison couldn't even be performed.
#[inline]
pub fn set_partial_eq<M: Set>(a: &M, b: &dyn PartialReflect) -> Option<bool> {
    let ReflectRef::Set(set) = b.reflect_ref() else {
        return Some(false);
    };

    if a.len() != set.len() {
        return Some(false);
    }

    for value in a.iter() {
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
/// # use std::collections::HashSet;
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
pub fn set_debug(dyn_set: &dyn Set, f: &mut Formatter<'_>) -> core::fmt::Result {
    let mut debug = f.debug_set();
    for value in dyn_set.iter() {
        debug.entry(&value as &dyn Debug);
    }
    debug.finish()
}

/// Applies the elements of reflected set `b` to the corresponding elements of set `a`.
///
/// If a value from `b` does not exist in `a`, the value is cloned and inserted.
/// If a value from `a` does not exist in `b`, the value is removed.
///
/// # Panics
///
/// This function panics if `b` is not a reflected set.
#[inline]
pub fn set_apply<M: Set>(a: &mut M, b: &dyn PartialReflect) {
    if let Err(err) = set_try_apply(a, b) {
        panic!("{err}");
    }
}

/// Tries to apply the elements of reflected set `b` to the corresponding elements of set `a`
/// and returns a Result.
///
/// If a value from `b` does not exist in `a`, the value is cloned and inserted.
/// If a value from `a` does not exist in `b`, the value is removed.
///
/// # Errors
///
/// This function returns an [`ApplyError::MismatchedKinds`] if `b` is not a reflected set or if
/// applying elements to each other fails.
#[inline]
pub fn set_try_apply<S: Set>(a: &mut S, b: &dyn PartialReflect) -> Result<(), ApplyError> {
    let set_value = b.reflect_ref().as_set()?;

    for b_value in set_value.iter() {
        if a.get(b_value).is_none() {
            a.insert_boxed(b_value.to_dynamic());
        }
    }
    a.retain(&mut |value| set_value.get(value).is_some());

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{PartialReflect, Set};

    use super::DynamicSet;
    use alloc::string::{String, ToString};

    #[test]
    fn test_into_iter() {
        let expected = ["foo", "bar", "baz"];

        let mut set = DynamicSet::default();
        set.insert(expected[0].to_string());
        set.insert(expected[1].to_string());
        set.insert(expected[2].to_string());

        for item in set.into_iter() {
            let value = item
                .try_take::<String>()
                .expect("couldn't downcast to String");
            let index = expected
                .iter()
                .position(|i| *i == value.as_str())
                .expect("Element found in expected array");
            assert_eq!(expected[index], value);
        }
    }

    #[test]
    fn apply() {
        let mut map_a = DynamicSet::default();
        map_a.insert(0);
        map_a.insert(1);

        let mut map_b = DynamicSet::default();
        map_b.insert(1);
        map_b.insert(2);

        map_a.apply(&map_b);

        assert!(map_a.get(&0).is_none());
        assert_eq!(map_a.get(&1).unwrap().try_downcast_ref(), Some(&1));
        assert_eq!(map_a.get(&2).unwrap().try_downcast_ref(), Some(&2));
    }
}
