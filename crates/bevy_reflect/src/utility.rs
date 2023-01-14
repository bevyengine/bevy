//! Helpers for working with Bevy reflection.

use crate::TypeInfo;
use bevy_utils::HashMap;
use once_cell::race::OnceBox;
use parking_lot::RwLock;
use std::any::{Any, TypeId};

mod sealed {
    use crate::{utility::TypePathStorage, TypeInfo};

    trait Sealed {}
    impl Sealed for TypeInfo {}
    impl Sealed for TypePathStorage {}

    pub trait TypedProperty: 'static {}
    impl<T: Sealed + 'static> TypedProperty for T {}
}
pub use sealed::TypedProperty;

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
pub type NonGenericTypePathCell = NonGenericTypedCell<TypePathStorage>;

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
pub type GenericTypePathCell = GenericTypedCell<TypePathStorage>;

impl<T: TypedProperty> GenericTypedCell<T> {
    /// Initialize a [`GenericTypedCell`] for generic types.
    pub const fn new() -> Self {
        Self(OnceBox::new())
    }

    /// Returns a reference to the [`TypeInfo`]/[`TypePath`] stored in the cell.
    ///
    /// This method will then return the correct [`TypeInfo`]/[`TypePath`] reference for the given type `T`.
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

pub struct TypePathStorage {
    path: String,
    short_path: String,
    ident: Option<String>,
    crate_name: Option<String>,
    module_path: Option<String>,
}

impl TypePathStorage {
    pub fn new_primitive<A: AsRef<str>>(name: A) -> Self {
        Self {
            path: name.as_ref().to_owned(),
            short_path: name.as_ref().to_owned(),
            ident: Some(name.as_ref().to_owned()),
            crate_name: None,
            module_path: None,
        }
    }

    pub fn new_anonymous<A: AsRef<str>, B: AsRef<str>>(path: A, short_path: B) -> Self {
        Self {
            path: path.as_ref().to_owned(),
            short_path: short_path.as_ref().to_owned(),
            ident: None,
            crate_name: None,
            module_path: None,
        }
    }

    pub fn new_named<A, B, C, D, E>(
        path: A,
        short_path: B,
        ident: C,
        crate_name: D,
        module_path: E,
    ) -> Self
    where
        A: AsRef<str>,
        B: AsRef<str>,
        C: AsRef<str>,
        D: AsRef<str>,
        E: AsRef<str>,
    {
        Self {
            path: path.as_ref().to_owned(),
            short_path: short_path.as_ref().to_owned(),
            ident: Some(ident.as_ref().to_owned()),
            crate_name: Some(crate_name.as_ref().to_owned()),
            module_path: Some(module_path.as_ref().to_owned()),
        }
    }

    #[inline]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[inline]
    pub fn short_path(&self) -> &str {
        &self.short_path
    }

    #[inline]
    pub fn ident(&self) -> Option<&str> {
        self.ident.as_deref()
    }

    #[inline]
    pub fn crate_name(&self) -> Option<&str> {
        self.crate_name.as_deref()
    }

    #[inline]
    pub fn module_path(&self) -> Option<&str> {
        self.module_path.as_deref()
    }
}

pub mod __private {
    #[macro_export]
    macro_rules! void_tokens {
        ($($tokens: tt)*) => {};
    }

    #[macro_export]
    macro_rules! first_present {
        ({ $($first:tt)* } $( { $($rest:tt)* } )*) => {
            $($first)*
        }
    }

    pub use first_present;
    pub use void_tokens;
}

#[macro_export]
macro_rules! impl_type_path_stored {
    ($storage_fn: expr, impl$({ $($param:tt)* })? for $($impl_tt: tt)+) => {
        const _: () = {
            trait GetStorage {
                fn get_storage() -> &'static $crate::utility::TypePathStorage;
            }

            impl$(< $($param)* >)? GetStorage for $($impl_tt)+ {
                #[inline]
                fn get_storage() -> &'static $crate::utility::TypePathStorage {
                    $crate::utility::__private::first_present!(
                        $({
                        $crate::utility::__private::void_tokens!($($param)*);
                            static CELL: $crate::utility::GenericTypePathCell = $crate::utility::GenericTypePathCell::new();
                            return CELL.get_or_insert::<Self, _>($storage_fn);
                        })?
                        {
                            static CELL: $crate::utility::NonGenericTypePathCell = $crate::utility::NonGenericTypePathCell::new();
                            return CELL.get_or_set($storage_fn);
                        }
                    );
                }
            }

            impl $(< $($param)* >)? $crate::TypePath for $($impl_tt)+ {
                fn type_path() -> &'static str {
                    &Self::get_storage().path()
                }

                fn short_type_path() -> &'static str {
                    &Self::get_storage().short_path()
                }

                fn type_ident() -> Option<&'static str> {
                    match Self::get_storage().ident() {
                        Some(x) => Some(x),
                        None => None
                    }
                }

                fn crate_name() -> Option<&'static str> {
                    match Self::get_storage().crate_name() {
                        Some(x) => Some(x),
                        None => None
                    }
                }

                fn module_path() -> Option<&'static str> {
                    match Self::get_storage().module_path() {
                        Some(x) => Some(x),
                        None => None
                    }
                }
            }
        };
    }
}
pub use impl_type_path_stored;
