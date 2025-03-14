use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields};

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
        // skip that field
        for attr in &field.attrs {
            if attr.path().is_ident("skip_mapping") {
                match &attr.meta {
                    syn::Meta::Path(_) => continue 'fields,
                    _ => panic!("just use the bare `#[skip_mapping]`"),
                }
            }
        }

        let ty = &field.ty;
        let map_field = if let Some(field_name) = &field.ident {
            // Named field (struct)
            quote! {
                <#ty as bevy_ecs::entity::MapEntities>::map_entities(&mut self.#field_name, entity_mapper);
            }
        } else {
            // Unnamed field (tuple-like struct)
            let idx = syn::Index::from(i);
            quote! {
                <#ty as bevy_ecs::entity::MapEntities>::map_entities(&mut self.#idx, entity_mapper);
            }
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
