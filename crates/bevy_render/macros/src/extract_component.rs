use bevy_macro_utils::fq_std::{FQClone, FQOption};
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Path};

pub fn derive_extract_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_render_path: Path = crate::bevy_render_path();
    let bevy_ecs_path: Path = bevy_macro_utils::BevyManifest::shared(|manifest| {
        manifest
            .maybe_get_path("bevy_ecs")
            .expect("bevy_ecs should be found in manifest")
    });

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: #FQClone });

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

    let sync_target = if let Some(attr) = ast
        .attrs
        .iter()
        .find(|a| a.path().is_ident("extract_component_sync_target"))
    {
        let sync_target = match attr.parse_args::<syn::Type>() {
            Ok(sync_target) => sync_target,
            Err(e) => return e.to_compile_error().into(),
        };

        quote! {
            #sync_target
        }
    } else {
        quote! {
            Self
        }
    };

    TokenStream::from(quote! {
        impl #impl_generics #bevy_render_path::sync_component::SyncComponent for #struct_name #type_generics #where_clause {
            type Target = #sync_target;
        }

        impl #impl_generics #bevy_render_path::extract_component::ExtractComponent for #struct_name #type_generics #where_clause {
            type QueryData = &'static Self;
            type QueryFilter = #filter;
            type Out = Self;

            fn extract_component(item: #bevy_ecs_path::query::QueryItem<'_, '_, Self::QueryData>) -> #FQOption<Self::Out> {
                #FQOption::Some(item.clone())
            }
        }
    })
}
