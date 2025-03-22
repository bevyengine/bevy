use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Field, Member};

pub fn derive_map_entities(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident.clone();

    match map_entities(name, &input.data) {
        Ok(t) => t,
        Err(e) => e.into_compile_error().into(),
    }
}

fn map_entities(name: syn::Ident, data: &Data) -> syn::Result<TokenStream> {
    let attr_has_skip = |attr: &Attribute| attr.path().is_ident("skip_mapping");
    let field_has_skip = |field: &Field| field.attrs.iter().any(attr_has_skip);

    match data {
        Data::Struct(data) => {
            let map_entities = data.fields
                .iter()
                .enumerate()
                .filter(|(_, field)| field_has_skip(field))
                .map(|(i, field)| {
                    let ty = &field.ty;
                    let member = field.ident.clone().map_or(Member::from(i), Member::Named);
                    quote! {
                        <#ty as bevy_ecs::entity::MapEntities>::map_entities(&mut self.#member, entity_mapper);
                    }
                });
            Ok(quote! {
                impl MapEntities for #name {
                       fn map_entities<M: bevy_ecs::entity::EntityMapper>(&mut self, entity_mapper: &mut M) {
                           #(#map_entities)*
                       }
                   }
               }.into())
        }
        Data::Enum(data) => {
            let variants = data.variants.iter().map(|variant| {
                let variant_name = &variant.ident;

                let (fields, members): (Vec<_>, Vec<_>) = variant.fields
                    .iter()
                    .enumerate()
                    .filter(|(_, field)| field_has_skip(field))
                    .map(|(i, field)| {
                        let member = field.ident.clone().map_or(Member::from(i), Member::Named);
                        (field, member)
                    })
                    .unzip();

                let idents: Vec<_> = members.iter()
                    .map(|member| format_ident!("__self_{}", member))
                    .collect();

                let map_entities = fields.iter().zip(idents.iter()).map(|(field, ident)| {
                        let ty = &field.ty;
                        quote! {
                            <#ty as bevy_ecs::entity::MapEntities>::map_entities(&mut self.#ident, entity_mapper);
                        }
                    });

                quote! {
                    Self::#variant_name { #(ref mut #members: #idents,)* } => {
                        #(#map_entities)*
                    }
                }
            });
            Ok(quote! {
                impl MapEntities for #name {
                    fn map_entities<M: bevy_ecs::entity::EntityMapper>(&mut self, entity_mapper: &mut M) {
                        match self {
                            #(#variants)*
                        }
                    }
                }
            }.into())
        }
        Data::Union(data) => {
            return Err(syn::Error::new(
                data.fields.span(),
                "MapEntities can not be derived for Unions",
            ))
        }
    }
}
