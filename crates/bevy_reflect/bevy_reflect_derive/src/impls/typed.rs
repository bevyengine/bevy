use crate::utility::{extend_where_clause, WhereClauseOptions};
use crate::ReflectMeta;
use proc_macro2::Ident;
use syn::Generics;
use std::borrow::Cow;
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, LitStr};

use crate::{
    derive_data::{ReflectMeta, ReflectTypePath},
    utility::wrap_in_option,
};

fn combine_generics(
    ty_generics: Vec<proc_macro2::TokenStream>,
    generics: &Generics,
) -> impl Iterator<Item = proc_macro2::TokenStream> {
    let const_generic_strings: Vec<_> = generics
        .const_params()
        .map(|param| {
            let ident = &param.ident;
            let ty = &param.ty;

            quote! {
                &<#ty as ::std::string::ToString>::to_string(&#ident)
            }
        })
        .collect();

    let mut generics = ty_generics
        .into_iter()
        .chain(const_generic_strings.into_iter())
        .flat_map(|t| [", ".to_token_stream(), t]);
    generics.next(); // Skip first comma.
    generics
}

/// Returns an expression for a `&'static str`,
/// representing either a [long path](long) or [short path](short).
///
/// [long]: ReflectTypePath::non_generic_type_path
/// [short]: ReflectTypePath::non_generic_short_path
fn type_path_generator(long_path: bool, meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let path_to_type = meta.path_to_type();
    let generics = meta.generics();
    let bevy_reflect_path = meta.bevy_reflect_path();

    if let ReflectTypePath::Primitive(name) = path_to_type {
        let name = LitStr::new(&name.to_string(), name.span());
        return quote! {
            #bevy_reflect_path::utility::TypePathStorage::new_primitive(#name)
        };
    }

    let ty_generic_paths: Vec<_> = generics
        .type_params()
        .map(|param| {
            let ident = &param.ident;
            quote! {
                <#ident as #bevy_reflect_path::TypePath>
            }
        })
        .collect();

    let ident = path_to_type.get_ident().unwrap().to_string();
    let ident = LitStr::new(&ident, path_to_type.span());

    let path = if long_path {
        let ty_generics: Vec<_> = ty_generic_paths
            .iter()
            .map(|cell| {
                quote! {
                    #cell::type_path()
                }
            })
            .collect();

        let generics = combine_generics(ty_generics, generics);
        let path = path_to_type.non_generic_type_path();

        quote! {
            ::std::borrow::ToOwned::to_owned(::core::concat!(#path, "::<"))
                #(+ #generics)*
                + ">"
        }
    } else {
        let ty_generics: Vec<_> = ty_generic_paths
            .iter()
            .map(|cell| {
                quote! {
                    #cell::short_type_path()
                }
            })
            .collect();

        let generics = combine_generics(ty_generics, generics);

        quote! {
            ::std::borrow::ToOwned::to_owned(::core::concat!(#ident, "<"))
                #(+ #generics)*
                + ">"
        }
    };

    path
}

/// Returns an expression for a `NonGenericTypedCell` or `GenericTypedCell`.
fn static_typed_cell(
    meta: &ReflectMeta,
    property: TypedProperty,
    generator: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let bevy_reflect_path = meta.bevy_reflect_path();
    if meta.impl_is_generic() {
        let cell_type = match property {
            TypedProperty::TypePath => quote!(GenericTypePathCell),
            TypedProperty::TypeInfo => quote!(GenericTypeInfoCell),
        };

        quote! {
            static CELL: #bevy_reflect_path::utility::#cell_type = #bevy_reflect_path::utility::#cell_type::new();
            CELL.get_or_insert::<Self, _>(|| {
                #generator
            })
        }
    } else {
        let cell_type = match property {
            TypedProperty::TypePath => unreachable!(
                "cannot have a non-generic type path cell. use string literals instead."
            ),
            TypedProperty::TypeInfo => quote!(NonGenericTypeInfoCell),
        };

        quote! {
            static CELL: #bevy_reflect_path::utility::#cell_type = #bevy_reflect_path::utility::#cell_type::new();
            CELL.get_or_set(|| {
                #generator
            })
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum TypedProperty {
    TypeInfo,
    TypePath,
}

pub(crate) fn impl_type_path(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
) -> proc_macro2::TokenStream {
    let path_to_type = meta.path_to_type();
    let bevy_reflect_path = meta.bevy_reflect_path();

    let (type_path, short_type_path) = if meta.impl_is_generic() {
        let long_path_cell = static_typed_cell(
            meta,
            TypedProperty::TypePath,
            type_path_generator(true, meta),
        );
        let short_path_cell = static_typed_cell(
            meta,
            TypedProperty::TypePath,
            type_path_generator(false, meta),
        );
        (
            long_path_cell.to_token_stream(),
            short_path_cell.to_token_stream(),
        )
    } else {
        (
            path_to_type.non_generic_type_path(),
            path_to_type.non_generic_short_path(),
        )
    };

    let type_ident = wrap_in_option(path_to_type.type_ident());
    let module_path = wrap_in_option(path_to_type.module_path());
    let crate_name = wrap_in_option(path_to_type.crate_name());

    // Add Typed bound for each active field
    let where_reflect_clause = extend_where_clause(where_clause, where_clause_options);

    let primitive_assert = if let ReflectTypePath::Primitive(_) = path_to_type {
        Some(quote! {
            const _: () = {
                mod private_scope {
                    // Compiles if it can be named when there are no imports.
                    type AssertIsPrimitive = #path_to_type;
                }
            };
        })
    } else {
        None
    };

    let (impl_generics, ty_generics, where_clause) = meta.generics().split_for_impl();

    quote! {
        #primitive_assert

        impl #impl_generics #bevy_reflect_path::TypePath for #path_to_type #ty_generics #where_reflect_clause {
            fn type_path() -> &'static str {
                #type_path
            }

            fn short_type_path() -> &'static str {
                #short_type_path
            }

            fn type_ident() -> Option<&'static str> {
                #type_ident
            }

            fn crate_name() -> Option<&'static str> {
                #crate_name
            }

            fn module_path() -> Option<&'static str> {
                #module_path
            }
        }
    }
}

pub(crate) fn impl_typed(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
    type_info_generator: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let path_to_type = meta.path_to_type();
    let bevy_reflect_path = meta.bevy_reflect_path();

    let type_path_impl = impl_type_path(meta, where_clause_options);

    let type_info_cell = static_typed_cell(meta, TypedProperty::TypeInfo, type_info_generator);

    let (impl_generics, ty_generics, where_clause) = meta.generics().split_for_impl();

    let where_reflect_clause = extend_where_clause(where_clause, where_clause_options);

    quote! {
        #type_path_impl

        impl #impl_generics #bevy_reflect_path::Typed for #path_to_type #ty_generics #where_reflect_clause {
            fn type_info() -> &'static #bevy_reflect_path::TypeInfo {
                #type_info_cell
            }
        }
    }
}
