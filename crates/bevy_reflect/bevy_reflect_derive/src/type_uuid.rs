extern crate proc_macro;

use bevy_macro_utils::BevyManifest;
use quote::{quote, ToTokens};
use syn::*;
use uuid::Uuid;

pub fn type_uuid_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();
    let bevy_reflect_path: Path = BevyManifest::default().get_path("bevy_reflect");

    // Build the trait implementation
    let name = &ast.ident;

    let (impl_generics, type_generics, _) = &ast.generics.split_for_impl();
    assert!(
        impl_generics.to_token_stream().is_empty() && type_generics.to_token_stream().is_empty(),
        "#[derive(TypeUuid)] is not supported for generics.",
    );

    let mut uuid = None;
    for attribute in ast.attrs.iter().filter_map(|attr| attr.parse_meta().ok()) {
        let name_value = if let Meta::NameValue(name_value) = attribute {
            name_value
        } else {
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
        .map(|byte| format!("{:#X}", byte))
        .map(|byte_str| syn::parse_str::<LitInt>(&byte_str).unwrap());

    let gen = quote! {
        impl #bevy_reflect_path::TypeUuid for #name {
            const TYPE_UUID: #bevy_reflect_path::Uuid = #bevy_reflect_path::Uuid::from_bytes([
                #( #bytes ),*
            ]);
        }
    };
    gen.into()
}
