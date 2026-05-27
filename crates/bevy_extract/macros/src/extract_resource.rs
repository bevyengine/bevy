use bevy_macro_utils::fq_std::FQClone;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Path};

pub fn derive_extract_resource(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_render_path: Path = crate::bevy_render_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: #FQClone });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let app_label = match ast.attrs.iter().find(|a| a.path().is_ident("extract_app")) {
        Some(attr) => match attr.parse_args::<syn::Type>() {
            Ok(label) => label,
            Err(e) => return e.to_compile_error().into(),
        },
        None => {
            return syn::Error::new_spanned(
                &ast.ident,
                "ExtractResource requires #[extract_app(MyAppLabel)] to specify the target sub-app",
            )
            .to_compile_error()
            .into();
        }
    };

    TokenStream::from(quote! {
        impl #impl_generics #bevy_render_path::extract_resource::ExtractResource<#app_label> for #struct_name #type_generics #where_clause {
            type Source = Self;

            fn extract_resource(source: &Self::Source) -> Self {
                source.clone()
            }
        }
    })
}
