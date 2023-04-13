//! This crate contains macros used by Bevy's `Reflect` API.
//!
//! The main export of this crate is the derive macro for [`Reflect`]. This allows
//! types to easily implement `Reflect` along with other `bevy_reflect` traits,
//! such as `Struct`, `GetTypeRegistration`, and more— all with a single derive!
//!
//! Some other noteworthy exports include the derive macros for [`FromReflect`] and
//! [`TypeUuid`], as well as the [`reflect_trait`] attribute macro.
//!
//! [`Reflect`]: crate::derive_reflect
//! [`FromReflect`]: crate::derive_from_reflect
//! [`TypeUuid`]: crate::derive_type_uuid
//! [`reflect_trait`]: macro@reflect_trait

extern crate proc_macro;

mod container_attributes;
mod derive_data;
#[cfg(feature = "documentation")]
mod documentation;
mod enum_utility;
mod field_attributes;
mod fq_std;
mod from_reflect;
mod impls;
mod reflect_value;
mod registration;
mod trait_reflection;
mod type_uuid;
mod utility;

use crate::derive_data::{ReflectDerive, ReflectMeta, ReflectStruct};
use crate::type_uuid::gen_impl_type_uuid;
use proc_macro::TokenStream;
use quote::quote;
use reflect_value::ReflectValueDef;
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput};
use type_uuid::TypeUuidDef;

pub(crate) static REFLECT_ATTRIBUTE_NAME: &str = "reflect";
pub(crate) static REFLECT_VALUE_ATTRIBUTE_NAME: &str = "reflect_value";

/// The main derive macro used by `bevy_reflect` for deriving its `Reflect` trait.
///
/// This macro can be used on all structs and enums (unions are not supported).
/// It will automatically generate the implementations for `Reflect`, `Typed`, and `GetTypeRegistration`.
/// And, depending on the item's structure, will either implement `Struct`, `TupleStruct`, or `Enum`.
///
/// # Container Attributes
///
/// This macro comes with some helper attributes that can be added to the container item
/// in order to provide additional functionality or alter the generated implementations.
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
///   a base value using its [`Default`] implementation avoiding issues with ignored fields.
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
/// [`reflect_trait`]: macro@reflect_trait
#[proc_macro_derive(Reflect, attributes(reflect, reflect_value))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let derive_data = match ReflectDerive::from_input(&ast) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    match derive_data {
        ReflectDerive::Struct(struct_data) | ReflectDerive::UnitStruct(struct_data) => {
            impls::impl_struct(&struct_data)
        }
        ReflectDerive::TupleStruct(struct_data) => impls::impl_tuple_struct(&struct_data),
        ReflectDerive::Enum(meta) => impls::impl_enum(&meta),
        ReflectDerive::Value(meta) => impls::impl_value(&meta),
    }
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
/// This takes the form: `#[reflect(default = "path::to::my_function)]` where `my_function` is a parameterless
/// function that must return some default value for the type.
///
/// Specifying a custom default can be used to give different fields their own specialized defaults,
/// or to remove the `Default` requirement on fields marked with `#[reflect(ignore)]`.
/// Additionally, either form of this attribute can be used to fill in fields that are simply missing,
/// such as when converting a partially-constructed dynamic type to a concrete one.
#[proc_macro_derive(FromReflect, attributes(reflect))]
pub fn derive_from_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let derive_data = match ReflectDerive::from_input(&ast) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    match derive_data {
        ReflectDerive::Struct(struct_data) | ReflectDerive::UnitStruct(struct_data) => {
            from_reflect::impl_struct(&struct_data)
        }
        ReflectDerive::TupleStruct(struct_data) => from_reflect::impl_tuple_struct(&struct_data),
        ReflectDerive::Enum(meta) => from_reflect::impl_enum(&meta),
        ReflectDerive::Value(meta) => from_reflect::impl_value(&meta),
    }
}

// From https://github.com/randomPoison/type-uuid
#[proc_macro_derive(TypeUuid, attributes(uuid))]
pub fn derive_type_uuid(input: TokenStream) -> TokenStream {
    type_uuid::type_uuid_derive(input)
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
/// ```ignore
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
/// # Examples
///
/// Types can be passed with or without registering type data:
///
/// ```ignore
/// impl_reflect_value!(foo);
/// impl_reflect_value!(bar(Debug, Default, Serialize, Deserialize));
/// ```
///
/// Generic types can also specify their parameters and bounds:
///
/// ```ignore
/// impl_reflect_value!(foo<T1, T2: Baz> where T1: Bar (Default, Serialize, Deserialize));
/// ```
///
/// [deriving `Reflect`]: Reflect
#[proc_macro]
pub fn impl_reflect_value(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as ReflectValueDef);
    let meta = ReflectMeta::new(
        &def.type_name,
        &def.generics,
        def.traits.unwrap_or_default(),
    );

    #[cfg(feature = "documentation")]
    let meta = meta.with_docs(documentation::Documentation::from_attributes(&def.attrs));

    impls::impl_value(&meta)
}

/// A replacement for `#[derive(Reflect)]` to be used with foreign types which
/// the definitions of cannot be altered.
///
/// This macro is an alternative to [`impl_reflect_value!`] and [`impl_from_reflect_value!`]
/// which implement foreign types as Value types. Note that there is no `impl_from_reflect_struct`,
/// as this macro will do the job of both. This macro implements them as `Struct` types,
/// which have greater functionality. The type being reflected must be in scope, as you cannot
/// qualify it in the macro as e.g. `bevy::prelude::Vec3`.
///
/// It may be necessary to add `#[reflect(Default)]` for some types, specifically non-constructible
/// foreign types. Without `Default` reflected for such types, you will usually get an arcane
/// error message and fail to compile. If the type does not implement `Default`, it may not
/// be possible to reflect without extending the macro.
///
/// # Example
/// Implementing `Reflect` for `bevy::prelude::Vec3` as a struct type:
/// ```ignore
/// use bevy::prelude::Vec3;
///
/// impl_reflect_struct!(
///    #[reflect(PartialEq, Serialize, Deserialize, Default)]
///    struct Vec3 {
///        x: f32,
///        y: f32,
///        z: f32
///    }
/// );
/// ```
#[proc_macro]
pub fn impl_reflect_struct(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let derive_data = match ReflectDerive::from_input(&ast) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    match derive_data {
        ReflectDerive::Struct(struct_data) => {
            let impl_struct: proc_macro2::TokenStream = impls::impl_struct(&struct_data).into();
            let impl_from_struct: proc_macro2::TokenStream =
                from_reflect::impl_struct(&struct_data).into();

            TokenStream::from(quote! {
                #impl_struct

                #impl_from_struct
            })
        }
        ReflectDerive::TupleStruct(..) => syn::Error::new(
            ast.span(),
            "impl_reflect_struct does not support tuple structs",
        )
        .into_compile_error()
        .into(),
        ReflectDerive::UnitStruct(..) => syn::Error::new(
            ast.span(),
            "impl_reflect_struct does not support unit structs",
        )
        .into_compile_error()
        .into(),
        _ => syn::Error::new(ast.span(), "impl_reflect_struct only supports structs")
            .into_compile_error()
            .into(),
    }
}

/// A macro used to generate a `FromReflect` trait implementation for the given type.
///
/// This is functionally the same as [deriving `FromReflect`] on a type that [derives `Reflect`] using
/// the `#[reflect_value]` container attribute.
///
/// The only reason this macro exists is so that `bevy_reflect` can easily implement `FromReflect` on
/// primitives and other Rust types internally.
///
/// # Examples
///
/// ```ignore
/// impl_from_reflect_value!(foo<T1, T2: Baz> where T1: Bar);
/// ```
///
/// [deriving `FromReflect`]: FromReflect
/// [derives `Reflect`]: Reflect
#[proc_macro]
pub fn impl_from_reflect_value(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as ReflectValueDef);
    from_reflect::impl_value(&ReflectMeta::new(
        &def.type_name,
        &def.generics,
        def.traits.unwrap_or_default(),
    ))
}

/// Derives `TypeUuid` for the given type. This is used internally to implement `TypeUuid` on foreign types, such as those in the std. This macro should be used in the format of `<[Generic Params]> [Type (Path)], [Uuid (String Literal)]`.
#[proc_macro]
pub fn impl_type_uuid(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as TypeUuidDef);
    gen_impl_type_uuid(def)
}
