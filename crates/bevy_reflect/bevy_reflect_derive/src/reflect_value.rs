use crate::container_attributes::ReflectAttrs;
use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::token::{Paren, Where};
use syn::{parenthesized, Generics};

pub struct ReflectValueDef {
    pub type_name: Ident,
    pub generics: Generics,
    pub attrs: Option<ReflectAttrs>,
}

impl Parse for ReflectValueDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let type_ident = input.parse::<Ident>()?;
        let generics = input.parse::<Generics>()?;
        let mut lookahead = input.lookahead1();
        let mut where_clause = None;
        if lookahead.peek(Where) {
            where_clause = Some(input.parse()?);
            lookahead = input.lookahead1();
        }

        let mut attrs = None;
        if lookahead.peek(Paren) {
            let content;
            parenthesized!(content in input);
            attrs = Some(content.parse::<ReflectAttrs>()?);
        }

        Ok(ReflectValueDef {
            type_name: type_ident,
            generics: Generics {
                where_clause,
                ..generics
            },
            attrs,
        })
    }
}
