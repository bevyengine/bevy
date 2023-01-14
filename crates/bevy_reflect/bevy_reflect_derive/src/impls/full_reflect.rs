use quote::quote;

use crate::derive_data::ReflectMeta;

pub(crate) fn impl_full_reflect(meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let type_name = meta.type_name();
    let bevy_reflect_path = meta.bevy_reflect_path();

    let (impl_generics, ty_generics, where_clause) = meta.generics().split_for_impl();
    quote! {
        impl #impl_generics #bevy_reflect_path::Reflect for #type_name #ty_generics #where_clause {}
    }
}
