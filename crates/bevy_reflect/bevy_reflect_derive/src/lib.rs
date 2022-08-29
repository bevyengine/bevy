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
mod enum_utility;
mod field_attributes;
mod from_reflect;
mod impls;
mod reflect_value;
mod registration;
mod trait_reflection;
mod type_name;
mod type_uuid;
mod utility;

use crate::derive_data::{ReflectDerive, ReflectMeta, ReflectStruct};
use proc_macro::TokenStream;
use quote::quote;
use reflect_value::{NamedReflectValueDef, ReflectValueDef};
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput};
use type_name::TypeNameDef;

pub(crate) static REFLECT_ATTRIBUTE_NAME: &str = "reflect";
pub(crate) static REFLECT_VALUE_ATTRIBUTE_NAME: &str = "reflect_value";
pub(crate) static TYPE_NAME_ATTRIBUTE_NAME: &str = "type_name";

#[proc_macro_derive(Reflect, attributes(reflect, reflect_value, module, type_name))]
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

/// Implement [`TypeName`] on a type. The type name is automatically deducted following a
/// specific convention.
///
/// ## Type name convention
///
/// The type name is the module path followed by the ident of the type.
/// If the type is generic the type name of it's generic parameter is included between `<` and `>`.
///
/// See examples.
///
/// ## Custom type name
///
/// It's possible to override the default behaviour and choosing a custom type name by using
/// the `type_name` attribute after the `derive` attribute.
///
/// A common usage is to using your crate name instead of the complete module path.
///
/// It's highly discouraged to using unprefixed type name that could collide with another type
/// or an malformed type name (e.g. `BlAH@blah blah`).
///
/// ## Example
///
/// ```ignore
/// # bevy_reflect::TypeName;
///
/// mod a {
///     pub mod b {
///         pub mod c {
///             #[derive(TypeName)]
///             pub struct ABC;
///         }
///
///         #[derive(TypeName)]
///         #[type_name("my_lib::AB")]
///         pub struct AB<T>(T);
///     }
///
///     #[derive(TypeName)]
///     pub struct A<const N: usize>(N);
/// }
///
/// # use a::A;
/// # use a::b::AB;
/// # use a::b::c::ABC;
///
/// assert_eq!(ABC::name(), "a::b::c::ABC");
/// assert_eq!(AB::<u32>::name(), "my_lib::AB<u32>");
/// assert_eq!(A::<5>::name(), "a::A<5>");
/// ```
#[proc_macro_derive(TypeName, attributes(module, type_name))]
pub fn derive_type_name(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let derive_data = match ReflectDerive::from_input(&ast) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    match derive_data {
        ReflectDerive::TupleStruct(struct_data)
        | ReflectDerive::Struct(struct_data)
        | ReflectDerive::UnitStruct(struct_data) => impls::impl_type_name(struct_data.meta()),
        ReflectDerive::Enum(meta) => impls::impl_type_name(meta.meta()),
        ReflectDerive::Value(meta) => impls::impl_type_name(&meta),
    }
    .into()
}

/// Derives the `FromReflect` trait.
///
/// This macro supports the following field attributes:
/// * `#[reflect(ignore)]`: Ignores the field. This requires the field to implement [`Default`].
/// * `#[reflect(default)]`: If the field's value cannot be read, uses its [`Default`] implementation.
/// * `#[reflect(default = "some_func")]`: If the field's value cannot be read, uses the function with the given name.
///
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

#[proc_macro_attribute]
pub fn reflect_trait(args: TokenStream, input: TokenStream) -> TokenStream {
    trait_reflection::reflect_trait(&args, input)
}

#[proc_macro]
pub fn impl_reflect_value(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as NamedReflectValueDef);

    let reflected_type_name = def.get_reflected_type_name();

    impls::impl_value(&ReflectMeta::new(
        &def.def.type_name,
        &def.def.generics,
        def.def.traits.unwrap_or_default(),
        Some(reflected_type_name),
    ))
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

#[proc_macro]
pub fn impl_from_reflect_value(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as ReflectValueDef);
    from_reflect::impl_value(&ReflectMeta::new(
        &def.type_name,
        &def.generics,
        def.traits.unwrap_or_default(),
        None,
    ))
}

/// A replacement for `#[derive(TypeName)]` to be used with foreign types which
/// the definitions of cannot be altered.
///
/// But unlike `#[derive(TypeName)]` that prefix the type name with the module path
/// using the macro [`module_path`], `impl_type_name` use only the ident of the type
/// as type name.
///
/// # Example
/// Implementing `TypeName` for `Vec<T>`:
/// ```ignore
/// impl_type_name!(Vec<T: TypeName>);
/// ```
#[proc_macro]
pub fn impl_type_name(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as TypeNameDef);
    let meta = ReflectMeta::new(
        &def.type_name,
        &def.generics,
        Default::default(),
        Some(def.type_name.to_string()),
    );
    TokenStream::from(impls::impl_type_name(&meta))
}
