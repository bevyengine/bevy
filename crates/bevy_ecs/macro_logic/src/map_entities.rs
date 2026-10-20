use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::Parse, spanned::Spanned, Data, DataEnum, DataStruct, Expr, ExprPath, Ident, Member,
    Path, Token,
};

use crate::component::relationship_field;

const ENTITIES: &str = "entities";

/// Implements `MapEntities`
pub fn map_entities(
    data: &Data,
    bevy_ecs: &Path,
    self_ident: Ident,
    is_relationship: bool,
    is_relationship_target: bool,
    map_entities_attr: Option<MapEntitiesAttributeKind>,
) -> Option<TokenStream> {
    if let Some(map_entities_override) = map_entities_attr {
        let map_entities_tokens = map_entities_override.to_token_stream(bevy_ecs);
        return Some(quote!(
            #map_entities_tokens(#self_ident, mapper)
        ));
    }

    match data {
        Data::Struct(DataStruct { fields, .. }) => {
            let mut map = Vec::with_capacity(fields.len());

            let relationship = if is_relationship || is_relationship_target {
                relationship_field(fields, "MapEntities", fields.span()).ok()
            } else {
                None
            };
            fields
                .iter()
                .enumerate()
                .filter(|(_, field)| {
                    field.attrs.iter().any(|a| a.path().is_ident(ENTITIES))
                        || relationship.is_some_and(|relationship| relationship == *field)
                })
                .for_each(|(index, field)| {
                    let field_member = field
                        .ident
                        .clone()
                        .map_or(Member::from(index), Member::Named);

                    map.push(quote!(#self_ident.#field_member.map_entities(mapper);));
                });
            if map.is_empty() {
                return None;
            };
            Some(quote!(
                #(#map)*
            ))
        }
        Data::Enum(DataEnum { variants, .. }) => {
            let mut map = Vec::with_capacity(variants.len());

            for variant in variants.iter() {
                let field_members = variant
                    .fields
                    .iter()
                    .enumerate()
                    .filter(|(_, field)| field.attrs.iter().any(|a| a.path().is_ident(ENTITIES)))
                    .map(|(index, field)| {
                        field
                            .ident
                            .clone()
                            .map_or(Member::from(index), Member::Named)
                    })
                    .collect::<Vec<_>>();

                let ident = &variant.ident;
                let field_idents = field_members
                    .iter()
                    .map(|member| format_ident!("__self{}", member))
                    .collect::<Vec<_>>();

                map.push(
                    quote!(Self::#ident {#(#field_members: #field_idents,)* ..} => {
                        #(#field_idents.map_entities(mapper);)*
                    }),
                );
            }

            if map.is_empty() {
                return None;
            };

            Some(quote!(
                match #self_ident {
                    #(#map,)*
                    _ => {}
                }
            ))
        }
        Data::Union(_) => None,
    }
}

/// The type of `MapEntities` attribute.
#[derive(Debug)]
pub enum MapEntitiesAttributeKind {
    /// expressions like function or struct names
    ///
    /// structs will throw compile errors on the code generation so this is safe
    Path(ExprPath),
    /// When no value is specified
    Default,
}

impl MapEntitiesAttributeKind {
    fn from_expr(value: Expr) -> syn::Result<Self> {
        match value {
            Expr::Path(path) => Ok(Self::Path(path)),
            // throw meaningful error on all other expressions
            _ => Err(syn::Error::new(
                value.span(),
                [
                    "Not supported in this position, please use one of the following:",
                    "- path to function",
                    "- nothing to default to MapEntities implementation",
                ]
                .join("\n"),
            )),
        }
    }

    fn to_token_stream(&self, bevy_ecs_path: &Path) -> TokenStream {
        match self {
            MapEntitiesAttributeKind::Path(path) => path.to_token_stream(),
            MapEntitiesAttributeKind::Default => {
                quote!(
                   <Self as #bevy_ecs_path::entity::MapEntities>::map_entities
                )
            }
        }
    }
}

impl Parse for MapEntitiesAttributeKind {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            input.parse::<Expr>().and_then(Self::from_expr)
        } else {
            Ok(Self::Default)
        }
    }
}
