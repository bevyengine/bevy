use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub(super) fn derive_map_entities(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();

    match input.data {
        Data::Struct(data_struct) => map_struct(name, &data_struct.fields),
        _ => unimplemented!(),
    }
    .into()
}

fn map_struct(name: syn::Ident, fields: &Fields) -> proc_macro2::TokenStream {
    let mut map_entities = vec![];

    'fields: for (i, field) in fields.iter().enumerate() {
        // check for skipping fields
        for attr in &field.attrs {
            if attr.path().is_ident("skip_mapping") {
                match &attr.meta {
                    // skip that fields
                    syn::Meta::Path(_) => continue 'fields,
                    // the attribute has been used incorrectly
                    _ => panic!("just use the bare `#[skip_mapping]`"),
                }
            }
        }

        let ty = &field.ty;
        let member = match &field.ident {
            // a named field (struct)
            Some(field_name) => syn::Member::Named(field_name.clone()),
            // Unnamed field (tuple-like struct)
            None => syn::Member::Unnamed(syn::Index::from(i)),
        };

        let map_field = quote! {
            <#ty as bevy_ecs::entity::MapEntities>::map_entities(&mut self.#member, entity_mapper);
        };

        map_entities.push(map_field);
    }

    quote! {
     impl MapEntities for #name {
            fn map_entities<M: bevy_ecs::entity::EntityMapper>(&mut self, entity_mapper: &mut M) {
                #(#map_entities)*
            }
        }
    }
}
