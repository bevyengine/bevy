//! Helpers for working with Bevy reflection.

use crate::TypeInfo;
use alloc::boxed::Box;
use bevy_platform::{
    hash::{DefaultHasher, FixedHasher, NoOpHash},
    sync::{OnceLock, PoisonError, RwLock},
};
use bevy_utils::TypeIdMap;
use core::{
    any::{Any, TypeId},
    hash::BuildHasher,
};

/// A type that can be stored in a ([`Non`])[`GenericTypeCell`].
///
/// [`Non`]: NonGenericTypeCell
pub trait TypedProperty: sealed::Sealed {
    /// The type of the value stored in [`GenericTypeCell`].
    type Stored: 'static;
}

/// Used to store a [`String`] in a [`GenericTypePathCell`] as part of a [`TypePath`] implementation.
///
/// [`TypePath`]: crate::TypePath
/// [`String`]: alloc::string::String
pub struct TypePathComponent;

mod sealed {
    use super::{TypeInfo, TypePathComponent, TypedProperty};
    use alloc::string::String;

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
/// # use core::any::Any;
/// # use bevy_reflect::{DynamicTypePath, NamedField, PartialReflect, Reflect, ReflectMut, ReflectOwned, ReflectRef, StructInfo, Typed, TypeInfo, TypePath, ApplyError};
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
///             let info = StructInfo::new::<Self>(&fields);
///             TypeInfo::Struct(info)
///         })
///     }
/// }
/// # impl TypePath for Foo {
/// #     fn type_path() -> &'static str { todo!() }
/// #     fn short_type_path() -> &'static str { todo!() }
/// # }
/// # impl PartialReflect for Foo {
/// #     fn get_represented_type_info(&self) -> Option<&'static TypeInfo> { todo!() }
/// #     fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> { todo!() }
/// #     fn as_partial_reflect(&self) -> &dyn PartialReflect { todo!() }
/// #     fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect { todo!() }
/// #     fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> { todo!() }
/// #     fn try_as_reflect(&self) -> Option<&dyn Reflect> { todo!() }
/// #     fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> { todo!() }
/// #     fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> { todo!() }
/// #     fn reflect_ref(&self) -> ReflectRef<'_> { todo!() }
/// #     fn reflect_mut(&mut self) -> ReflectMut<'_> { todo!() }
/// #     fn reflect_owned(self: Box<Self>) -> ReflectOwned { todo!() }
/// # }
/// # impl Reflect for Foo {
/// #     fn into_any(self: Box<Self>) -> Box<dyn Any> { todo!() }
/// #     fn as_any(&self) -> &dyn Any { todo!() }
/// #     fn as_any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #     fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> { todo!() }
/// #     fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #     fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #     fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// # }
/// ```
///
/// [`TypePath`]: crate::TypePath
pub struct NonGenericTypeCell<T: TypedProperty>(OnceLock<T::Stored>);

/// See [`NonGenericTypeCell`].
pub type NonGenericTypeInfoCell = NonGenericTypeCell<TypeInfo>;

impl<T: TypedProperty> NonGenericTypeCell<T> {
    /// Initialize a [`NonGenericTypeCell`] for non-generic types.
    pub const fn new() -> Self {
        Self(OnceLock::new())
    }

    /// Returns a reference to the [`TypedProperty`] stored in the cell.
    ///
    /// If there is no entry found, a new one will be generated from the given function.
    pub fn get_or_set<F>(&self, f: F) -> &T::Stored
    where
        F: FnOnce() -> T::Stored,
    {
        self.0.get_or_init(f)
    }
}

impl<T: TypedProperty> Default for NonGenericTypeCell<T> {
    fn default() -> Self {
        Self::new()
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
/// # use core::any::Any;
/// # use bevy_reflect::{DynamicTypePath, PartialReflect, Reflect, ReflectMut, ReflectOwned, ReflectRef, TupleStructInfo, Typed, TypeInfo, TypePath, UnnamedField, ApplyError, Generics, TypeParamInfo};
/// use bevy_reflect::utility::GenericTypeInfoCell;
///
/// struct Foo<T>(T);
///
/// impl<T: Reflect + Typed + TypePath> Typed for Foo<T> {
///     fn type_info() -> &'static TypeInfo {
///         static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
///         CELL.get_or_insert::<Self, _>(|| {
///             let fields = [UnnamedField::new::<T>(0)];
///             let info = TupleStructInfo::new::<Self>(&fields)
///                 .with_generics(Generics::from_iter([TypeParamInfo::new::<T>("T")]));
///             TypeInfo::TupleStruct(info)
///         })
///     }
/// }
/// # impl<T: TypePath> TypePath for Foo<T> {
/// #     fn type_path() -> &'static str { todo!() }
/// #     fn short_type_path() -> &'static str { todo!() }
/// # }
/// # impl<T: PartialReflect + TypePath> PartialReflect for Foo<T> {
/// #     fn get_represented_type_info(&self) -> Option<&'static TypeInfo> { todo!() }
/// #     fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> { todo!() }
/// #     fn as_partial_reflect(&self) -> &dyn PartialReflect { todo!() }
/// #     fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect { todo!() }
/// #     fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> { todo!() }
/// #     fn try_as_reflect(&self) -> Option<&dyn Reflect> { todo!() }
/// #     fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> { todo!() }
/// #     fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> { todo!() }
/// #     fn reflect_ref(&self) -> ReflectRef<'_> { todo!() }
/// #     fn reflect_mut(&mut self) -> ReflectMut<'_> { todo!() }
/// #     fn reflect_owned(self: Box<Self>) -> ReflectOwned { todo!() }
/// # }
/// # impl<T: Reflect + Typed + TypePath> Reflect for Foo<T> {
/// #     fn into_any(self: Box<Self>) -> Box<dyn Any> { todo!() }
/// #     fn as_any(&self) -> &dyn Any { todo!() }
/// #     fn as_any_mut(&mut self) -> &mut dyn Any { todo!() }
/// #     fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> { todo!() }
/// #     fn as_reflect(&self) -> &dyn Reflect { todo!() }
/// #     fn as_reflect_mut(&mut self) -> &mut dyn Reflect { todo!() }
/// #     fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> { todo!() }
/// # }
/// ```
///
///  Implementing [`TypePath`] with generics.
///
/// ```
/// # use core::any::Any;
/// # use bevy_reflect::TypePath;
/// use bevy_reflect::utility::GenericTypePathCell;
///
/// struct Foo<T>(T);
///
/// impl<T: TypePath> TypePath for Foo<T> {
///     fn type_path() -> &'static str {
///         static CELL: GenericTypePathCell = GenericTypePathCell::new();
///         CELL.get_or_insert::<Self, _>(|| format!("my_crate::foo::Foo<{}>", T::type_path()))
///     }
///
///     fn short_type_path() -> &'static str {
///         static CELL: GenericTypePathCell = GenericTypePathCell::new();
///         CELL.get_or_insert::<Self, _>(|| format!("Foo<{}>", T::short_type_path()))
///     }
///
///     fn type_ident() -> Option<&'static str> {
///         Some("Foo")
///     }
///
///     fn module_path() -> Option<&'static str> {
///         Some("my_crate::foo")
///     }
///
///     fn crate_name() -> Option<&'static str> {
///         Some("my_crate")
///     }
/// }
/// ```
/// [`impl_type_path`]: crate::impl_type_path
/// [`TypePath`]: crate::TypePath
pub struct GenericTypeCell<T: TypedProperty>(RwLock<TypeIdMap<&'static T::Stored>>);

/// See [`GenericTypeCell`].
pub type GenericTypeInfoCell = GenericTypeCell<TypeInfo>;
/// See [`GenericTypeCell`].
pub type GenericTypePathCell = GenericTypeCell<TypePathComponent>;

impl<T: TypedProperty> GenericTypeCell<T> {
    /// Initialize a [`GenericTypeCell`] for generic types.
    pub const fn new() -> Self {
        Self(RwLock::new(TypeIdMap::with_hasher(NoOpHash)))
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
        self.get_or_insert_by_type_id(TypeId::of::<G>(), f)
    }

    /// Returns a reference to the [`TypedProperty`] stored in the cell, if any.
    ///
    /// This method will then return the correct [`TypedProperty`] reference for the given type `T`.
    fn get_by_type_id(&self, type_id: TypeId) -> Option<&T::Stored> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&type_id)
            .copied()
    }

    /// Returns a reference to the [`TypedProperty`] stored in the cell.
    ///
    /// This method will then return the correct [`TypedProperty`] reference for the given type `T`.
    /// If there is no entry found, a new one will be generated from the given function.
    fn get_or_insert_by_type_id<F>(&self, type_id: TypeId, f: F) -> &T::Stored
    where
        F: FnOnce() -> T::Stored,
    {
        match self.get_by_type_id(type_id) {
            Some(info) => info,
            None => self.insert_by_type_id(type_id, f()),
        }
    }

    fn insert_by_type_id(&self, type_id: TypeId, value: T::Stored) -> &T::Stored {
        let mut write_lock = self.0.write().unwrap_or_else(PoisonError::into_inner);

        write_lock
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

impl<T: TypedProperty> Default for GenericTypeCell<T> {
    fn default() -> Self {
        Self::new()
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
pub fn reflect_hasher() -> DefaultHasher {
    FixedHasher.build_hasher()
}
