//! Helpers for working with Bevy reflection.

use crate::TypeInfo;
use bevy_utils::HashMap;
use once_cell::race::OnceBox;
use parking_lot::RwLock;
use std::any::{Any, TypeId};

mod sealed {
    use super::TypeInfo;

    pub trait Sealed {}

    impl Sealed for TypeInfo {}
    impl Sealed for String {}
}

pub trait TypedProperty: sealed::Sealed + 'static {}
impl<T: sealed::Sealed + 'static> TypedProperty for T {}

/// A container for [`TypeInfo`] or [`TypePathStorage`] over non-generic types, allowing instances to be stored statically.
///
/// This is specifically meant for use with _non_-generic types. If your type _is_ generic,
/// then use [`GenericTypedCell`] instead. Otherwise, it will not take into account all
/// monomorphizations of your type.
///
/// ## Example
///
/// ```
/// # use std::any::Any;
/// # use bevy_reflect::{NamedField, Reflect, ReflectMut, ReflectOwned, ReflectRef, StructInfo, Typed, TypeInfo};
/// use bevy_reflect::utility::NonGenericTypeInfoCell;
///
/// struct Foo {
///     bar: i32
/// }
///
/// impl Typed for Foo {
///     fn type_info() -> &'static TypeInfo {
///         static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
///         CELL.get_or_set(|| {
///             let fields = [NamedField::new::<i32>("bar")];
///             let info = StructInfo::new::<Self>("Foo", &fields);
///             TypeInfo::Struct(info)
///         })
///     }
/// }
/// #
/// # impl Reflect for Foo {
/// #     fn type_name(&self) -> &str { todo!() }
/// #     fn get_type_info(&self) -> &'static TypeInfo { todo!() }
/// #     fn into_any(self: Box<Self>) -> Box<dyn Any> { todo!() }
/// #     fn as_any(&self) -> &dyn Any { todo!() }
/// #     fn as_any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #     fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> { todo!() }
/// #     fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #     fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #     fn apply(&mut self, value: &dyn Reflect) { todo!() }
/// #     fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// #     fn reflect_ref(&self) -> ReflectRef { todo!() }
/// #     fn reflect_mut(&mut self) -> ReflectMut { todo!() }
/// #     fn reflect_owned(self: Box<Self>) -> ReflectOwned { todo!() }
/// #     fn clone_value(&self) -> Box<dyn Reflect> { todo!() }
/// # }
/// ```
pub struct NonGenericTypedCell<T: TypedProperty>(OnceBox<T>);

pub type NonGenericTypeInfoCell = NonGenericTypedCell<TypeInfo>;

impl<T: TypedProperty> NonGenericTypedCell<T> {
    /// Initialize a [`NonGenericTypedCell`] for non-generic types.
    pub const fn new() -> Self {
        Self(OnceBox::new())
    }

    /// Returns a reference to the [`TypeInfo`]/[`TypePathStorage`] stored in the cell.
    ///
    /// If there is no entry found, a new one will be generated from the given function.
    pub fn get_or_set<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        self.0.get_or_init(|| Box::new(f()))
    }
}

/// A container for [`TypeInfo`] or [`TypePathStorage`] over generic types, allowing instances to be stored statically.
///
/// This is specifically meant for use with generic types. If your type isn't generic,
/// then use [`NonGenericTypedCell`] instead as it should be much more performant.
///
/// ## Example
///
/// ```
/// # use std::any::Any;
/// # use bevy_reflect::{Reflect, ReflectMut, ReflectOwned, ReflectRef, TupleStructInfo, Typed, TypeInfo, UnnamedField};
/// use bevy_reflect::utility::GenericTypeInfoCell;
///
/// struct Foo<T: Reflect>(T);
///
/// impl<T: Reflect> Typed for Foo<T> {
///     fn type_info() -> &'static TypeInfo {
///         static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
///         CELL.get_or_insert::<Self, _>(|| {
///             let fields = [UnnamedField::new::<T>(0)];
///             let info = TupleStructInfo::new::<Self>("Foo", &fields);
///             TypeInfo::TupleStruct(info)
///         })
///     }
/// }
/// #
/// # impl<T: Reflect> Reflect for Foo<T> {
/// #     fn type_name(&self) -> &str { todo!() }
/// #     fn get_type_info(&self) -> &'static TypeInfo { todo!() }
/// #     fn into_any(self: Box<Self>) -> Box<dyn Any> { todo!() }
/// #     fn as_any(&self) -> &dyn Any { todo!() }
/// #     fn as_any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #     fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> { todo!() }
/// #     fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #     fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #     fn apply(&mut self, value: &dyn Reflect) { todo!() }
/// #     fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// #     fn reflect_ref(&self) -> ReflectRef { todo!() }
/// #     fn reflect_mut(&mut self) -> ReflectMut { todo!() }
/// #     fn reflect_owned(self: Box<Self>) -> ReflectOwned { todo!() }
/// #     fn clone_value(&self) -> Box<dyn Reflect> { todo!() }
/// # }
/// ```
pub struct GenericTypedCell<T: TypedProperty>(OnceBox<RwLock<HashMap<TypeId, &'static T>>>);

pub type GenericTypeInfoCell = GenericTypedCell<TypeInfo>;
pub type GenericTypePathCell = GenericTypedCell<String>;

impl<T: TypedProperty> GenericTypedCell<T> {
    /// Initialize a [`GenericTypedCell`] for generic types.
    pub const fn new() -> Self {
        Self(OnceBox::new())
    }

    /// Returns a reference to the [`TypeInfo`]/[`TypePathStorage`] stored in the cell.
    ///
    /// This method will then return the correct [`TypeInfo`]/[`TypePathStorage`] reference for the given type `T`.
    /// If there is no entry found, a new one will be generated from the given function.
    pub fn get_or_insert<G, F>(&self, f: F) -> &T
    where
        G: Any + ?Sized,
        F: FnOnce() -> T,
    {
        let type_id = TypeId::of::<G>();
        // let mapping = self.0.get_or_init(|| Box::new(RwLock::default()));
        let mapping = self.0.get_or_init(Box::default);
        if let Some(info) = mapping.read().get(&type_id) {
            return info;
        }

        mapping.write().entry(type_id).or_insert_with(|| {
            // We leak here in order to obtain a `&'static` reference.
            // Otherwise, we won't be able to return a reference due to the `RwLock`.
            // This should be okay, though, since we expect it to remain statically
            // available over the course of the application.
            Box::leak(Box::new(f()))
        })
    }
}
