use crate::{serde::Serializable, List, Map, Struct, Tuple, TupleStruct};
use std::{any::Any, fmt::Debug};

pub use bevy_utils::AHasher as ReflectHasher;

/// An immutable enumeration of "kinds" of reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A `ReflectRef` is obtained via [`Reflect::reflect_ref`].
pub enum ReflectRef<'a> {
    Struct(&'a dyn Struct),
    TupleStruct(&'a dyn TupleStruct),
    Tuple(&'a dyn Tuple),
    List(&'a dyn List),
    Map(&'a dyn Map),
    Value(&'a dyn Reflect),
}

/// A mutable enumeration of "kinds" of reflected type.
///
/// Each variant contains a trait object with methods specific to a kind of
/// type.
///
/// A `ReflectMut` is obtained via [`Reflect::reflect_mut`].
pub enum ReflectMut<'a> {
    Struct(&'a mut dyn Struct),
    TupleStruct(&'a mut dyn TupleStruct),
    Tuple(&'a mut dyn Tuple),
    List(&'a mut dyn List),
    Map(&'a mut dyn Map),
    Value(&'a mut dyn Reflect),
}

/// A reflected Rust type.
///
/// Methods for working with particular kinds of Rust type are available using the [`List`], [`Map`],
/// [`Struct`], [`TupleStruct`], and [`Tuple`] subtraits.
///
/// When using `#[derive(Reflect)]` with a struct or tuple struct, the suitable subtrait for that
/// type (`Struct` or `TupleStruct`) is derived automatically.
///
/// # Safety
/// Implementors _must_ ensure that [`Reflect::any`] and [`Reflect::any_mut`] both return the `self`
/// value passed in. If this is not done, [`Reflect::downcast`](trait.Reflect.html#method.downcast)
/// will be UB (and also just logically broken).
pub unsafe trait Reflect: Any + Send + Sync {
    /// Returns the [type name] of the underlying type.
    ///
    /// [type name]: std::any::type_name
    fn type_name(&self) -> &str;

    /// Returns the value as a [`&dyn Any`][std::any::Any].
    fn any(&self) -> &dyn Any;

    /// Returns the value as a [`&mut dyn Any`][std::any::Any].
    fn any_mut(&mut self) -> &mut dyn Any;

    /// Applies a reflected value to this value.
    ///
    /// If a type implements a subtrait of `Reflect`, then the semantics of this
    /// method are as follows:
    /// - If `T` is a [`Struct`], then the value of each named field of `value` is
    ///   applied to the corresponding named field of `self`. Fields which are
    ///   not present in both structs are ignored.
    /// - If `T` is a [`TupleStruct`] or [`Tuple`], then the value of each
    ///   numbered field is applied to the corresponding numbered field of
    ///   `self.` Fields which are not present in both values are ignored.
    /// - If `T` is a [`List`], then each element of `value` is applied to the
    ///   corresponding element of `self`. Up to `self.len()` items are applied,
    ///   and excess elements in `value` are appended to `self`.
    /// - If `T` is a [`Map`], then for each key in `value`, the associated
    ///   value is applied to the value associated with the same key in `self`.
    ///   Keys which are not present in both maps are ignored.
    /// - If `T` is none of these, then `value` is downcast to `T`, cloned, and
    ///   assigned to `self`.
    ///
    /// Note that `Reflect` must be implemented manually for [`List`]s and
    /// [`Map`]s in order to achieve the correct semantics, as derived
    /// implementations will have the semantics for [`Struct`], [`TupleStruct`]
    /// or none of the above depending on the kind of type. For lists, use the
    /// [`list_apply`] helper function when implementing this method.
    ///
    /// [`list_apply`]: crate::list_apply
    ///
    /// # Panics
    ///
    /// Derived implementations of this method will panic:
    /// - If the type of `value` is not of the same kind as `T` (e.g. if `T` is
    ///   a `List`, while `value` is a `Struct`).
    /// - If `T` is any complex type and the corresponding fields or elements of
    ///   `self` and `value` are not of the same type.
    /// - If `T` is a value type and `self` cannot be downcast to `T`
    fn apply(&mut self, value: &dyn Reflect);

    /// Performs a type-checked assignment of a reflected value to this value.
    ///
    /// If `value` does not contain a value of type `T`, returns an `Err`
    /// containing the trait object.
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>;

    /// Returns an enumeration of "kinds" of type.
    ///
    /// See [`ReflectRef`].
    fn reflect_ref(&self) -> ReflectRef;

    /// Returns a mutable enumeration of "kinds" of type.
    ///
    /// See [`ReflectMut`].
    fn reflect_mut(&mut self) -> ReflectMut;

    /// Clones the value as a `Reflect` trait object.
    ///
    /// When deriving `Reflect` for a struct or struct tuple, the value is
    /// cloned via [`Struct::clone_dynamic`] (resp.
    /// [`TupleStruct::clone_dynamic`]). Implementors of other `Reflect`
    /// subtraits (e.g. [`List`], [`Map`]) should use those subtraits'
    /// respective `clone_dynamic` methods.
    fn clone_value(&self) -> Box<dyn Reflect>;

    /// Returns a hash of the value (which includes the type).
    ///
    /// If the underlying type does not support hashing, returns `None`.
    fn reflect_hash(&self) -> Option<u64>;

    /// Returns a "partial equality" comparison result.
    ///
    /// If the underlying type does not support equality testing, returns `None`.
    fn reflect_partial_eq(&self, _value: &dyn Reflect) -> Option<bool>;

    /// Returns a serializable version of the value.
    ///
    /// If the underlying type does not support serialization, returns `None`.
    fn serializable(&self) -> Option<Serializable>;
}

/// A trait for types which can be constructed from a reflected type.
///
/// This trait can be derived on types which implement [`Reflect`]. Some complex
/// types (such as `Vec<T>`) may only be reflected if their element types
/// implement this trait.
///
/// For structs and tuple structs, fields marked with the `#[reflect(ignore)]`
/// attribute will be constructed using the `Default` implementation of the
/// field type, rather than the corresponding field value (if any) of the
/// reflected value.
pub trait FromReflect: Reflect + Sized {
    /// Constructs a concrete instance of `Self` from a reflected value.
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self>;
}

impl Debug for dyn Reflect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Reflect({})", self.type_name())
    }
}

impl dyn Reflect {
    /// Downcasts the value to type `T`, consuming the trait object.
    ///
    /// If the underlying value is not of type `T`, returns `Err(self)`.
    pub fn downcast<T: Reflect>(self: Box<dyn Reflect>) -> Result<Box<T>, Box<dyn Reflect>> {
        // SAFE?: Same approach used by std::any::Box::downcast. ReflectValue is always Any and type
        // has been checked.
        if self.is::<T>() {
            unsafe {
                let raw: *mut dyn Reflect = Box::into_raw(self);
                Ok(Box::from_raw(raw as *mut T))
            }
        } else {
            Err(self)
        }
    }

    /// Downcasts the value to type `T`, unboxing and consuming the trait object.
    ///
    /// If the underlying value is not of type `T`, returns `Err(self)`.
    pub fn take<T: Reflect>(self: Box<dyn Reflect>) -> Result<T, Box<dyn Reflect>> {
        self.downcast::<T>().map(|value| *value)
    }

    /// Returns `true` if the underlying value is of type `T`, or `false`
    /// otherwise.
    #[inline]
    pub fn is<T: Reflect>(&self) -> bool {
        self.any().is::<T>()
    }

    /// Downcasts the value to type `T` by reference.
    ///
    /// If the underlying value is not of type `T`, returns `None`.
    #[inline]
    pub fn downcast_ref<T: Reflect>(&self) -> Option<&T> {
        self.any().downcast_ref::<T>()
    }

    /// Downcasts the value to type `T` by mutable reference.
    ///
    /// If the underlying value is not of type `T`, returns `None`.
    #[inline]
    pub fn downcast_mut<T: Reflect>(&mut self) -> Option<&mut T> {
        self.any_mut().downcast_mut::<T>()
    }
}
