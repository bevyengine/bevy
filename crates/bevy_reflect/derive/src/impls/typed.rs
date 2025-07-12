use crate::{
    derive_data::{ReflectMeta, ReflectTypePath},
    string_expr::StringExpr,
    where_clause_options::WhereClauseOptions,
};
use bevy_macro_utils::fq_std::FQOption;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

/// Returns an expression for a `NonGenericTypeCell` or `GenericTypeCell`  to generate `'static` references.
fn static_type_cell(
    meta: &ReflectMeta,
    property: TypedProperty,
    generator: TokenStream,
) -> TokenStream {
    let bevy_reflect_path = meta.bevy_reflect_path();
    if meta.type_path().impl_is_generic() {
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
                "Cannot have a non-generic type path cell. Use string literals and core::concat instead."
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

pub(crate) fn impl_type_path(meta: &ReflectMeta) -> TokenStream {
    let where_clause_options = WhereClauseOptions::new(meta);

    if !meta.attrs().type_path_attrs().should_auto_derive() {
        return TokenStream::new();
    }

    let type_path = meta.type_path();
    let bevy_reflect_path = meta.bevy_reflect_path();

    let (long_type_path, short_type_path) = if type_path.impl_is_generic() {
        let long_path_cell = static_type_cell(
            meta,
            TypedProperty::TypePath,
            type_path.long_type_path(bevy_reflect_path).into_owned(),
        );
        let short_path_cell = static_type_cell(
            meta,
            TypedProperty::TypePath,
            type_path.short_type_path(bevy_reflect_path).into_owned(),
        );
        (
            long_path_cell.to_token_stream(),
            short_path_cell.to_token_stream(),
        )
    } else {
        (
            type_path.long_type_path(bevy_reflect_path).into_borrowed(),
            type_path.short_type_path(bevy_reflect_path).into_borrowed(),
        )
    };

    let type_ident = wrap_in_option(type_path.type_ident().map(StringExpr::into_borrowed));
    let module_path = wrap_in_option(type_path.module_path().map(StringExpr::into_borrowed));
    let crate_name = wrap_in_option(type_path.crate_name().map(StringExpr::into_borrowed));

    let primitive_assert = if let ReflectTypePath::Primitive(_) = type_path {
        Some(quote! {
            const _: () = {
                mod private_scope {
                    // Compiles if it can be named when there are no imports.
                    type AssertIsPrimitive = #type_path;
                }
            };
        })
    } else {
        None
    };

    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();

    // Add Typed bound for each active field
    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    quote! {
        #primitive_assert

        impl #impl_generics #bevy_reflect_path::TypePath for #type_path #ty_generics #where_reflect_clause {
            fn type_path() -> &'static str {
                #long_type_path
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
    where_clause_options: &WhereClauseOptions,
    type_info_generator: TokenStream,
) -> TokenStream {
    let meta = where_clause_options.meta();
    let type_path = meta.type_path();
    let bevy_reflect_path = meta.bevy_reflect_path();

    let type_info_cell = static_type_cell(meta, TypedProperty::TypeInfo, type_info_generator);

    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();

    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    quote! {
        impl #impl_generics #bevy_reflect_path::Typed for #type_path #ty_generics #where_reflect_clause {
            #[inline]
            fn type_info() -> &'static #bevy_reflect_path::TypeInfo {
                #type_info_cell
            }
        }
    }
}

/// Turns an `Option<TokenStream>` into a `TokenStream` for an `Option`.
fn wrap_in_option(tokens: Option<TokenStream>) -> TokenStream {
    match tokens {
        Some(tokens) => quote! {
            #FQOption::Some(#tokens)
        },
        None => quote! {
            #FQOption::None
        },
    }
}
