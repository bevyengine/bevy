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
    fn apply(&mut self, value: &dyn Reflect);
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>>;

    /// Returns an enumeration of "kinds" of type.
    ///
    /// See [`ReflectRef`].
    fn reflect_ref(&self) -> ReflectRef;

    /// Returns a mutable enumeration of "kinds" of type.
    ///
    /// See [`ReflectMut`].
    fn reflect_mut(&mut self) -> ReflectMut;

    /// Clones the value as a [`Reflect`] trait object.
    fn clone_value(&self) -> Box<dyn Reflect>;

    /// Returns a hash of the value (which includes the type).
    ///
    /// If the underlying type does not support hashing, returns `None`.
    fn reflect_hash(&self) -> Option<u64>;

    /// Returns a "partial equal" comparison result.
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
/// This trait is automatically derived for types which derive [`Reflect`].
///
/// For structs and tuple structs, fields marked with the `#[reflect(ignore)]`
/// attribute will be constructed using the `Default` implementation of the
/// field type, rather than the corresponding field value (if any) of the
/// reflected value.
pub trait FromReflect: Reflect + Sized {
    /// Creates a clone of a reflected value, converting it to a concrete type if it was a dynamic types (e.g. [`DynamicStruct`](crate::DynamicStruct))
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
