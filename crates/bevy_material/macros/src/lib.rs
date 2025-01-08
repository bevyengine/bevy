use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub(crate) fn bevy_material_path() -> syn::Path {
    BevyManifest::shared().get_path("bevy_material")
}

#[proc_macro_derive(Pipelines)]
pub fn derive_pipelines(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_material_path: syn::Path = bevy_material_path();
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_material_path::material_pipeline::Pipelines for #struct_name #type_generics #where_clause {

        }
    })
}
