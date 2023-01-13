
use crate::utility::{extend_where_clause, WhereClauseOptions};
use crate::ReflectMeta;
use proc_macro2::Ident;
use quote::quote;

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

pub(crate) fn impl_type_path(meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let path_to_type = meta.path_to_type();
    let bevy_reflect_path = meta.bevy_reflect_path();

    let type_path_cell =
        static_typed_cell(meta, TypedProperty::TypePath, type_path_generator(meta));

    let (impl_generics, ty_generics, where_clause) = meta.generics().split_for_impl();

    // Add Typed bound for each active field
    let where_reflect_clause = extend_where_clause(where_clause, where_clause_options);

    quote! {
        const _: () = {
            trait GetStorage {
                fn get_storage() -> &'static #bevy_reflect_path::utility::TypePathStorage;
            }

            impl #impl_generics GetStorage for #path_to_type #ty_generics #where_reflect_clause {
                #[inline]
                fn get_storage() -> &'static #bevy_reflect_path::utility::TypePathStorage {
                    #type_path_cell
                }
            }

            impl #impl_generics #bevy_reflect_path::TypePath for #path_to_type #ty_generics #where_reflect_clause {
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

pub(crate) fn impl_typed(
    meta: &ReflectMeta,
    where_clause_options: &WhereClauseOptions,
    type_info_generator: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let path_to_type = meta.path_to_type();
    let bevy_reflect_path = meta.bevy_reflect_path();

    let type_path_impl = impl_type_path(meta);

    let type_info_cell =
        static_typed_cell(meta, TypedProperty::TypeInfo, type_info_generator);

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
