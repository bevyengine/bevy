//! Helpers for working with Bevy reflection.

use crate::TypeInfo;
use bevy_utils::HashMap;
use once_cell::race::OnceBox;
use parking_lot::RwLock;
use std::any::{Any, TypeId};

/// A container over non-generic types, allowing instances to be stored statically.
///
/// This is specifically meant for use with _non_-generic types. If your type _is_ generic,
/// then use [`GenericDataCell`] instead. Otherwise, it will not take into account all
/// monomorphizations of your type.
pub struct NonGenericDataCell<Data>(OnceBox<Data>);

impl<Data> NonGenericDataCell<Data> {
    /// Initialize a [`NonGenericDataCell`] for non-generic types.
    pub const fn new() -> Self {
        Self(OnceBox::new())
    }

    /// Returns a reference to the `Data` stored in the cell.
    ///
    /// If there is no `Data` found, a new one will be generated from the given function.
    pub fn get_or_set<F>(&self, f: F) -> &Data
    where
        F: FnOnce() -> Data,
    {
        self.0.get_or_init(|| Box::new(f()))
    }
}

/// A container over generic types, allowing instances to be stored statically.
///
/// This is specifically meant for use with generic types. If your type isn't generic,
/// then use [`NonGenericDataCell`] instead as it should be much more performant.
pub struct GenericDataCell<Data: 'static>(OnceBox<RwLock<HashMap<TypeId, &'static Data>>>);

impl<Data> GenericDataCell<Data> {
    /// Initialize a [`GenericDataCell`] for generic types.
    pub const fn new() -> Self {
        Self(OnceBox::new())
    }

    /// Returns a reference to the `Data` stored in the cell.
    ///
    /// This method will then return the correct `Data` reference for the given type `T`.
    /// If there is no `Data` found, a new one will be generated from the given function.
    pub fn get_or_insert<T, F>(&self, f: F) -> &Data
    where
        T: Any + ?Sized,
        F: FnOnce() -> Data,
    {
        let type_id = TypeId::of::<T>();
        let mapping = self.0.get_or_init(|| Box::new(RwLock::default()));
        if let Some(info) = mapping.read().get(&type_id) {
            return info;
        }

        // We leak here in order to obtain a `&'static` reference.
        // Otherwise, we won't be able to return a reference due to the `RwLock`.
        // This should be okay, though, since we expect it to remain statically
        // available over the course of the application.
        let value = Box::leak(Box::new(f()));

        mapping.write().entry(type_id).or_insert(value)
    }
}

/// A container for [`TypeInfo`] over non-generic types, allowing instances to be stored statically.
///
/// This is specifically meant for use with _non_-generic types. If your type _is_ generic,
/// then use [`GenericTypeInfoCell`] instead. Otherwise, it will not take into account all
/// monomorphizations of your type.
///
/// ## Example
///
/// ```
/// # use std::any::Any;
/// # use bevy_reflect::{NamedField, Reflect, ReflectMut, ReflectRef, StructInfo, Typed, TypeInfo, TypePath};
/// use bevy_reflect::utility::NonGenericTypeInfoCell;
///
/// struct Foo {
///   bar: i32
/// }
///
/// impl Typed for Foo {
///   fn type_info() -> &'static TypeInfo {
///     static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
///     CELL.get_or_set(|| {
///       let fields = [NamedField::new::<i32, _>("bar")];
///       let info = StructInfo::new::<Self>(&fields);
///       TypeInfo::Struct(info)
///     })
///   }
/// }
/// # impl TypePath for Foo {
/// #   fn name() -> &'static str { "Foo" }
/// # }
/// #
/// # impl Reflect for Foo {
/// #   fn type_path(&self) -> &str { todo!() }
/// #   fn get_type_info(&self) -> &'static TypeInfo { todo!() }
/// #   fn into_any(self: Box<Self>) -> Box<dyn Any> { todo!() }
/// #   fn as_any(&self) -> &dyn Any { todo!() }
/// #   fn as_any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #   fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #   fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #   fn apply(&mut self, value: &dyn Reflect) { todo!() }
/// #   fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// #   fn reflect_ref(&self) -> ReflectRef { todo!() }
/// #   fn reflect_mut(&mut self) -> ReflectMut { todo!() }
/// #   fn clone_value(&self) -> Box<dyn Reflect> { todo!() }
/// # }
/// ```
pub type NonGenericTypeInfoCell = NonGenericDataCell<TypeInfo>;

/// A container for [`TypeInfo`] over generic types, allowing instances to be stored statically.
///
/// This is specifically meant for use with generic types. If your type isn't generic,
/// then use [`NonGenericTypeInfoCell`] instead as it should be much more performant.
///
/// ## Example
///
/// ```
/// # use std::any::Any;
/// # use bevy_reflect::{Reflect, ReflectMut, ReflectRef, TupleStructInfo, Typed, TypeInfo, UnnamedField, TypePath};
/// use bevy_reflect::utility::GenericTypeInfoCell;
///
/// struct Foo<T: Reflect>(T);
///
/// impl<T: Reflect + TypePath> Typed for Foo<T> {
///   fn type_info() -> &'static TypeInfo {
///     static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
///     CELL.get_or_insert::<Self, _>(|| {
///       let fields = [UnnamedField::new::<T>(0)];
///       let info = TupleStructInfo::new::<Self>(&fields);
///       TypeInfo::TupleStruct(info)
///     })
///   }
/// }
///
/// # impl<T: Reflect> TypePath for Foo<T> {
/// #   fn name() -> &'static str { todo!() }
/// # }
/// #
/// # impl<T: Reflect> Reflect for Foo<T> {
/// #   fn type_path(&self) -> &str { todo!() }
/// #   fn get_type_info(&self) -> &'static TypeInfo { todo!() }
/// #   fn into_any(self: Box<Self>) -> Box<dyn Any> { todo!() }
/// #   fn as_any(&self) -> &dyn Any { todo!() }
/// #   fn as_any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #   fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #   fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #   fn apply(&mut self, value: &dyn Reflect) { todo!() }
/// #   fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// #   fn reflect_ref(&self) -> ReflectRef { todo!() }
/// #   fn reflect_mut(&mut self) -> ReflectMut { todo!() }
/// #   fn clone_value(&self) -> Box<dyn Reflect> { todo!() }
/// # }
/// ```
pub type GenericTypeInfoCell = GenericDataCell<TypeInfo>;

/// A container for [`String`] over generic types, allowing instances to be stored statically.
///
/// Used when implementing [`TypePath`][crate::TypePath] for generic types to store the type path.
///
/// ## Example
///
/// ```
/// # use bevy_reflect::TypePath;
/// use bevy_reflect::utility::GenericTypeNameCell;
///
/// struct Foo<T>(T);
///
/// impl<T: TypePath> TypePath for Foo<T> {
///     fn name() -> &'static str {
///         static CELL: GenericTypeNameCell = GenericTypeNameCell::new();
///         CELL.get_or_insert::<Self, _>(|| {
///             format!(concat!(module_path!(), "::Foo<{}>"), T::name())
///         })
///     }
/// }
/// ```
pub type GenericTypeNameCell = GenericDataCell<String>;
