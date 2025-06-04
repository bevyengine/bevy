use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub(super) fn derive_map_entities(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();

    match input.data {
        Data::Struct(data_struct) => map_struct(name, &data_struct.fields),
        Data::Enum(data_enum) => map_enum(name, &data_enum.variants),
        Data::Union(_) => panic!("MapEntities is only valid on structs or enums"),
    }
    .into()
}

fn map_struct(name: syn::Ident, fields: &Fields) -> proc_macro2::TokenStream {
    let mut map_entities = vec![];

    for (i, field) in fields.iter().enumerate() {
        if skip_mapping(&field.attrs) {
            continue;
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

// fn map_enum(name:syn::Ident, variantes: &[])

fn map_enum(
    name: syn::Ident,
    variants: &syn::punctuated::Punctuated<syn::Variant, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    let variants = variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let pattern = get_enum_pattern(&variant.fields);
        let map_entities = map_enum_variant(&variant.fields);

        quote! {
            #name::#variant_name #pattern => {
                #map_entities
            }
        }
    });

    quote! {
        impl MapEntities for #name {
            fn map_entities<M: bevy_ecs::entity::EntityMapper>(&mut self, entity_mapper: &mut M) {
                match self {
                    #(#variants)*
                    // allows empty enums to derive aswell
                    _ => unreachable!()
                }
            }
        }
    }
}

/// Generates the match pattern for unnamed fields (e.g., `(field_0, field_1)`)
///
/// Or named fields (e.g., `(item, quantity)`)
fn get_enum_pattern(fields: &Fields) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(named_fields) => {
            let field_vars = named_fields.named.iter().map(|field| {
                let var_name = field.ident.as_ref().unwrap();
                quote! { #var_name }
            });
            quote! { {#(#field_vars),*} }
        }
        Fields::Unnamed(unnamed_fields) => {
            let field_vars = (0..unnamed_fields.unnamed.len()).map(|i| {
                let var_name =
                    syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site());
                quote! { #var_name }
            });
            quote! { ( #(#field_vars),* ) }
        }
        Fields::Unit => quote! {},
    }
}

// Function to generate code to map fields for an enum variant
fn map_enum_variant(fields: &Fields) -> proc_macro2::TokenStream {
    let mut map_entities = vec![];

    for (i, field) in fields.iter().enumerate() {
        if skip_mapping(&field.attrs) {
            continue;
        }

        let ty = &field.ty;
        let member = match &field.ident {
            // named field
            Some(field_name) => field_name.clone(),
            // Unnamed field
            None => syn::Ident::new(&format!("field_{}", i), proc_macro2::Span::call_site()),
        };

        let map_field = quote! {
            <#ty as bevy_ecs::entity::MapEntities>::map_entities(#member, entity_mapper);
        };

        map_entities.push(map_field);
    }

    quote! {
        #(#map_entities)*
    }
}

/// check if any attribute contains `skip_mapping`
fn skip_mapping(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("skip_mapping") {
            continue;
        }
        match &attr.meta {
            // skip that fields
            syn::Meta::Path(_) => return true,
            // the attribute has been used incorrectly
            _ => panic!("just use the bare `#[skip_mapping]`"),
        }
    }

    false
}
