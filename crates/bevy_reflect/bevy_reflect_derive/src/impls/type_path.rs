use quote::quote;

use crate::derive_data::ReflectMeta;

pub(crate) fn impl_type_path(reflect_meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let generics = reflect_meta.generics();
    let type_name = reflect_meta.type_name();
    let bevy_reflect_path = reflect_meta.bevy_reflect_path();
    let type_path_options = reflect_meta.type_path_options();

    let is_generic = !generics.params.is_empty();

    let module_path = match type_path_options.module_path.as_ref() {
        Some(x) if x.is_empty() => None,
        Some(x) => Some(quote!(#x)),
        None => Some(quote!(module_path!())),
    };

    let module_path_len = match module_path.as_ref() {
        Some(module_path) => quote!(#module_path.len()),
        None => quote!(0),
    };

    let module_path_2columns = match module_path.as_ref() {
        Some(module_path) => quote!(concat!(#module_path, "::")),
        None => quote!(""),
    };

    let crate_name_len = match module_path {
        Some(module_path) => quote!(#bevy_reflect_path::utility::crate_name_len(#module_path)),
        None => quote!(0),
    };

    let type_ident = type_path_options
        .type_ident
        .as_ref()
        .map(|x| quote!(#x))
        .unwrap_or_else(|| {
            let type_name = type_name.to_string();
            quote!(#type_name)
        });

    let get_type_path = if is_generic {
        let values = {
            let getters = generics.params.iter().map(|p| match p {
                syn::GenericParam::Type(p) => {
                    let ty = &p.ident;
                    quote!(<#ty as #bevy_reflect_path::TypePath>::type_path())
                }
                syn::GenericParam::Lifetime(p) => {
                    let name = format!("'{}", p.lifetime.ident.to_string());
                    quote!(#name)
                }
                syn::GenericParam::Const(p) => {
                    let name = &p.ident;
                    quote!(#name)
                }
            });

            quote!(#(#getters),*)
        };

        let brackets = {
            let brackets = vec![quote!({}); generics.params.len()];
            quote!(#(#brackets),*).to_string()
        };

        quote! {
            static CELL: #bevy_reflect_path::utility::GenericTypePathCell = #bevy_reflect_path::utility::GenericTypePathCell::new();
            CELL.get_or_insert::<Self, _>(|| {
                format!(concat!(#module_path_2columns, #type_ident, "<", #brackets, ">"), #values)
            })
        }
    } else {
        quote! {
            concat!(#module_path_2columns, #type_ident)
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics #bevy_reflect_path::TypePath for #type_name #ty_generics #where_clause {
            #[inline]
            fn type_path() -> &'static str {
                #get_type_path
            }

            #[inline]
            fn short_type_name_base() -> &'static str {
                const IDENT_POS: usize = #module_path_len + 2;
                const GENERIC_POS: usize = IDENT_POS + #type_ident.len();
                &<Self as #bevy_reflect_path::TypePath>::type_path()[IDENT_POS..GENERIC_POS]
            }

            #[inline]
            fn short_type_name() -> &'static str {
                const IDENT_POS: usize = #module_path_len + 2;
                &<Self as #bevy_reflect_path::TypePath>::type_path()[IDENT_POS..]
            }

            #[inline]
            fn module_path() -> &'static str {
                &<Self as #bevy_reflect_path::TypePath>::type_path()[..#module_path_len]
            }

            #[inline]
            fn crate_name() -> &'static str {
                &<Self as #bevy_reflect_path::TypePath>::type_path()[..#crate_name_len]
            }
        }
    }
}
