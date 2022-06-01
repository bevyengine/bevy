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
mod type_uuid;
mod utility;

use crate::derive_data::{ReflectDerive, ReflectMeta, ReflectStruct};
use proc_macro::TokenStream;
use quote::quote;
use reflect_value::ReflectValueDef;
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput};

pub(crate) static REFLECT_ATTRIBUTE_NAME: &str = "reflect";
pub(crate) static REFLECT_VALUE_ATTRIBUTE_NAME: &str = "reflect_value";

#[proc_macro_derive(Reflect, attributes(reflect, reflect_value, module))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let derive_data = match ReflectDerive::from_input(&ast) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    match derive_data {
        ReflectDerive::Struct(struct_data) => impls::impl_struct(&struct_data),
        ReflectDerive::UnitStruct(struct_data) => impls::impl_struct(&struct_data),
        ReflectDerive::TupleStruct(struct_data) => impls::impl_tuple_struct(&struct_data),
        ReflectDerive::Enum(meta) => impls::impl_enum(&meta),
        ReflectDerive::Value(meta) => impls::impl_value(&meta),
    }
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
    let reflect_value_def = parse_macro_input!(input as ReflectValueDef);
    let meta = reflect_value_def.as_meta();
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
        _ => syn::Error::new(
            ast.span(),
            "impl_reflect_struct is only supported for standard structs",
        )
        .into_compile_error()
        .into(),
    }
}

#[proc_macro]
pub fn impl_from_reflect_value(input: TokenStream) -> TokenStream {
    let reflect_value_def = parse_macro_input!(input as ReflectValueDef);
    let meta = reflect_value_def.as_meta();
    from_reflect::impl_value(&meta)
}
