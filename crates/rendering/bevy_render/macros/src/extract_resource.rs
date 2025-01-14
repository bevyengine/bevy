use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Path};

pub fn derive_extract_resource(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_render_path: Path = crate::bevy_render_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Clone });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_render_path::extract_resource::ExtractResource for #struct_name #type_generics #where_clause {
            type Source = Self;

            fn extract_resource(source: &Self::Source) -> Self {
                source.clone()
            }
        }
    })
}
