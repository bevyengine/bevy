//! Helpers for working with Bevy reflection.

use crate::TypeInfo;
use bevy_utils::{FixedState, StableHashMap};
use once_cell::race::OnceBox;
use std::{
    any::{Any, TypeId},
    hash::BuildHasher,
    sync::{PoisonError, RwLock},
};

/// A type that can be stored in a ([`Non`])[`GenericTypeCell`].
///
/// [`Non`]: NonGenericTypeCell
pub trait TypedProperty: sealed::Sealed {
    type Stored: 'static;
}

/// Used to store a [`String`] in a [`GenericTypePathCell`] as part of a [`TypePath`] implementation.
///
/// [`TypePath`]: crate::TypePath
pub struct TypePathComponent;

mod sealed {
    use super::{TypeInfo, TypePathComponent, TypedProperty};

    pub trait Sealed {}

    impl Sealed for TypeInfo {}
    impl Sealed for TypePathComponent {}

    impl TypedProperty for TypeInfo {
        type Stored = Self;
    }

    impl TypedProperty for TypePathComponent {
        type Stored = String;
    }
}

/// A container for [`TypeInfo`] over non-generic types, allowing instances to be stored statically.
///
/// This is specifically meant for use with _non_-generic types. If your type _is_ generic,
/// then use [`GenericTypeCell`] instead. Otherwise, it will not take into account all
/// monomorphizations of your type.
///
/// Non-generic [`TypePath`]s should be trivially generated with string literals and [`concat!`].
///
/// ## Example
///
/// ```
/// # use std::any::Any;
/// # use bevy_reflect::{DynamicTypePath, NamedField, Reflect, ReflectMut, ReflectOwned, ReflectRef, StructInfo, Typed, TypeInfo, TypePath};
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
/// #     fn get_represented_type_info(&self) -> Option<&'static TypeInfo> { todo!() }
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

/// # impl TypePath for Foo {
/// #   fn type_path() -> &'static str { todo!() }
/// #   fn short_type_path() -> &'static str { todo!() }
/// # }
/// ```
///
/// [`TypePath`]: crate::TypePath
pub struct NonGenericTypeCell<T: TypedProperty>(OnceBox<T::Stored>);

/// See [`NonGenericTypeCell`].
pub type NonGenericTypeInfoCell = NonGenericTypeCell<TypeInfo>;

impl<T: TypedProperty> NonGenericTypeCell<T> {
    /// Initialize a [`NonGenericTypeCell`] for non-generic types.
    pub const fn new() -> Self {
        Self(OnceBox::new())
    }

    /// Returns a reference to the [`TypedProperty`] stored in the cell.
    ///
    /// If there is no entry found, a new one will be generated from the given function.
    pub fn get_or_set<F>(&self, f: F) -> &T::Stored
    where
        F: FnOnce() -> T::Stored,
    {
        self.0.get_or_init(|| Box::new(f()))
    }
}

/// A container for [`TypedProperty`] over generic types, allowing instances to be stored statically.
///
/// This is specifically meant for use with generic types. If your type isn't generic,
/// then use [`NonGenericTypeCell`] instead as it should be much more performant.
///
/// `#[derive(TypePath)]` and [`impl_type_path`] should always be used over [`GenericTypePathCell`]
/// where possible.
///
/// ## Examples
///
/// Implementing [`TypeInfo`] with generics.
///
/// ```
/// # use std::any::Any;
/// # use bevy_reflect::{DynamicTypePath, Reflect, ReflectMut, ReflectOwned, ReflectRef, TupleStructInfo, Typed, TypeInfo, TypePath, UnnamedField};
/// use bevy_reflect::utility::GenericTypeInfoCell;
///
/// struct Foo<T>(T);
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
/// #     fn get_represented_type_info(&self) -> Option<&'static TypeInfo> { todo!() }
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
/// # impl<T: Reflect> TypePath for Foo<T> {
/// #   fn type_path() -> &'static str { todo!() }
/// #   fn short_type_path() -> &'static str { todo!() }
/// # }
/// ```
///
///  Implementing [`TypePath`] with generics.
///
/// ```
/// # use std::any::Any;
/// # use bevy_reflect::{DynamicTypePath, Reflect, ReflectMut, ReflectOwned, ReflectRef, TypeInfo, TypePath};
/// use bevy_reflect::utility::GenericTypePathCell;
///
/// struct Foo<T>(T);
///
/// impl<T: Reflect + TypePath> TypePath for Foo<T> {
///     fn type_path() -> &'static str {
///         static CELL: GenericTypePathCell = GenericTypePathCell::new();
///         CELL.get_or_insert::<Self, _>(|| format!("my_crate::foo::Foo<{}>", T::type_path()))
///     }
///     
///     fn short_type_path() -> &'static str {
///         static CELL: GenericTypePathCell = GenericTypePathCell::new();
///         CELL.get_or_insert::<Self, _>(|| format!("Foo<{}>", T::short_type_path()))
///     }
/// }
/// #
/// # impl<T: Reflect + TypePath> Reflect for Foo<T> {
/// #     fn type_name(&self) -> &str { todo!() }
/// #     fn get_represented_type_info(&self) -> Option<&'static TypeInfo> { todo!() }
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
/// [`impl_type_path`]: crate::impl_type_path
/// [`TypePath`]: crate::TypePath
pub struct GenericTypeCell<T: TypedProperty>(RwLock<StableHashMap<TypeId, &'static T::Stored>>);

/// See [`GenericTypeCell`].
pub type GenericTypeInfoCell = GenericTypeCell<TypeInfo>;
/// See [`GenericTypeCell`].
pub type GenericTypePathCell = GenericTypeCell<TypePathComponent>;

impl<T: TypedProperty> GenericTypeCell<T> {
    /// Initialize a [`GenericTypeCell`] for generic types.
    pub const fn new() -> Self {
        // Use `bevy_utils::StableHashMap` over `bevy_utils::HashMap`
        // because `BuildHasherDefault` is unfortunately not const.
        Self(RwLock::new(StableHashMap::with_hasher(FixedState)))
    }

    /// Returns a reference to the [`TypedProperty`] stored in the cell.
    ///
    /// This method will then return the correct [`TypedProperty`] reference for the given type `T`.
    /// If there is no entry found, a new one will be generated from the given function.
    pub fn get_or_insert<G, F>(&self, f: F) -> &T::Stored
    where
        G: Any + ?Sized,
        F: FnOnce() -> T::Stored,
    {
        let type_id = TypeId::of::<G>();

        // Put in a separate scope, so `mapping` is dropped before `f`,
        // since `f` might want to call `get_or_insert` recursively
        // and we don't want a deadlock!
        {
            let mapping = self.0.read().unwrap_or_else(PoisonError::into_inner);
            if let Some(info) = mapping.get(&type_id) {
                return info;
            }
        }

        let value = f();

        let mut mapping = self.0.write().unwrap_or_else(PoisonError::into_inner);
        mapping
            .entry(type_id)
            .insert({
                // We leak here in order to obtain a `&'static` reference.
                // Otherwise, we won't be able to return a reference due to the `RwLock`.
                // This should be okay, though, since we expect it to remain statically
                // available over the course of the application.
                Box::leak(Box::new(value))
            })
            .get()
    }
}

/// Deterministic fixed state hasher to be used by implementors of [`Reflect::reflect_hash`].
///
/// Hashes should be deterministic across processes so hashes can be used as
/// checksums for saved scenes, rollback snapshots etc. This function returns
/// such a hasher.
///
/// [`Reflect::reflect_hash`]: crate::Reflect
#[inline]
pub fn reflect_hasher() -> bevy_utils::AHasher {
    FixedState.build_hasher()
}
