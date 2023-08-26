use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Path};

pub fn derive_extract_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_render_path: Path = crate::bevy_render_path();
    let bevy_ecs_path: Path = bevy_macro_utils::BevyManifest::default()
        .maybe_get_path("bevy_ecs")
        .expect("bevy_ecs should be found in manifest");

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Clone });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let filter = if let Some(attr) = ast
        .attrs
        .iter()
        .find(|a| a.path().is_ident("extract_component_filter"))
    {
        let filter = match attr.parse_args::<syn::Type>() {
            Ok(filter) => filter,
            Err(e) => return e.to_compile_error().into(),
        };

        quote! {
            #filter
        }
    } else {
        quote! {
            ()
        }
    };

    TokenStream::from(quote! {
        impl #impl_generics #bevy_render_path::extract_component::ExtractComponent for #struct_name #type_generics #where_clause {
            type Query = &'static Self;

            type Filter = #filter;
            type Out = Self;

            fn extract_component(item: #bevy_ecs_path::query::QueryItem<'_, Self::Query>) -> Option<Self::Out> {
                Some(item.clone())
            }
        }
    })
}
