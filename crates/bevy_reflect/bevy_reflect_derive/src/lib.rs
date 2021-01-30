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
mod field_attributes;
mod from_reflect;
mod impls;
mod reflect_value;
mod registration;
mod trait_reflection;
mod type_uuid;
mod utility;

use crate::container_attributes::ReflectTraits;
use crate::derive_data::ReflectDeriveData;
use derive_data::DeriveType;
use proc_macro::TokenStream;
use quote::quote;
use reflect_value::ReflectValueDef;
use syn::{parse_macro_input, Data, DeriveInput, Meta};

pub(crate) static REFLECT_ATTRIBUTE_NAME: &str = "reflect";
pub(crate) static REFLECT_VALUE_ATTRIBUTE_NAME: &str = "reflect_value";

#[proc_macro_derive(Reflect, attributes(reflect, reflect_value, module))]
pub fn derive_reflect(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    // TODO: Update and replace
    if let Data::Enum(enum_data) = ast.data {
        let mut traits = ReflectTraits::default();
        for attribute in ast.attrs.iter().filter_map(|attr| attr.parse_meta().ok()) {
            let meta_list = if let Meta::List(meta_list) = attribute {
                meta_list
            } else {
                continue;
            };

            if let Some(ident) = meta_list.path.get_ident() {
                if ident == REFLECT_ATTRIBUTE_NAME {
                    traits = ReflectTraits::from_nested_metas(&meta_list.nested);
                } else if ident == REFLECT_VALUE_ATTRIBUTE_NAME {
                    traits = ReflectTraits::from_nested_metas(&meta_list.nested);
                }
            }
        }

        let reflect_path = utility::get_bevy_reflect_path();

        let variants = enum_data
            .variants
            .iter()
            .enumerate()
            .map(|(index, variant)| (variant, index))
            .collect::<Vec<_>>();

        return impls::impl_enum(
            &ast.ident,
            &ast.generics,
            registration::impl_get_type_registration(
                &ast.ident,
                &reflect_path,
                traits.idents(),
                &ast.generics,
            ),
            &reflect_path,
            &traits,
            &variants,
        );
    }

    let derive_data = match ReflectDeriveData::from_input(&ast) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    match derive_data.derive_type() {
        DeriveType::Struct | DeriveType::UnitStruct => impls::impl_struct(&derive_data),
        DeriveType::TupleStruct => impls::impl_tuple_struct(&derive_data),
        DeriveType::Value => impls::impl_value(
            derive_data.type_name(),
            derive_data.generics(),
            derive_data.get_type_registration(),
            derive_data.bevy_reflect_path(),
            derive_data.traits(),
        ),
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

    let derive_data = match ReflectDeriveData::from_input(&ast) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    match derive_data.derive_type() {
        DeriveType::Struct | DeriveType::UnitStruct => from_reflect::impl_struct(&derive_data),
        DeriveType::TupleStruct => from_reflect::impl_tuple_struct(&derive_data),
        DeriveType::Value => from_reflect::impl_value(
            derive_data.type_name(),
            &ast.generics,
            derive_data.bevy_reflect_path(),
        ),
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

    let bevy_reflect_path = utility::get_bevy_reflect_path();
    let ty = &reflect_value_def.type_name;
    let reflect_traits = reflect_value_def.traits.unwrap_or_default();
    let registration_data = &reflect_traits.idents();
    let get_type_registration_impl = registration::impl_get_type_registration(
        ty,
        &bevy_reflect_path,
        registration_data,
        &reflect_value_def.generics,
    );
    impls::impl_value(
        ty,
        &reflect_value_def.generics,
        get_type_registration_impl,
        &bevy_reflect_path,
        &reflect_traits,
    )
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
    let derive_data = match ReflectDeriveData::from_input(&ast) {
        Ok(data) => data,
        Err(err) => return err.into_compile_error().into(),
    };

    let impl_struct: proc_macro2::TokenStream = impls::impl_struct(&derive_data).into();
    let impl_from_struct: proc_macro2::TokenStream = from_reflect::impl_struct(&derive_data).into();

    TokenStream::from(quote! {
        #impl_struct

        #impl_from_struct
    })
}

#[proc_macro]
pub fn impl_from_reflect_value(input: TokenStream) -> TokenStream {
    let reflect_value_def = parse_macro_input!(input as ReflectValueDef);

    let bevy_reflect_path = utility::get_bevy_reflect_path();
    let ty = &reflect_value_def.type_name;
    from_reflect::impl_value(ty, &reflect_value_def.generics, &bevy_reflect_path)
}
