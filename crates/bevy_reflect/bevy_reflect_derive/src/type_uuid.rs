extern crate proc_macro;

use bevy_macro_utils::BevyManifest;
use quote::quote;
use syn::*;
use uuid::Uuid;

pub(crate) fn type_uuid_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let mut ast: DeriveInput = syn::parse(input).unwrap();
    let bevy_reflect_path: Path = BevyManifest::default().get_path("bevy_reflect");

    // Build the trait implementation
    let name = &ast.ident;

    ast.generics.type_params_mut().for_each(|param| {
        param
            .bounds
            .push(syn::parse_quote!(#bevy_reflect_path::TypeUuid));
    });

    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let mut uuid = None;
    for attribute in ast.attrs.iter().filter_map(|attr| attr.parse_meta().ok()) {
        let Meta::NameValue(name_value) = attribute else {
            continue;
        };

        if name_value
            .path
            .get_ident()
            .map(|i| i != "uuid")
            .unwrap_or(true)
        {
            continue;
        }

        let uuid_str = match name_value.lit {
            Lit::Str(lit_str) => lit_str,
            _ => panic!("`uuid` attribute must take the form `#[uuid = \"xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx\"`."),
        };

        uuid = Some(
            Uuid::parse_str(&uuid_str.value())
                .expect("Value specified to `#[uuid]` attribute is not a valid UUID."),
        );
    }

    let uuid =
        uuid.expect("No `#[uuid = \"xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx\"` attribute found.");
    let bytes = uuid
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:#X}"))
        .map(|byte_str| syn::parse_str::<LitInt>(&byte_str).unwrap());

    let base = quote! { #bevy_reflect_path::Uuid::from_bytes([#( #bytes ),*]) };
    let type_uuid = ast.generics.type_params().fold(base, |acc, param| {
        let ident = &param.ident;
        quote! {
            #bevy_reflect_path::__macro_exports::generate_composite_uuid(#acc, <#ident as #bevy_reflect_path::TypeUuid>::TYPE_UUID)
        }
    });

    let gen = quote! {
        impl #impl_generics #bevy_reflect_path::TypeUuid for #name #type_generics #where_clause {
            const TYPE_UUID: #bevy_reflect_path::Uuid = #type_uuid;
        }
    };
    gen.into()
}
