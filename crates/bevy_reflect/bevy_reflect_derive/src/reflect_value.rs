use crate::container_attributes::ReflectTraits;
use crate::ReflectMeta;
use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::token::{Paren, Where};
use syn::{parenthesized, Generics};

/// A struct used to define a simple reflected value type (such as primitives).
///
/// This takes the form:
///
/// ```ignore
/// // Standard
/// foo(TraitA, TraitB)
///
/// // With generics
/// foo<T1: Bar, T2>(TraitA, TraitB)
///
/// // With generics and where clause
/// foo<T1, T2> where T1: Bar (TraitA, TraitB)
/// ```
pub(crate) struct ReflectValueDef {
    pub type_name: Ident,
    pub generics: Generics,
    pub traits: Option<ReflectTraits>,
}

impl ReflectValueDef {
    pub fn as_meta(&self) -> ReflectMeta {
        ReflectMeta::new(
            &self.type_name,
            &self.generics,
            self.traits.clone().unwrap_or_default(),
        )
    }
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

        let mut traits = None;
        if lookahead.peek(Paren) {
            let content;
            parenthesized!(content in input);
            traits = Some(content.parse::<ReflectTraits>()?);
        }

        Ok(ReflectValueDef {
            type_name: type_ident,
            generics: Generics {
                where_clause,
                ..generics
            },
            traits,
        })
    }
}
