use crate::utility::{extend_where_clause, WhereClauseOptions};
use crate::ReflectMeta;
use proc_macro2::Ident;
use std::borrow::Cow;
use quote::quote;
use syn::{spanned::Spanned, LitStr};

use crate::{derive_data::{PathToType, ReflectMeta}, fq_std::FQOption};

/// Returns an expression for a `TypePathStorage`.
pub(crate) fn type_path_generator(meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let path_to_type = meta.path_to_type();
    let generics = meta.generics();
    let bevy_reflect_path = meta.bevy_reflect_path();

    if let PathToType::Primitive(name) = path_to_type {
        let name = LitStr::new(&name.to_string(), name.span());
        return quote! {
            #bevy_reflect_path::utility::TypePathStorage::new_primitive(#name)
        };
    }

    let is_generic = meta.impl_is_generic();

    let ty_generic_paths: Vec<_> = generics
        .type_params()
        .map(|param| {
            let ident = &param.ident;
            quote! {
                <#ident as #bevy_reflect_path::TypePath>
            }
        })
        .collect();

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

    let comma = quote!(", ");

    let combine_generics = |ty_generics: Vec<proc_macro2::TokenStream>| {
        let mut generics = ty_generics
            .into_iter()
            .map(Cow::Owned)
            .chain(const_generic_strings.iter().map(Cow::Borrowed))
            .flat_map(|t| [Cow::Borrowed(&comma), t]);
        generics.next(); // Skip first comma.
        generics
    };

    let ident = path_to_type.ident().unwrap().to_string();
    let ident = LitStr::new(&ident, path_to_type.span());

    let path = {
        let path = path_to_type.non_generic_path();

        if is_generic {
            let ty_generics: Vec<_> = ty_generic_paths
                .iter()
                .map(|type_path| {
                    quote! {
                        #type_path::type_path()
                    }
                })
                .collect();

            let generics = combine_generics(ty_generics);

            quote! {
                ::std::borrow::ToOwned::to_owned(#path)
                    + "::<"
                    #(+ #generics)*
                    + ">"
            }
        } else {
            quote! {
                #path
            }
        }
    };

    let short_path = {
        if is_generic {
            let ty_generics: Vec<_> = ty_generic_paths
                .iter()
                .map(|type_path| {
                    quote! {
                        #type_path::short_type_path()
                    }
                })
                .collect();

            let generics = combine_generics(ty_generics);

            quote! {
                ::std::borrow::ToOwned::to_owned(::core::concat!(#ident, "<"))
                    #(+ #generics)*
                    + ">"
            }
        } else {
            quote! {
                #ident.to_owned()
            }
        }
    };

    if !path_to_type.is_named() {
        quote! {
            #bevy_reflect_path::utility::TypePathStorage::new_anonymous(
                #path,
                #short_path,
            )
        }
    } else {
        let crate_name = path_to_type.crate_name();
        let module_path = path_to_type.module_path();

        quote! {
            #bevy_reflect_path::utility::TypePathStorage::new_named(
                #path,
                #short_path,
                #ident.to_owned(),
                #crate_name.to_owned(),
                #module_path.to_owned(),
            )
        }
    }
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
            TypedProperty::TypePath => quote!(NonGenericTypePathCell),
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

    let (type_path_storage, type_path, short_type_path) = if meta.impl_is_generic() {
        let cell = static_typed_cell(meta, TypedProperty::TypePath, type_path_generator(meta));

        (
            Some(cell),
            quote! {
                Self::get_storage().path()
            },
            quote! {
                Self::get_storage().short_path()
            },
        )
    } else {
        (
            None,
            path_to_type.non_generic_path(),
            path_to_type.non_generic_short_path(),
        )
    };

    let name = path_to_type.name().map(|name| quote! {
        #FQOption::Some(#name)
    }).unwrap_or_else(|| quote! {
        #FQOption::None
    });
    let module_path = path_to_type.module_path().map(|name| quote! {
        #FQOption::Some(#name)
    }).unwrap_or_else(|| quote! {
        #FQOption::None
    });
    let crate_name = path_to_type.crate_name().map(|name| quote! {
        #FQOption::Some(#name)
    }).unwrap_or_else(|| quote! {
        #FQOption::None
    });

    // Add Typed bound for each active field
    let where_reflect_clause = extend_where_clause(where_clause, where_clause_options);

    let primitive_assert = if let PathToType::Primitive(_) = path_to_type {
        Some(quote! {
            mod private_scope {
                // Compiles if it can be named with its ident when there are no imports.
                type AssertIsPrimitive = #path_to_type;
            }
        })
    } else {
        None
    };

    let (impl_generics, ty_generics, where_clause) = meta.generics().split_for_impl();

    let type_path_helper = type_path_storage.map(|storage| {
        quote! {
            trait GetStorage {
                fn get_storage() -> &'static #bevy_reflect_path::utility::TypePathStorage;
            }

            impl #impl_generics GetStorage for #path_to_type #ty_generics #where_reflect_clause {
                #[inline]
                fn get_storage() -> &'static #bevy_reflect_path::utility::TypePathStorage {
                    #storage
                }
            }
        }
    });

    quote! {
        const _: () = {
            #primitive_assert

            #type_path_helper


            impl #impl_generics #bevy_reflect_path::TypePath for #path_to_type #ty_generics #where_reflect_clause {
                fn type_path() -> &'static str {
                    #type_path
                }

                fn short_type_path() -> &'static str {
                    #short_type_path
                }

                fn type_ident() -> Option<&'static str> {
                    #name
                }

                fn crate_name() -> Option<&'static str> {
                    #crate_name
                }

                fn module_path() -> Option<&'static str> {
                    #module_path
                }
            }
        };
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
