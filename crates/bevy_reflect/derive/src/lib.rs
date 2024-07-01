#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! This crate contains macros used by Bevy's `Reflect` API.
//!
//! The main export of this crate is the derive macro for [`Reflect`]. This allows
//! types to easily implement `Reflect` along with other `bevy_reflect` traits,
//! such as `Struct`, `GetTypeRegistration`, and more— all with a single derive!
//!
//! Some other noteworthy exports include the derive macros for [`FromReflect`] and
//! [`TypePath`], as well as the [`reflect_trait`] attribute macro.
//!
//! [`Reflect`]: crate::derive_reflect
//! [`FromReflect`]: crate::derive_from_reflect
//! [`TypePath`]: crate::derive_type_path
//! [`reflect_trait`]: macro@reflect_trait

extern crate proc_macro;

mod container_attributes;
mod custom_attributes;
mod derive_data;
#[cfg(feature = "documentation")]
mod documentation;
mod enum_utility;
mod field_attributes;
mod from_reflect;
mod impls;
mod reflect_value;
mod registration;
mod serialization;
mod trait_reflection;
mod type_path;
mod utility;

use crate::derive_data::{ReflectDerive, ReflectMeta, ReflectStruct};
use container_attributes::ContainerAttributes;
use derive_data::{ReflectImplSource, ReflectProvenance, ReflectTraitToImpl, ReflectTypePath};
use proc_macro::TokenStream;
use quote::quote;
use reflect_value::ReflectValueDef;
use syn::{parse_macro_input, DeriveInput};
use type_path::NamedTypePathDef;

pub(crate) static REFLECT_ATTRIBUTE_NAME: &str = "reflect";
pub(crate) static REFLECT_VALUE_ATTRIBUTE_NAME: &str = "reflect_value";
pub(crate) static TYPE_PATH_ATTRIBUTE_NAME: &str = "type_path";
pub(crate) static TYPE_NAME_ATTRIBUTE_NAME: &str = "type_name";

/// Used both for [`impl_reflect`] and [`derive_reflect`].
///
/// [`impl_reflect`]: macro@impl_reflect
/// [`derive_reflect`]: derive_reflect()
fn match_reflect_impls(ast: DeriveInput, source: ReflectImplSource) -> TokenStream {
    let derive_data = match ReflectDerive::from_input(
        &ast,
        ReflectProvenance {
            source,
            trait_: ReflectTraitToImpl::Reflect,
        },
    ) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    let (reflect_impls, from_reflect_impl) = match derive_data {
        ReflectDerive::Struct(struct_data) | ReflectDerive::UnitStruct(struct_data) => (
            impls::impl_struct(&struct_data),
            if struct_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_struct(&struct_data))
            } else {
                None
            },
        ),
        ReflectDerive::TupleStruct(struct_data) => (
            impls::impl_tuple_struct(&struct_data),
            if struct_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_tuple_struct(&struct_data))
            } else {
                None
            },
        ),
        ReflectDerive::Enum(enum_data) => (
            impls::impl_enum(&enum_data),
            if enum_data.meta().from_reflect().should_auto_derive() {
                Some(from_reflect::impl_enum(&enum_data))
            } else {
                None
            },
        ),
        ReflectDerive::Value(meta) => (
            impls::impl_value(&meta),
            if meta.from_reflect().should_auto_derive() {
                Some(from_reflect::impl_value(&meta))
            } else {
                None
            },
        ),
    };

    TokenStream::from(quote! {
        const _: () = {
            #reflect_impls
            #from_reflect_impl
        };
    })
}

/// The main derive macro used by `bevy_reflect` for deriving its `Reflect` trait.
///
/// This macro can be used on all structs and enums (unions are not supported).
/// It will automatically generate implementations for `Reflect`, `Typed`, `GetTypeRegistration`, and `FromReflect`.
/// And, depending on the item's structure, will either implement `Struct`, `TupleStruct`, or `Enum`.
///
/// See the [`FromReflect`] derive macro for more information on how to customize the `FromReflect` implementation.
///
/// # Container Attributes
///
/// This macro comes with some helper attributes that can be added to the container item
/// in order to provide additional functionality or alter the generated implementations.
///
/// In addition to those listed, this macro can also use the attributes for [`TypePath`] derives.
///
/// ## `#[reflect(Ident)]`
///
/// The `#[reflect(Ident)]` attribute is used to add type data registrations to the `GetTypeRegistration`
/// implementation corresponding to the given identifier, prepended by `Reflect`.
///
/// For example, `#[reflect(Foo, Bar)]` would add two registrations:
/// one for `ReflectFoo` and another for `ReflectBar`.
/// This assumes these types are indeed in-scope wherever this macro is called.
///
/// This is often used with traits that have been marked by the [`#[reflect_trait]`](macro@reflect_trait)
/// macro in order to register the type's implementation of that trait.
///
/// ### Default Registrations
///
/// The following types are automatically registered when deriving `Reflect`:
///
/// * `ReflectFromReflect` (unless opting out of `FromReflect`)
/// * `SerializationData`
/// * `ReflectFromPtr`
///
/// ### Special Identifiers
///
/// There are a few "special" identifiers that work a bit differently:
///
/// * `#[reflect(Debug)]` will force the implementation of `Reflect::reflect_debug` to rely on
///   the type's [`Debug`] implementation.
///   A custom implementation may be provided using `#[reflect(Debug(my_debug_func))]` where
///   `my_debug_func` is the path to a function matching the signature:
///   `(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result`.
/// * `#[reflect(PartialEq)]` will force the implementation of `Reflect::reflect_partial_eq` to rely on
///   the type's [`PartialEq`] implementation.
///   A custom implementation may be provided using `#[reflect(PartialEq(my_partial_eq_func))]` where
///   `my_partial_eq_func` is the path to a function matching the signature:
///   `(&self, value: &dyn #bevy_reflect_path::Reflect) -> bool`.
/// * `#[reflect(Hash)]` will force the implementation of `Reflect::reflect_hash` to rely on
///   the type's [`Hash`] implementation.
///   A custom implementation may be provided using `#[reflect(Hash(my_hash_func))]` where
///   `my_hash_func` is the path to a function matching the signature: `(&self) -> u64`.
/// * `#[reflect(Default)]` will register the `ReflectDefault` type data as normal.
///   However, it will also affect how certain other operations are performed in order
///   to improve performance and/or robustness.
///   An example of where this is used is in the [`FromReflect`] derive macro,
///   where adding this attribute will cause the `FromReflect` implementation to create
///   a base value using its [`Default`] implementation avoiding issues with ignored fields
///   (for structs and tuple structs only).
///
/// ## `#[reflect_value]`
///
/// The `#[reflect_value]` attribute (which may also take the form `#[reflect_value(Ident)]`),
/// denotes that the item should implement `Reflect` as though it were a base value type.
/// This means that it will forgo implementing `Struct`, `TupleStruct`, or `Enum`.
///
/// Furthermore, it requires that the type implements [`Clone`].
/// If planning to serialize this type using the reflection serializers,
/// then the `Serialize` and `Deserialize` traits will need to be implemented and registered as well.
///
/// ## `#[reflect(from_reflect = false)]`
///
/// This attribute will opt-out of the default `FromReflect` implementation.
///
/// This is useful for when a type can't or shouldn't implement `FromReflect`,
/// or if a manual implementation is desired.
///
/// Note that in the latter case, `ReflectFromReflect` will no longer be automatically registered.
///
/// ## `#[reflect(type_path = false)]`
///
/// This attribute will opt-out of the default `TypePath` implementation.
///
/// This is useful for when a type can't or shouldn't implement `TypePath`,
/// or if a manual implementation is desired.
///
/// ## `#[reflect(no_field_bounds)]`
///
/// This attribute will opt-out of the default trait bounds added to all field types
/// for the generated reflection trait impls.
///
/// Normally, all fields will have the bounds `TypePath`, and either `FromReflect` or `Reflect`
/// depending on if `#[reflect(from_reflect = false)]` is used.
/// However, this might not always be desirable, and so this attribute may be used to remove those bounds.
///
/// ### Example
///
/// If a type is recursive the default bounds will cause an overflow error when building:
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// #[derive(Reflect)] // ERROR: overflow evaluating the requirement `Foo: FromReflect`
/// struct Foo {
///   foo: Vec<Foo>,
/// }
///
/// // Generates a where clause like:
/// // impl bevy_reflect::Reflect for Foo
/// // where
/// //   Self: Any + Send + Sync,
/// //   Vec<Foo>: FromReflect + TypePath,
/// ```
///
/// In this case, `Foo` is given the bounds `Vec<Foo>: FromReflect + TypePath`,
/// which requires that `Foo` implements `FromReflect`,
/// which requires that `Vec<Foo>` implements `FromReflect`,
/// and so on, resulting in the error.
///
/// To fix this, we can add `#[reflect(no_field_bounds)]` to `Foo` to remove the bounds on `Vec<Foo>`:
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// #[derive(Reflect)]
/// #[reflect(no_field_bounds)]
/// struct Foo {
///   foo: Vec<Foo>,
/// }
///
/// // Generates a where clause like:
/// // impl bevy_reflect::Reflect for Foo
/// // where
/// //   Self: Any + Send + Sync,
/// ```
///
/// ## `#[reflect(where T: Trait, U::Assoc: Trait, ...)]`
///
/// This attribute can be used to add additional bounds to the generated reflection trait impls.
///
/// This is useful for when a type needs certain bounds only applied to the reflection impls
/// that are not otherwise automatically added by the derive macro.
///
/// ### Example
///
/// In the example below, we want to enforce that `T::Assoc: List` is required in order for
/// `Foo<T>` to be reflectable, but we don't want it to prevent `Foo<T>` from being used
/// in places where `T::Assoc: List` is not required.
///
/// ```ignore
/// trait Trait {
///   type Assoc;
/// }
///
/// #[derive(Reflect)]
/// #[reflect(where T::Assoc: List)]
/// struct Foo<T: Trait> where T::Assoc: Default {
///   value: T::Assoc,
/// }
///
/// // Generates a where clause like:
/// //
/// // impl<T: Trait> bevy_reflect::Reflect for Foo<T>
/// // where
/// //   Self: Any + Send + Sync,
/// //   T::Assoc: Default,
/// //   T: TypePath,
/// //   T::Assoc: FromReflect + TypePath,
/// //   T::Assoc: List,
/// // {/* ... */}
/// ```
///
/// ## `#[reflect(@...)]`
///
/// This attribute can be used to register custom attributes to the type's `TypeInfo`.
///
/// It accepts any expression after the `@` symbol that resolves to a value which implements `Reflect`.
///
/// Any number of custom attributes may be registered, however, each the type of each attribute must be unique.
/// If two attributes of the same type are registered, the last one will overwrite the first.
///
/// ### Example
///
/// ```ignore
/// #[derive(Reflect)]
/// struct Required;
///
/// #[derive(Reflect)]
/// struct EditorTooltip(String);
///
/// impl EditorTooltip {
///   fn new(text: &str) -> Self {
///     Self(text.to_string())
///   }
/// }
///
/// #[derive(Reflect)]
/// // Specify a "required" status and tooltip:
/// #[reflect(@Required, @EditorTooltip::new("An ID is required!"))]
/// struct Id(u8);
/// ```
///
/// # Field Attributes
///
/// Along with the container attributes, this macro comes with some attributes that may be applied
/// to the contained fields themselves.
///
/// ## `#[reflect(ignore)]`
///
/// This attribute simply marks a field to be ignored by the reflection API.
///
/// This allows fields to completely opt-out of reflection,
/// which may be useful for maintaining invariants, keeping certain data private,
/// or allowing the use of types that do not implement `Reflect` within the container.
///
/// ## `#[reflect(skip_serializing)]`
///
/// This works similar to `#[reflect(ignore)]`, but rather than opting out of _all_ of reflection,
/// it simply opts the field out of both serialization and deserialization.
/// This can be useful when a field should be accessible via reflection, but may not make
/// sense in a serialized form, such as computed data.
///
/// What this does is register the `SerializationData` type within the `GetTypeRegistration` implementation,
/// which will be used by the reflection serializers to determine whether or not the field is serializable.
///
/// ## `#[reflect(@...)]`
///
/// This attribute can be used to register custom attributes to the field's `TypeInfo`.
///
/// It accepts any expression after the `@` symbol that resolves to a value which implements `Reflect`.
///
/// Any number of custom attributes may be registered, however, each the type of each attribute must be unique.
/// If two attributes of the same type are registered, the last one will overwrite the first.
///
/// ### Example
///
/// ```ignore
/// #[derive(Reflect)]
/// struct EditorTooltip(String);
///
/// impl EditorTooltip {
///   fn new(text: &str) -> Self {
///     Self(text.to_string())
///   }
/// }
///
/// #[derive(Reflect)]
/// struct Slider {
///   // Specify a custom range and tooltip:
///   #[reflect(@0.0..=1.0, @EditorTooltip::new("Must be between 0 and 1"))]
///   value: f32,
/// }
/// ```
///
/// [`reflect_trait`]: macro@reflect_trait
#[proc_macro_derive(Reflect, attributes(reflect, reflect_value, type_path, type_name))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match_reflect_impls(ast, ReflectImplSource::DeriveLocalType)
}

/// Derives the `FromReflect` trait.
///
/// # Field Attributes
///
/// ## `#[reflect(ignore)]`
///
/// The `#[reflect(ignore)]` attribute is shared with the [`#[derive(Reflect)]`](Reflect) macro and has much of the same
/// functionality in that it denotes that a field will be ignored by the reflection API.
///
/// The only major difference is that using it with this derive requires that the field implements [`Default`].
/// Without this requirement, there would be no way for `FromReflect` to automatically construct missing fields
/// that have been ignored.
///
/// ## `#[reflect(default)]`
///
/// If a field cannot be read, this attribute specifies a default value to be used in its place.
///
/// By default, this attribute denotes that the field's type implements [`Default`].
/// However, it can also take in a path string to a user-defined function that will return the default value.
/// This takes the form: `#[reflect(default = "path::to::my_function")]` where `my_function` is a parameterless
/// function that must return some default value for the type.
///
/// Specifying a custom default can be used to give different fields their own specialized defaults,
/// or to remove the `Default` requirement on fields marked with `#[reflect(ignore)]`.
/// Additionally, either form of this attribute can be used to fill in fields that are simply missing,
/// such as when converting a partially-constructed dynamic type to a concrete one.
#[proc_macro_derive(FromReflect, attributes(reflect))]
pub fn derive_from_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let derive_data = match ReflectDerive::from_input(
        &ast,
        ReflectProvenance {
            source: ReflectImplSource::DeriveLocalType,
            trait_: ReflectTraitToImpl::FromReflect,
        },
    ) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    let from_reflect_impl = match derive_data {
        ReflectDerive::Struct(struct_data) | ReflectDerive::UnitStruct(struct_data) => {
            from_reflect::impl_struct(&struct_data)
        }
        ReflectDerive::TupleStruct(struct_data) => from_reflect::impl_tuple_struct(&struct_data),
        ReflectDerive::Enum(meta) => from_reflect::impl_enum(&meta),
        ReflectDerive::Value(meta) => from_reflect::impl_value(&meta),
    };

    TokenStream::from(quote! {
        const _: () = {
            #from_reflect_impl
        };
    })
}

/// Derives the `TypePath` trait, providing a stable alternative to [`std::any::type_name`].
///
/// # Container Attributes
///
/// ## `#[type_path = "my_crate::foo"]`
///
/// Optionally specifies a custom module path to use instead of [`module_path`].
///
/// This path does not include the final identifier.
///
/// ## `#[type_name = "RenamedType"]`
///
/// Optionally specifies a new terminating identifier for `TypePath`.
///
/// To use this attribute, `#[type_path = "..."]` must also be specified.
#[proc_macro_derive(TypePath, attributes(type_path, type_name))]
pub fn derive_type_path(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let derive_data = match ReflectDerive::from_input(
        &ast,
        ReflectProvenance {
            source: ReflectImplSource::DeriveLocalType,
            trait_: ReflectTraitToImpl::TypePath,
        },
    ) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    let type_path_impl = impls::impl_type_path(derive_data.meta());

    TokenStream::from(quote! {
        const _: () = {
            #type_path_impl
        };
    })
}

/// A macro that automatically generates type data for traits, which their implementors can then register.
///
/// The output of this macro is a struct that takes reflected instances of the implementor's type
/// and returns the value as a trait object.
/// Because of this, **it can only be used on [object-safe] traits.**
///
/// For a trait named `MyTrait`, this will generate the struct `ReflectMyTrait`.
/// The generated struct can be created using `FromType` with any type that implements the trait.
/// The creation and registration of this generated struct as type data can be automatically handled
/// by [`#[derive(Reflect)]`](Reflect).
///
/// # Example
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// # use std::any::TypeId;
/// # use bevy_reflect_derive::{Reflect, reflect_trait};
/// #[reflect_trait] // Generates `ReflectMyTrait`
/// trait MyTrait {
///   fn print(&self) -> &str;
/// }
///
/// #[derive(Reflect)]
/// #[reflect(MyTrait)] // Automatically registers `ReflectMyTrait`
/// struct SomeStruct;
///
/// impl MyTrait for SomeStruct {
///   fn print(&self) -> &str {
///     "Hello, World!"
///   }
/// }
///
/// // We can create the type data manually if we wanted:
/// let my_trait: ReflectMyTrait = FromType::<SomeStruct>::from_type();
///
/// // Or we can simply get it from the registry:
/// let mut registry = TypeRegistry::default();
/// registry.register::<SomeStruct>();
/// let my_trait = registry
///   .get_type_data::<ReflectMyTrait>(TypeId::of::<SomeStruct>())
///   .unwrap();
///
/// // Then use it on reflected data
/// let reflected: Box<dyn Reflect> = Box::new(SomeStruct);
/// let reflected_my_trait: &dyn MyTrait = my_trait.get(&*reflected).unwrap();
/// assert_eq!("Hello, World!", reflected_my_trait.print());
/// ```
///
/// [object-safe]: https://doc.rust-lang.org/reference/items/traits.html#object-safety
#[proc_macro_attribute]
pub fn reflect_trait(args: TokenStream, input: TokenStream) -> TokenStream {
    trait_reflection::reflect_trait(&args, input)
}

/// A macro used to generate reflection trait implementations for the given type.
///
/// This is functionally the same as [deriving `Reflect`] using the `#[reflect_value]` container attribute.
///
/// The only reason for this macro's existence is so that `bevy_reflect` can easily implement the reflection traits
/// on primitives and other Rust types internally.
///
/// Since this macro also implements `TypePath`, the type path must be explicit.
/// See [`impl_type_path!`] for the exact syntax.
///
/// # Examples
///
/// Types can be passed with or without registering type data:
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_reflect_value!(my_crate::Foo);
/// impl_reflect_value!(my_crate::Bar(Debug, Default, Serialize, Deserialize));
/// ```
///
/// Generic types can also specify their parameters and bounds:
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_reflect_value!(my_crate::Foo<T1, T2: Baz> where T1: Bar (Default, Serialize, Deserialize));
/// ```
///
/// Custom type paths can be specified:
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_reflect_value!((in not_my_crate as NotFoo) Foo(Debug, Default));
/// ```
///
/// [deriving `Reflect`]: Reflect
#[proc_macro]
pub fn impl_reflect_value(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input with ReflectValueDef::parse_reflect);

    let default_name = &def.type_path.segments.last().unwrap().ident;
    let type_path = if def.type_path.leading_colon.is_none() && def.custom_path.is_none() {
        ReflectTypePath::Primitive(default_name)
    } else {
        ReflectTypePath::External {
            path: &def.type_path,
            custom_path: def.custom_path.map(|path| path.into_path(default_name)),
            generics: &def.generics,
        }
    };

    let meta = ReflectMeta::new(type_path, def.traits.unwrap_or_default());

    #[cfg(feature = "documentation")]
    let meta = meta.with_docs(documentation::Documentation::from_attributes(&def.attrs));

    let reflect_impls = impls::impl_value(&meta);
    let from_reflect_impl = from_reflect::impl_value(&meta);

    TokenStream::from(quote! {
        const _: () = {
            #reflect_impls
            #from_reflect_impl
        };
    })
}

/// A replacement for `#[derive(Reflect)]` to be used with foreign types which
/// the definitions of cannot be altered.
///
/// This macro is an alternative to [`impl_reflect_value!`] and [`impl_from_reflect_value!`]
/// which implement foreign types as Value types. Note that there is no `impl_from_reflect`,
/// as this macro will do the job of both. This macro implements them using one of the reflect
/// variant traits (`bevy_reflect::{Struct, TupleStruct, Enum}`, etc.),
/// which have greater functionality. The type being reflected must be in scope, as you cannot
/// qualify it in the macro as e.g. `bevy::prelude::Vec3`.
///
/// It is necessary to add a `#[type_path = "my_crate::foo"]` attribute to all types.
///
/// It may be necessary to add `#[reflect(Default)]` for some types, specifically non-constructible
/// foreign types. Without `Default` reflected for such types, you will usually get an arcane
/// error message and fail to compile. If the type does not implement `Default`, it may not
/// be possible to reflect without extending the macro.
///
///
/// # Example
/// Implementing `Reflect` for `bevy::prelude::Vec3` as a struct type:
/// ```ignore (bevy_reflect is not accessible from this crate)
/// use bevy::prelude::Vec3;
///
/// impl_reflect!(
///     #[reflect(PartialEq, Serialize, Deserialize, Default)]
///     #[type_path = "bevy::prelude"]
///     struct Vec3 {
///         x: f32,
///         y: f32,
///         z: f32
///     }
/// );
/// ```
#[proc_macro]
pub fn impl_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match_reflect_impls(ast, ReflectImplSource::ImplRemoteType)
}

/// A macro used to generate a `FromReflect` trait implementation for the given type.
///
/// This is functionally the same as [deriving `FromReflect`] on a type that [derives `Reflect`] using
/// the `#[reflect_value]` container attribute.
///
/// The only reason this macro exists is so that `bevy_reflect` can easily implement `FromReflect` on
/// primitives and other Rust types internally.
///
/// Please note that this macro will not work with any type that [derives `Reflect`] normally
/// or makes use of the [`impl_reflect_value!`] macro, as those macros also implement `FromReflect`
/// by default.
///
/// # Examples
///
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_from_reflect_value!(foo<T1, T2: Baz> where T1: Bar);
/// ```
///
/// [deriving `FromReflect`]: FromReflect
/// [derives `Reflect`]: Reflect
#[proc_macro]
pub fn impl_from_reflect_value(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input with ReflectValueDef::parse_from_reflect);

    let default_name = &def.type_path.segments.last().unwrap().ident;
    let type_path = if def.type_path.leading_colon.is_none()
        && def.custom_path.is_none()
        && def.generics.params.is_empty()
    {
        ReflectTypePath::Primitive(default_name)
    } else {
        ReflectTypePath::External {
            path: &def.type_path,
            custom_path: def.custom_path.map(|alias| alias.into_path(default_name)),
            generics: &def.generics,
        }
    };

    let from_reflect_impl =
        from_reflect::impl_value(&ReflectMeta::new(type_path, def.traits.unwrap_or_default()));

    TokenStream::from(quote! {
        const _: () = {
            #from_reflect_impl
        };
    })
}

/// A replacement for [deriving `TypePath`] for use on foreign types.
///
/// Since (unlike the derive) this macro may be invoked in a different module to where the type is defined,
/// it requires an 'absolute' path definition.
///
/// Specifically, a leading `::` denoting a global path must be specified
/// or a preceding `(in my_crate::foo)` to specify the custom path must be used.
///
/// # Examples
///
/// Implementing `TypePath` on a foreign type:
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_type_path!(::foreign_crate::foo::bar::Baz);
/// ```
///
/// On a generic type (this can also accept trait bounds):
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_type_path!(::foreign_crate::Foo<T>);
/// impl_type_path!(::foreign_crate::Goo<T: ?Sized>);
/// ```
///
/// On a primitive (note this will not compile for a non-primitive type):
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_type_path!(bool);
/// ```
///
/// With a custom type path:
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_type_path!((in other_crate::foo::bar) Baz);
/// ```
///
/// With a custom type path and a custom type name:
/// ```ignore (bevy_reflect is not accessible from this crate)
/// impl_type_path!((in other_crate::foo as Baz) Bar);
/// ```
///
/// [deriving `TypePath`]: TypePath
#[proc_macro]
pub fn impl_type_path(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as NamedTypePathDef);

    let type_path = match def {
        NamedTypePathDef::External {
            ref path,
            custom_path,
            ref generics,
        } => {
            let default_name = &path.segments.last().unwrap().ident;

            ReflectTypePath::External {
                path,
                custom_path: custom_path.map(|path| path.into_path(default_name)),
                generics,
            }
        }
        NamedTypePathDef::Primitive(ref ident) => ReflectTypePath::Primitive(ident),
    };

    let meta = ReflectMeta::new(type_path, ContainerAttributes::default());

    let type_path_impl = impls::impl_type_path(&meta);

    TokenStream::from(quote! {
        const _: () = {
            #type_path_impl
        };
    })
}
