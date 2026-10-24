//! A trait used to power [set-like] operations via reflection.
//!
//! [set-like]: https://doc.rust-lang.org/stable/std/collections/struct.HashSet.html
use alloc::{boxed::Box, vec::Vec};
use core::fmt::{Debug, Formatter};

use crate::{
    dynamic::{DynamicSet, DynamicValue, DynamicValueRef},
    generics::impl_generic_info_methods,
    ty::impl_type_methods,
    Generics, Reflect, ReflectRef, Type, TypePath,
};

/// A trait used to power [set-like] operations via [reflection].
///
/// Sets contain zero or more entries of a fixed type, and correspond to types
/// like [`HashSet`] and [`BTreeSet`].
/// The order of these entries is not guaranteed by this trait.
///
/// # Hashing and equality
///
/// All values are expected to return a valid hash value from [`Reflect::reflect_hash`] and be
/// comparable using [`Reflect::reflect_partial_eq`].
/// If using the [`#[derive(Reflect)]`](derive@crate::Reflect) macro, this can be done by adding
/// `#[reflect(Hash, PartialEq)]` to the entire struct or enum.
/// The ordering is expected to be total, that is as if the reflected type implements the [`Eq`] trait.
/// This is true even for manual implementors who do not hash or compare values,
/// as it is still relied on by [`DynamicSet`].
///
/// # Example
///
/// ```
/// use bevy_reflect::{Reflect, set::Set};
/// use std::collections::HashSet;
///
///
/// let foo: &mut dyn Set = &mut HashSet::<u32>::new();
/// foo.insert_boxed(Box::new(123_u32));
/// assert_eq!(foo.len(), 1);
///
/// let field: &dyn Reflect = foo.get(&123_u32).unwrap();
/// assert_eq!(field.try_downcast_ref::<u32>(), Some(&123_u32));
/// ```
///
/// [`HashSet`]: std::collections::HashSet
/// [`BTreeSet`]: alloc::collections::BTreeSet
/// [set-like]: https://doc.rust-lang.org/stable/std/collections/struct.HashSet.html
/// [reflection]: crate
// Prevents unexpectedly importing this trait when trying to call, for example, `HashSet::iter`
#[rust_analyzer::completions(ignore_flyimport_methods)]
pub trait Set: Reflect {
    /// Constructs a value for this set from a dynamic value representation.
    fn value_from_dynamic(&self, dynamic: DynamicValue) -> Option<Box<dyn Reflect>>;

    /// Returns a reference to the value.
    ///
    /// If no value is contained, returns `None`.
    fn get(&self, value: &dyn Reflect) -> Option<&dyn Reflect>;

    /// Returns the number of elements in the set.
    fn len(&self) -> usize;

    /// Returns `true` if the list contains no elements.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the values of the set.
    fn iter(&self) -> Box<dyn Iterator<Item = &dyn Reflect> + '_>;

    /// Drain the values of this set to get a vector of owned values.
    ///
    /// After calling this function, `self` will be empty.
    fn drain(&mut self) -> Vec<Box<dyn Reflect>>;

    /// Retain only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` for which `f(&e)` returns `false`.
    fn retain(&mut self, f: &mut dyn FnMut(&dyn Reflect) -> bool);

    /// Inserts a value into the set.
    ///
    /// If the set did not have this value present, `true` is returned.
    /// If the set did have this value present, `false` is returned.
    fn insert_boxed(&mut self, value: Box<dyn Reflect>) -> bool;

    /// Removes a value from the set.
    ///
    /// If the set did have this value present, `true` is returned.
    /// If the set did not have this value present, `false` is returned.
    fn remove(&mut self, value: &dyn Reflect) -> bool;

    /// Checks if the given value is contained in the set
    fn contains(&self, value: &dyn Reflect) -> bool;

    /// Returns the [`SetInfo`] for this type.
    fn reflect_set_info(&self) -> &'static SetInfo {
        self.reflect_type_info().as_set().unwrap()
    }
}

/// A container for compile-time set info.
#[derive(Clone, Debug)]
pub struct SetInfo {
    ty: Type,
    generics: Generics,
    value_ty: Type,
    #[cfg(feature = "reflect_documentation")]
    docs: Option<&'static str>,
}

impl SetInfo {
    /// Create a new [`SetInfo`].
    pub fn new<TSet: Set + TypePath, TValue: Reflect + TypePath>() -> Self {
        Self {
            ty: Type::of::<TSet>(),
            generics: Generics::new(),
            value_ty: Type::of::<TValue>(),
            #[cfg(feature = "reflect_documentation")]
            docs: None,
        }
    }

    /// Sets the docstring for this set.
    #[cfg(feature = "reflect_documentation")]
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
    #[cfg(feature = "reflect_documentation")]
    pub fn docs(&self) -> Option<&'static str> {
        self.docs
    }

    impl_generic_info_methods!(generics);
}

pub(crate) trait SetLike {
    fn get(&self, value: DynamicValueRef<'_>) -> Option<DynamicValueRef<'_>>;

    fn len(&self) -> usize;

    fn iter(&self) -> impl Iterator<Item = DynamicValueRef<'_>>;
}

impl SetLike for DynamicSet {
    fn get(&self, value: DynamicValueRef<'_>) -> Option<DynamicValueRef<'_>> {
        self.get(value).map(DynamicValue::as_ref)
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn iter(&self) -> impl Iterator<Item = DynamicValueRef<'_>> {
        self.iter().map(DynamicValue::as_ref)
    }
}

impl SetLike for dyn Set {
    fn get(&self, value: DynamicValueRef<'_>) -> Option<DynamicValueRef<'_>> {
        self.get(value.as_concrete()?)
            .map(|value| DynamicValueRef::from(value))
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn iter(&self) -> impl Iterator<Item = DynamicValueRef<'_>> {
        self.iter().map(|value| DynamicValueRef::from(value))
    }
}

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
pub fn set_partial_eq(a: &dyn Set, b: &dyn Reflect) -> Option<bool> {
    let ReflectRef::Set(b) = b.reflect_ref() else {
        return Some(false);
    };
    set_like_partial_eq(a, b)
}

/// Same as above, but for any two [`SetLike`] types.
pub(crate) fn set_like_partial_eq<L: SetLike + ?Sized, R: SetLike + ?Sized>(
    a: &L,
    b: &R,
) -> Option<bool> {
    if a.len() != b.len() {
        return Some(false);
    }

    for value in a.iter() {
        if let Some(set_value) = b.get(value) {
            let eq_result = value.dynamic_partial_eq(set_value);
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
