#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Derive implementations for `bevy_gizmos`.

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Path};

/// Implements the [`GizmoConfigGroup`] trait for a gizmo config group type.
#[proc_macro_derive(GizmoConfigGroup)]
pub fn derive_gizmo_config_group(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_gizmos_path: Path = BevyManifest::default().get_path("bevy_gizmos");
    let bevy_reflect_path: Path = BevyManifest::default().get_path("bevy_reflect");

    ast.generics.make_where_clause().predicates.push(
        parse_quote! { Self: #bevy_reflect_path::Reflect + #bevy_reflect_path::TypePath + Default},
    );

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_gizmos_path::config::GizmoConfigGroup for #struct_name #type_generics #where_clause {
        }
    })
}
