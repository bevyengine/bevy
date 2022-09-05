use quote::quote;

use crate::derive_data::ReflectMeta;

pub(crate) fn impl_type_path(reflect_meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let generics = reflect_meta.generics();
    let type_name = reflect_meta.type_name();
    let bevy_reflect_path = reflect_meta.bevy_reflect_path();

    let is_generic = !generics.params.is_empty();

    let base_name = reflect_meta
        .reflected_type_path()
        .map(|x| quote!(#x))
        .unwrap_or_else(|| {
            let type_name = type_name.to_string();
            quote!(concat!(module_path!(), "::", #type_name))
        });

    let get_type_name = if is_generic {
        let values = {
            let getters = generics.params.iter().map(|p| match p {
                syn::GenericParam::Type(p) => {
                    let ty = &p.ident;
                    quote!(<#ty as #bevy_reflect_path::TypePath>::type_path())
                }
                syn::GenericParam::Lifetime(p) => {
                    let name = &p.lifetime.ident.to_string();
                    quote!(concat!("'", #name))
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
                format!(concat!("{}<", #brackets, ">"), BASE_NAME, #values)
            })
        }
    } else {
        quote! {
            BASE_NAME
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics #bevy_reflect_path::TypePath for #type_name #ty_generics #where_clause {
            fn type_path() -> &'static str {
                const BASE_NAME: &'static str = #base_name;
                #get_type_name
            }
        }
    }
}
